use std::collections::HashMap;

use webox_config::AppConfig;
use webox_engine::{
    BrowserInstanceEvent, BrowserInstanceEventKind, BrowserInstanceState, HostMouseButton,
    HostSurfaceInputEvent, RuntimeReadiness, StartupDiagnostics, WeboxEngine,
};
use webox_memory::{
    LinuxProcessMemoryCollector, MemoryAttribution, MemoryController, MemoryPressureLevel,
    PolicyDecision, RecoveryReport, SupportedSystemReport, TabTelemetry,
};
use webox_ui::{
    BrowserCommand, BrowserWindowModel, SurfaceFrameBuffer, SurfaceInputEvent, SurfaceMouseButton,
    SurfaceViewState, TabViewState, WindowId,
};

pub struct HostShell {
    config: AppConfig,
    engine: WeboxEngine,
    memory_controller: MemoryController,
    memory_collector: LinuxProcessMemoryCollector,
    windows: HashMap<WindowId, BrowserWindowModel>,
    startup_diagnostics: Vec<StartupDiagnostics>,
    recovery_reports: Vec<RecoveryReport>,
    tab_runtime_modes: HashMap<String, String>,
}

impl HostShell {
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        let target = config.startup.max_memory_per_tab_bytes;
        Self {
            engine: WeboxEngine::new(&config),
            config,
            memory_controller: MemoryController::new(target),
            memory_collector: LinuxProcessMemoryCollector::new(),
            windows: HashMap::new(),
            startup_diagnostics: Vec::new(),
            recovery_reports: Vec::new(),
            tab_runtime_modes: HashMap::new(),
        }
    }

    pub fn start(&mut self) {
        self.startup_diagnostics.push(self.engine.start());
        self.sync_engine_events();
    }

    /// Drive the CEF message loop for one iteration.
    /// Call this on every UI frame to keep CEF processing network, rendering,
    /// and IPC messages.
    pub fn tick(&mut self) {
        self.engine.tick();
        self.sync_engine_events();
    }

    pub fn shutdown(&mut self) {
        self.startup_diagnostics.push(self.engine.shutdown());
        self.sync_engine_events();
    }

    pub fn create_window(&mut self, id: impl Into<String>) -> WindowId {
        let id = id.into();
        self.windows
            .insert(id.clone(), BrowserWindowModel::new(id.clone()));
        id
    }

    pub fn open_tab(&mut self, window_id: &str, initial_url: &str) -> Result<String, String> {
        let descriptor = self
            .engine
            .create_browser_instance(initial_url)
            .map_err(|error| error.message)?;
        let tab_id = {
            let window = self
                .windows
                .get_mut(window_id)
                .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
            window.add_tab(descriptor.id.clone(), descriptor.initial_url.clone())
        };
        self.tab_runtime_modes
            .insert(tab_id.clone(), format!("{:?}", descriptor.backend));
        self.sync_engine_events();
        let window = self
            .windows
            .get_mut(window_id)
            .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
        window.set_active_tab(&tab_id);
        Ok(tab_id)
    }

    pub fn navigate_tab(
        &mut self,
        _window_id: &str,
        tab_id: &str,
        url: &str,
    ) -> Result<(), String> {
        self.engine
            .navigate_browser_instance(tab_id, url)
            .map_err(|error| error.message)?;
        self.sync_engine_events();
        Ok(())
    }

    pub fn finish_navigation(
        &mut self,
        _window_id: &str,
        tab_id: &str,
        title: &str,
    ) -> Result<(), String> {
        self.engine
            .finish_navigation(tab_id, title)
            .map_err(|error| error.message)?;
        self.sync_engine_events();
        Ok(())
    }

    pub fn fail_tab_navigation(&mut self, tab_id: &str, message: &str) -> Result<(), String> {
        self.engine
            .fail_navigation(tab_id, message)
            .map_err(|error| error.message)?;
        self.sync_engine_events();
        Ok(())
    }

    pub fn go_back(&mut self, _window_id: &str, tab_id: &str) -> Result<(), String> {
        self.engine
            .go_back_browser_instance(tab_id)
            .map_err(|error| error.message)?;
        self.sync_engine_events();
        Ok(())
    }

    pub fn go_forward(&mut self, _window_id: &str, tab_id: &str) -> Result<(), String> {
        self.engine
            .go_forward_browser_instance(tab_id)
            .map_err(|error| error.message)?;
        self.sync_engine_events();
        Ok(())
    }

    pub fn reload_tab(&mut self, _window_id: &str, tab_id: &str) -> Result<(), String> {
        self.engine
            .reload_browser_instance(tab_id)
            .map_err(|error| error.message)?;
        self.sync_engine_events();
        Ok(())
    }

    pub fn resize_tab_surface(
        &mut self,
        tab_id: &str,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        self.engine
            .resize_browser_surface(tab_id, width, height)
            .map_err(|error| error.message)?;
        self.sync_engine_events();
        Ok(())
    }

    pub fn focus_tab_surface(&mut self, tab_id: &str, focused: bool) -> Result<(), String> {
        self.engine
            .set_surface_focus(tab_id, focused)
            .map_err(|error| error.message)?;
        self.sync_engine_events();
        Ok(())
    }

    pub fn dispatch_surface_input(
        &mut self,
        tab_id: &str,
        event: SurfaceInputEvent,
    ) -> Result<(), String> {
        self.engine
            .dispatch_surface_input(tab_id, map_surface_input_event(event))
            .map_err(|error| error.message)?;
        self.sync_engine_events();
        Ok(())
    }

    pub fn close_tab(&mut self, window_id: &str, tab_id: &str) -> Result<(), String> {
        self.engine
            .close_browser_instance(tab_id)
            .map_err(|error| error.message)?;
        let window = self
            .windows
            .get_mut(window_id)
            .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
        window.close_tab(tab_id);
        self.tab_runtime_modes.remove(tab_id);
        self.sync_engine_events();
        Ok(())
    }

    pub fn record_tab_telemetry(
        &mut self,
        window_id: &str,
        telemetry: &TabTelemetry,
    ) -> Result<PolicyDecision, String> {
        self.record_tab_telemetry_with_attribution(
            window_id,
            telemetry,
            MemoryAttribution::synthetic(),
        )
    }

    pub fn collect_observed_tab_telemetry(
        &mut self,
        window_id: &str,
        tab_id: &str,
    ) -> Result<PolicyDecision, String> {
        let observed = self.memory_collector.collect_for_tab(tab_id);
        self.record_tab_telemetry_with_attribution(
            window_id,
            &observed.telemetry,
            observed.attribution,
        )
    }

    pub fn record_tab_telemetry_with_attribution(
        &mut self,
        window_id: &str,
        telemetry: &TabTelemetry,
        attribution: MemoryAttribution,
    ) -> Result<PolicyDecision, String> {
        let decision = self.memory_controller.evaluate(telemetry);
        let label = match decision.event.level {
            MemoryPressureLevel::Normal => None,
            MemoryPressureLevel::Warning => Some("memory warning".to_string()),
            MemoryPressureLevel::Critical => Some("critical memory pressure".to_string()),
            MemoryPressureLevel::Exhausted => Some("memory exhaustion risk".to_string()),
        };
        let attribution_label = Some(attribution.label());
        self.engine
            .update_browser_memory(
                &telemetry.tab_id,
                decision.event.total_bytes,
                label.clone(),
                if matches!(decision.event.level, MemoryPressureLevel::Exhausted) {
                    Some("Tab ended due to suspected memory exhaustion".to_string())
                } else {
                    None
                },
                attribution_label.clone(),
            )
            .map_err(|error| error.message)?;
        let window = self
            .windows
            .get_mut(window_id)
            .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
        window.set_memory_indicator(&telemetry.tab_id, label, attribution_label);
        if matches!(decision.event.level, MemoryPressureLevel::Exhausted) {
            let report = self
                .memory_controller
                .capture_recovery_report(telemetry, attribution);
            window.set_failure_state(
                &telemetry.tab_id,
                Some(format!(
                    "Tab ended due to suspected memory exhaustion ({})",
                    report.attribution.label()
                )),
            );
            self.recovery_reports.push(report);
        }
        self.sync_engine_events();
        Ok(decision)
    }

    pub fn dispatch_command(
        &mut self,
        window_id: &str,
        command: BrowserCommand,
    ) -> Result<(), String> {
        match command {
            BrowserCommand::Navigate { tab_id, url } => self.navigate_tab(window_id, &tab_id, &url),
            BrowserCommand::Reload { tab_id } => self.reload_tab(window_id, &tab_id),
            BrowserCommand::Back { tab_id } => self.go_back(window_id, &tab_id),
            BrowserCommand::Forward { tab_id } => self.go_forward(window_id, &tab_id),
            BrowserCommand::ActivateTab { tab_id } => {
                let window = self
                    .windows
                    .get_mut(window_id)
                    .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
                window.set_active_tab(&tab_id);
                self.focus_tab_surface(&tab_id, true)
            }
            BrowserCommand::CloseTab { tab_id } => self.close_tab(window_id, &tab_id),
        }
    }

    pub fn sync_engine_events(&mut self) {
        let events = self.engine.drain_events();
        for event in events {
            self.apply_engine_event(event);
        }
    }

    #[must_use]
    pub fn supported_system_report(&self, available_bytes: u64) -> SupportedSystemReport {
        self.memory_controller.system_report(available_bytes)
    }

    #[must_use]
    pub fn startup_diagnostics(&self) -> &[StartupDiagnostics] {
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
    pub fn recovery_reports(&self) -> &[RecoveryReport] {
        &self.recovery_reports
    }

    #[must_use]
    pub fn windows(&self) -> &HashMap<WindowId, BrowserWindowModel> {
        &self.windows
    }

    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    #[must_use]
    pub fn runtime_mode_for_tab(&self, tab_id: &str) -> Option<&str> {
        self.tab_runtime_modes.get(tab_id).map(String::as_str)
    }

    #[must_use]
    pub fn engine(&self) -> &WeboxEngine {
        &self.engine
    }

    fn apply_engine_event(&mut self, event: BrowserInstanceEvent) {
        if let Some(snapshot) = event.snapshot {
            self.update_windows_from_snapshot(&snapshot);
        } else if matches!(event.kind, BrowserInstanceEventKind::Closed) {
            self.remove_closed_tab(&event.browser_id);
        }
    }

    fn update_windows_from_snapshot(&mut self, snapshot: &BrowserInstanceState) {
        let next = TabViewState {
            id: snapshot.id.clone(),
            title: snapshot.title.clone(),
            url: snapshot.url.clone(),
            is_loading: snapshot.is_loading,
            memory_indicator: snapshot.memory_indicator.clone(),
            failure_state: snapshot.failure_state.clone(),
            memory_attribution: snapshot.memory_attribution.clone(),
            status_text: snapshot.status_text.clone(),
            can_go_back: snapshot.can_go_back,
            can_go_forward: snapshot.can_go_forward,
            surface: SurfaceViewState {
                surface_id: snapshot.surface.surface_id.clone(),
                width: snapshot.surface.width,
                height: snapshot.surface.height,
                focused: snapshot.surface.focused,
                frame_token: snapshot.surface.frame_token,
                frame_label: snapshot.surface.last_frame_label.clone(),
                render_evidence: snapshot.surface.render_evidence.clone(),
                frame_buffer: snapshot.surface.frame_buffer.as_ref().map(|buffer| {
                    SurfaceFrameBuffer {
                        width: buffer.width,
                        height: buffer.height,
                        bgra: buffer.bgra.clone(),
                    }
                }),
                damage_events: snapshot.surface.damage_events,
                host_surface_failure: snapshot.surface.host_surface_failure.clone(),
            },
        };

        for window in self.windows.values_mut() {
            if window.tabs.iter().any(|tab| tab.id == snapshot.id) {
                window.update_from_engine(next.clone());
                if window.active_tab_id.as_deref() == Some(snapshot.id.as_str()) {
                    window.set_active_tab(&snapshot.id);
                }
            }
        }
    }

    fn remove_closed_tab(&mut self, tab_id: &str) {
        for window in self.windows.values_mut() {
            if window.tabs.iter().any(|tab| tab.id == tab_id) {
                window.close_tab(tab_id);
            }
        }
    }
}

