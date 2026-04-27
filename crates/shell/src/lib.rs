use std::collections::HashMap;

use webox_config::AppConfig;
use webox_engine::{StartupDiagnostics, WeboxEngine};
use webox_memory::{
    MemoryController, MemoryPressureLevel, PolicyDecision, RecoveryReport, SupportedSystemReport,
    TabTelemetry,
};
use webox_ui::{BrowserCommand, BrowserWindowModel, WindowId};

pub struct HostShell {
    config: AppConfig,
    engine: WeboxEngine,
    memory_controller: MemoryController,
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
            windows: HashMap::new(),
            startup_diagnostics: Vec::new(),
            recovery_reports: Vec::new(),
            tab_runtime_modes: HashMap::new(),
        }
    }

    pub fn start(&mut self) {
        self.startup_diagnostics.push(self.engine.start());
    }

    pub fn shutdown(&mut self) {
        self.startup_diagnostics.push(self.engine.shutdown());
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
        let window = self
            .windows
            .get_mut(window_id)
            .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
        let tab_id = window.add_tab(descriptor.id.clone(), descriptor.initial_url.clone());
        if let Some(tab) = window.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.title = descriptor.title;
            tab.is_loading = descriptor.is_loading;
        }
        self.tab_runtime_modes
            .insert(tab_id.clone(), format!("{:?}", descriptor.backend));
        Ok(tab_id)
    }

    pub fn navigate_tab(&mut self, window_id: &str, tab_id: &str, url: &str) -> Result<(), String> {
        self.engine
            .navigate_browser_instance(tab_id, url)
            .map_err(|error| error.message)?;
        let window = self
            .windows
            .get_mut(window_id)
            .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
        window.navigate_tab(tab_id, url);
        Ok(())
    }

    pub fn finish_navigation(
        &mut self,
        window_id: &str,
        tab_id: &str,
        title: &str,
    ) -> Result<(), String> {
        self.engine
            .finish_navigation(tab_id, title)
            .map_err(|error| error.message)?;
        let window = self
            .windows
            .get_mut(window_id)
            .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
        window.finish_loading(tab_id, title);
        Ok(())
    }

    pub fn go_back(&mut self, window_id: &str, tab_id: &str) -> Result<(), String> {
        let window = self
            .windows
            .get_mut(window_id)
            .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
        window.go_back(tab_id);
        Ok(())
    }

    pub fn go_forward(&mut self, window_id: &str, tab_id: &str) -> Result<(), String> {
        let window = self
            .windows
            .get_mut(window_id)
            .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
        window.go_forward(tab_id);
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
        Ok(())
    }

    pub fn record_tab_telemetry(
        &mut self,
        window_id: &str,
        telemetry: &TabTelemetry,
    ) -> Result<PolicyDecision, String> {
        let decision = self.memory_controller.evaluate(telemetry);
        let window = self
            .windows
            .get_mut(window_id)
            .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
        let label = match decision.event.level {
            MemoryPressureLevel::Normal => None,
            MemoryPressureLevel::Warning => Some("memory warning".to_string()),
            MemoryPressureLevel::Critical => Some("critical memory pressure".to_string()),
            MemoryPressureLevel::Exhausted => Some("memory exhaustion risk".to_string()),
        };
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
            )
            .map_err(|error| error.message)?;
        window.set_memory_indicator(&telemetry.tab_id, label);
        if matches!(decision.event.level, MemoryPressureLevel::Exhausted) {
            let report = self.memory_controller.capture_recovery_report(telemetry);
            window.set_failure_state(
                &telemetry.tab_id,
                Some("Tab ended due to suspected memory exhaustion".to_string()),
            );
            self.recovery_reports.push(report);
        }
        Ok(decision)
    }

    pub fn dispatch_command(
        &mut self,
        window_id: &str,
        command: BrowserCommand,
    ) -> Result<(), String> {
        match command {
            BrowserCommand::Navigate { tab_id, url } => self.navigate_tab(window_id, &tab_id, &url),
            BrowserCommand::Reload { tab_id } => {
                let current_url = self
                    .windows
                    .get(window_id)
                    .and_then(|window| window.tabs.iter().find(|tab| tab.id == tab_id))
                    .map(|tab| tab.url.clone())
                    .ok_or_else(|| format!("Tab '{}' was not found", tab_id))?;
                self.navigate_tab(window_id, &tab_id, &current_url)
            }
            BrowserCommand::Back { tab_id } => self.go_back(window_id, &tab_id),
            BrowserCommand::Forward { tab_id } => self.go_forward(window_id, &tab_id),
            BrowserCommand::ActivateTab { tab_id } => {
                let window = self
                    .windows
                    .get_mut(window_id)
                    .ok_or_else(|| format!("Window '{}' was not found", window_id))?;
                window.set_active_tab(&tab_id);
                Ok(())
            }
            BrowserCommand::CloseTab { tab_id } => self.close_tab(window_id, &tab_id),
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
}

#[cfg(test)]
mod tests {
    use super::HostShell;
    use webox_config::AppConfig;
    use webox_memory::TabTelemetry;
    use webox_ui::BrowserCommand;

    #[test]
    fn host_shell_starts_and_opens_tabs() {
        let mut shell = HostShell::new(AppConfig::development());
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
        let mut shell = HostShell::new(AppConfig::development());
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
}
