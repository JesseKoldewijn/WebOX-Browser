use std::sync::Arc;

use webox_config::AppConfig;
use webox_engine::{
    BrowserInstanceDescriptor, BrowserInstanceEvent, BrowserInstanceState, HostSurfaceInputEvent,
    RuntimeReadiness, StartupDiagnostics, WeboxEngine,
};
use webox_memory::{
    LinuxProcessMemoryCollector, MemoryAttribution, MemoryController, MemoryEvent, PolicyDecision,
    SupportedSystemReport, TabTelemetry,
};

pub trait MemoryEventObserver: Send + Sync {
    fn on_memory_event(&self, event: &MemoryEvent);
}

#[derive(Clone, Debug)]
pub struct EmbeddedRuntimeConfig {
    pub app_config: AppConfig,
    pub available_memory_bytes: u64,
}

pub struct EmbeddedRuntime {
    engine: WeboxEngine,
    memory_controller: MemoryController,
    memory_collector: LinuxProcessMemoryCollector,
    initialized: bool,
    observer: Option<Arc<dyn MemoryEventObserver>>,
    startup_diagnostics: Vec<StartupDiagnostics>,
    latest_events: Vec<BrowserInstanceEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeBrowserSnapshot {
    pub browser: BrowserInstanceState,
    pub policy_decision: PolicyDecision,
}

impl EmbeddedRuntime {
    #[must_use]
    pub fn new(config: EmbeddedRuntimeConfig) -> Self {
        let mut engine = WeboxEngine::new(&config.app_config);
        let startup = engine.start();
        Self {
            engine,
            memory_controller: MemoryController::new(
                config.app_config.startup.max_memory_per_tab_bytes,
            ),
            memory_collector: LinuxProcessMemoryCollector::new(),
            initialized: true,
            observer: None,
            startup_diagnostics: vec![startup],
            latest_events: Vec::new(),
        }
    }

    pub fn register_memory_observer(&mut self, observer: Arc<dyn MemoryEventObserver>) {
        self.observer = Some(observer);
    }

    pub fn create_browser_instance(
        &mut self,
        initial_url: &str,
    ) -> Result<BrowserInstanceDescriptor, String> {
        let descriptor = self
            .engine
            .create_browser_instance(initial_url)
            .map_err(|error| error.message)?;
        self.capture_engine_events();
        Ok(descriptor)
    }

    pub fn navigate_browser_instance(&mut self, browser_id: &str, url: &str) -> Result<(), String> {
        self.engine
            .navigate_browser_instance(browser_id, url)
            .map_err(|error| error.message)?;
        self.capture_engine_events();
        Ok(())
    }

    pub fn reload_browser_instance(&mut self, browser_id: &str) -> Result<(), String> {
        self.engine
            .reload_browser_instance(browser_id)
            .map_err(|error| error.message)?;
        self.capture_engine_events();
        Ok(())
    }

    pub fn finish_navigation(&mut self, browser_id: &str, title: &str) -> Result<(), String> {
        self.engine
            .finish_navigation(browser_id, title)
            .map_err(|error| error.message)?;
        self.capture_engine_events();
        Ok(())
    }

    pub fn fail_navigation(&mut self, browser_id: &str, message: &str) -> Result<(), String> {
        self.engine
            .fail_navigation(browser_id, message)
            .map_err(|error| error.message)?;
        self.capture_engine_events();
        Ok(())
    }

    pub fn close_browser_instance(&mut self, browser_id: &str) -> Result<(), String> {
        self.engine
            .close_browser_instance(browser_id)
            .map_err(|error| error.message)?;
        self.capture_engine_events();
        Ok(())
    }

    pub fn resize_browser_surface(
        &mut self,
        browser_id: &str,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        self.engine
            .resize_browser_surface(browser_id, width, height)
            .map_err(|error| error.message)?;
        self.capture_engine_events();
        Ok(())
    }

    pub fn dispatch_text_input(&mut self, browser_id: &str, text: &str) -> Result<(), String> {
        self.dispatch_surface_input(
            browser_id,
            HostSurfaceInputEvent::Text {
                text: text.to_string(),
            },
        )
    }

    pub fn dispatch_surface_input(
        &mut self,
        browser_id: &str,
        event: HostSurfaceInputEvent,
    ) -> Result<(), String> {
        self.engine
            .dispatch_surface_input(browser_id, event)
            .map_err(|error| error.message)?;
        self.capture_engine_events();
        Ok(())
    }

    pub fn apply_memory_sample(
        &mut self,
        telemetry: &TabTelemetry,
    ) -> Result<RuntimeBrowserSnapshot, String> {
        self.apply_memory_sample_with_attribution(telemetry, MemoryAttribution::synthetic())
    }

    pub fn apply_observed_memory_sample(
        &mut self,
        browser_id: &str,
    ) -> Result<RuntimeBrowserSnapshot, String> {
        let observed = self.memory_collector.collect_for_tab(browser_id);
        self.apply_memory_sample_with_attribution(&observed.telemetry, observed.attribution)
    }