fn map_surface_input_event(event: SurfaceInputEvent) -> HostSurfaceInputEvent {
    match event {
        SurfaceInputEvent::PointerMove { x, y } => HostSurfaceInputEvent::PointerMove { x, y },
        SurfaceInputEvent::PointerButton {
            x,
            y,
            button,
            pressed,
            click_count,
        } => HostSurfaceInputEvent::PointerButton {
            x,
            y,
            button: match button {
                SurfaceMouseButton::Left => HostMouseButton::Left,
                SurfaceMouseButton::Middle => HostMouseButton::Middle,
                SurfaceMouseButton::Right => HostMouseButton::Right,
            },
            pressed,
            click_count,
        },
        SurfaceInputEvent::Wheel {
            x,
            y,
            delta_x,
            delta_y,
        } => HostSurfaceInputEvent::Wheel {
            x,
            y,
            delta_x,
            delta_y,
        },
        SurfaceInputEvent::Key { key_code, pressed } => {
            HostSurfaceInputEvent::Key { key_code, pressed }
        }
        SurfaceInputEvent::Text { text } => HostSurfaceInputEvent::Text { text },
        SurfaceInputEvent::Focus { focused } => HostSurfaceInputEvent::Focus { focused },
        SurfaceInputEvent::Resize { width, height } => {
            HostSurfaceInputEvent::Resize { width, height }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HostShell;
    use webox_config::AppConfig;
    use webox_memory::TabTelemetry;
    use webox_ui::BrowserCommand;

    #[test]
    fn host_shell_starts_and_opens_tabs() {
        let mut shell = HostShell::new(AppConfig::simulated());
        shell.start();
        let window = shell.create_window("window-1");
        let tab = shell.open_tab(&window, "https://webox.dev").unwrap();
        shell.finish_navigation(&window, &tab, "webox").unwrap();
        let decision = shell
            .record_tab_telemetry(
                &window,
                &TabTelemetry {
                    tab_id: tab.clone(),
                    renderer_bytes: 1,
                    browser_bytes: 1,
                    gpu_bytes: 1,
                },
            )
            .unwrap();
        assert_eq!(decision.event.tab_id, tab);
        assert_eq!(shell.startup_diagnostics().len(), 1);
    }

    #[test]
    fn host_shell_dispatches_navigation_commands() {
        let mut shell = HostShell::new(AppConfig::simulated());
        shell.start();
        let window = shell.create_window("window-1");
        let tab = shell.open_tab(&window, "https://webox.dev").unwrap();
        shell
            .dispatch_command(
                &window,
                BrowserCommand::Navigate {
                    tab_id: tab.clone(),
                    url: "https://example.com".to_string(),
                },
            )
            .unwrap();
        let current_url = &shell.windows()[&window].tabs[0].url;
        assert_eq!(current_url, "https://example.com");
        assert!(shell.windows()[&window].tabs[0].is_loading);
    }

    #[test]
    fn host_shell_updates_live_engine_state_for_tabs() {
        let mut shell = HostShell::new(AppConfig::simulated());
        shell.start();
        let window = shell.create_window("window-1");
        let tab = shell.open_tab(&window, "https://webox.dev").unwrap();

        shell
            .finish_navigation(&window, &tab, "webox home")
            .unwrap();
        let engine_tab = shell.engine().browser_instance(&tab).unwrap();
        assert_eq!(engine_tab.title, "webox home");
        assert_eq!(shell.runtime_mode_for_tab(&tab), Some("Simulated"));

        shell.close_tab(&window, &tab).unwrap();
        assert!(shell.engine().browser_instance(&tab).is_none());
    }

    #[test]
    fn host_shell_history_commands_use_engine_navigation() {
        let mut shell = HostShell::new(AppConfig::simulated());
        shell.start();
        let window = shell.create_window("window-1");
        let tab = shell.open_tab(&window, "https://webox.dev").unwrap();

        shell
            .navigate_tab(&window, &tab, "https://example.com/one")
            .unwrap();
        shell
            .navigate_tab(&window, &tab, "https://example.com/two")
            .unwrap();

        shell.go_back(&window, &tab).unwrap();
        assert_eq!(
            shell.windows()[&window].tabs[0].url,
            "https://example.com/one"
        );

        shell.go_forward(&window, &tab).unwrap();
        assert_eq!(
            shell.windows()[&window].tabs[0].url,
            "https://example.com/two"
        );
    }

    #[test]
    fn host_shell_surfaces_live_readiness_failures() {
        let mut shell = HostShell::new(AppConfig::development());
        shell.start();
        let window = shell.create_window("window-1");

        let result = shell.open_tab(&window, "https://webox.dev");

        assert!(result.is_err());
        assert!(!shell.live_mvp_ready());
        assert!(!shell.runtime_readiness().missing_paths.is_empty());
        assert_eq!(shell.startup_diagnostics()[0].component, "engine.readiness");
    }

    #[test]
    fn host_shell_records_observed_memory_attribution() {
        let mut shell = HostShell::new(AppConfig::simulated());
        shell.start();
        let window = shell.create_window("window-1");
        let tab = shell.open_tab(&window, "https://webox.dev").unwrap();

        shell.collect_observed_tab_telemetry(&window, &tab).unwrap();

        let tab_state = shell.windows()[&window]
            .tabs
            .iter()
            .find(|candidate| candidate.id == tab)
            .unwrap();
        assert!(
            tab_state
                .memory_attribution
                .as_deref()
                .is_some_and(|attribution| attribution.contains("AggregateProcessRss"))
        );
        assert!(
            tab_state
                .memory_attribution
                .as_deref()
                .is_some_and(|attribution| attribution.contains("live_mvp_evidence=true"))
        );
    }
}