    pub fn apply_memory_sample_with_attribution(
        &mut self,
        telemetry: &TabTelemetry,
        attribution: MemoryAttribution,
    ) -> Result<RuntimeBrowserSnapshot, String> {
        let decision = self.memory_controller.evaluate(telemetry);
        if let Some(observer) = &self.observer {
            observer.on_memory_event(&decision.event);
        }
        self.engine
            .update_browser_memory(
                &telemetry.tab_id,
                decision.event.total_bytes,
                match decision.event.level {
                    webox_memory::MemoryPressureLevel::Normal => None,
                    webox_memory::MemoryPressureLevel::Warning => {
                        Some("memory warning".to_string())
                    }
                    webox_memory::MemoryPressureLevel::Critical => {
                        Some("critical memory pressure".to_string())
                    }
                    webox_memory::MemoryPressureLevel::Exhausted => {
                        Some("memory exhaustion risk".to_string())
                    }
                },
                if matches!(
                    decision.event.level,
                    webox_memory::MemoryPressureLevel::Exhausted
                ) {
                    Some("Tab ended due to suspected memory exhaustion".to_string())
                } else {
                    None
                },
                Some(attribution.label()),
            )
            .map_err(|error| error.message)?;
        self.capture_engine_events();

        let browser = self
            .engine
            .browser_instance(&telemetry.tab_id)
            .cloned()
            .ok_or_else(|| format!("Browser instance '{}' was not found", telemetry.tab_id))?;

        Ok(RuntimeBrowserSnapshot {
            browser,
            policy_decision: decision,
        })
    }

    #[must_use]
    pub fn browser_instance(&self, browser_id: &str) -> Option<&BrowserInstanceState> {
        self.engine.browser_instance(browser_id)
    }

    #[must_use]
    pub fn browser_instances(&self) -> Vec<BrowserInstanceState> {
        self.engine.browser_instances()
    }

    pub fn drain_events(&mut self) -> Vec<BrowserInstanceEvent> {
        self.latest_events.drain(..).collect()
    }

    #[must_use]
    pub fn system_report(&self, available_memory_bytes: u64) -> SupportedSystemReport {
        self.memory_controller.system_report(available_memory_bytes)
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[StartupDiagnostics] {
        &self.startup_diagnostics
    }

    #[must_use]
    pub fn runtime_readiness(&self) -> &RuntimeReadiness {
        self.engine.runtime_readiness()
    }

    #[must_use]
    pub fn live_mvp_ready(&self) -> bool {
        self.engine.live_mvp_ready()
    }

    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn capture_engine_events(&mut self) {
        self.latest_events.extend(self.engine.drain_events());
    }
}

#[cfg(test)]
mod tests {
    use super::{EmbeddedRuntime, EmbeddedRuntimeConfig};
    use webox_config::AppConfig;
    use webox_memory::TabTelemetry;

    #[test]
    fn embedded_runtime_initializes() {
        let runtime = EmbeddedRuntime::new(EmbeddedRuntimeConfig {
            app_config: AppConfig::development(),
            available_memory_bytes: 16 * 1024 * 1024 * 1024,
        });
        assert!(runtime.is_initialized());
        assert_eq!(runtime.diagnostics().len(), 1);
    }

    #[test]
    fn embedded_runtime_tracks_browser_memory_and_navigation() {
        let mut runtime = EmbeddedRuntime::new(EmbeddedRuntimeConfig {
            app_config: AppConfig::simulated(),
            available_memory_bytes: 16 * 1024 * 1024 * 1024,
        });
        let instance = runtime
            .create_browser_instance("https://example.com")
            .unwrap();

        runtime
            .navigate_browser_instance(&instance.id, "https://example.com/heavy")
            .unwrap();
        runtime
            .finish_navigation(&instance.id, "Heavy App")
            .unwrap();
        runtime
            .resize_browser_surface(&instance.id, 1600, 900)
            .unwrap();
        let snapshot = runtime
            .apply_memory_sample(&TabTelemetry {
                tab_id: instance.id.clone(),
                renderer_bytes: 8,
                browser_bytes: 2,
                gpu_bytes: 1,
            })
            .unwrap();

        assert_eq!(snapshot.browser.title, "Heavy App");
        assert_eq!(snapshot.browser.url, "https://example.com/heavy");
        assert_eq!(snapshot.policy_decision.event.tab_id, instance.id);
        assert_eq!(snapshot.browser.surface.width, 1600);
        assert!(!runtime.drain_events().is_empty());
    }

    #[test]
    fn embedded_runtime_exposes_observed_memory_attribution() {
        let mut runtime = EmbeddedRuntime::new(EmbeddedRuntimeConfig {
            app_config: AppConfig::simulated(),
            available_memory_bytes: 16 * 1024 * 1024 * 1024,
        });
        let instance = runtime
            .create_browser_instance("https://example.com")
            .unwrap();

        let snapshot = runtime.apply_observed_memory_sample(&instance.id).unwrap();

        assert!(
            snapshot
                .browser
                .memory_attribution
                .as_deref()
                .is_some_and(|attribution| attribution.contains("live_mvp_evidence=true"))
        );
    }
}
