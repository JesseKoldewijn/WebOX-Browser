use std::collections::HashMap;
use std::path::Path;

use cef::args::Args;
use cef::rc::Rc;
use cef::{self, App, ImplApp, LogItems, LogSeverity, Settings, WrapApp, wrap_app};
use webox_config::{AppConfig, BrowserRuntimeMode};

wrap_app! {
    struct WeboxCefApp {
        process_label: String,
    }

    impl App {
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserFramework {
    Cef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineState {
    Created,
    Started,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartupDiagnostics {
    pub component: &'static str,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineLaunchSettings {
    pub framework: BrowserFramework,
    pub subprocess_path: String,
    pub subprocess_args: Vec<String>,
    pub remote_debugging_port: u16,
    pub resources_dir: String,
    pub locales_dir: String,
    pub runtime_mode: BrowserRuntimeMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeBackend {
    Simulated,
    Cef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserSurfaceRenderMode {
    Placeholder,
    CefOffscreen,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserSurfaceState {
    pub surface_id: String,
    pub render_mode: BrowserSurfaceRenderMode,
    pub width: u32,
    pub height: u32,
    pub focused: bool,
    pub frame_token: u64,
    pub last_frame_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserInstanceDescriptor {
    pub id: String,
    pub initial_url: String,
    pub title: String,
    pub is_loading: bool,
    pub backend: RuntimeBackend,
    pub surface: BrowserSurfaceState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserInstanceState {
    pub id: String,
    pub url: String,
    pub title: String,
    pub is_loading: bool,
    pub backend: RuntimeBackend,
    pub memory_usage_bytes: u64,
    pub memory_indicator: Option<String>,
    pub failure_state: Option<String>,
    pub memory_attribution: Option<String>,
    pub surface: BrowserSurfaceState,
    pub history: Vec<String>,
    pub history_index: usize,
    pub status_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserInstanceEventKind {
    Created,
    LoadStarted,
    LoadFinished,
    NavigationFailed,
    HistoryChanged,
    TitleChanged,
    SurfaceUpdated,
    FocusChanged,
    MemoryUpdated,
    Crashed,
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserInstanceEvent {
    pub browser_id: String,
    pub kind: BrowserInstanceEventKind,
    pub summary: String,
    pub snapshot: Option<BrowserInstanceState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineError {
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CefRuntimePlan {
    pub selected_backend: RuntimeBackend,
    pub primary_target: &'static str,
    pub integration_crate: &'static str,
    pub ui_host: &'static str,
    pub distribution_root: String,
    pub browser_subprocess_path: String,
    pub resources_dir: String,
    pub locales_dir: String,
    pub settings_summary: String,
}

pub struct CefRuntimeBootstrap {
    args: Args,
    app: App,
    settings: Settings,
}

impl CefRuntimeBootstrap {
    fn from_config(config: &AppConfig) -> Self {
        let settings = Settings {
            no_sandbox: (!config.startup.enable_cef_sandbox) as i32,
            browser_subprocess_path: config.subprocess.browser_subprocess_path.as_str().into(),
            framework_dir_path: config.cef.framework_dir.as_str().into(),
            cache_path: config.paths.cache_dir.as_str().into(),
            root_cache_path: config.paths.cache_dir.as_str().into(),
            resources_dir_path: config.cef.resources_dir.as_str().into(),
            locales_dir_path: config.cef.locales_dir.as_str().into(),
            user_agent_product: "webox/0.1.0".into(),
            log_file: config.subprocess.log_file_path.as_str().into(),
            log_severity: LogSeverity::INFO,
            log_items: LogItems::DEFAULT,
            javascript_flags: "--max-old-space-size=8192".into(),
            remote_debugging_port: i32::from(config.startup.remote_debugging_port),
            uncaught_exception_stack_size: 32,
            multi_threaded_message_loop: 0,
            external_message_pump: 0,
            ..Settings::default()
        };

        Self {
            args: Args::new(),
            app: WeboxCefApp::new("webox-browser-process".to_string()),
            settings,
        }
    }

    fn execute_subprocess_if_needed(&mut self) -> Option<i32> {
        let exit_code = cef::execute_process(
            Some(self.args.as_main_args()),
            Some(&mut self.app),
            std::ptr::null_mut(),
        );
        if exit_code >= 0 {
            Some(exit_code)
        } else {
            None
        }
    }

    fn initialize(&mut self) -> bool {
        cef::initialize(
            Some(self.args.as_main_args()),
            Some(&self.settings),
            Some(&mut self.app),
            std::ptr::null_mut(),
        ) != 0
    }
}

pub struct WeboxEngine {
    state: EngineState,
    launch_settings: EngineLaunchSettings,
    diagnostics: Vec<StartupDiagnostics>,
    next_browser_instance: usize,
    runtime_backend: RuntimeBackend,
    browser_instances: HashMap<String, BrowserInstanceState>,
    pending_events: Vec<BrowserInstanceEvent>,
}

impl WeboxEngine {
    #[must_use]
    pub fn new(config: &AppConfig) -> Self {
        Self {
            state: EngineState::Created,
            launch_settings: EngineLaunchSettings {
                framework: BrowserFramework::Cef,
                subprocess_path: config.subprocess.browser_subprocess_path.clone(),
                subprocess_args: config.subprocess.extra_args.clone(),
                remote_debugging_port: config.startup.remote_debugging_port,
                resources_dir: config.cef.resources_dir.clone(),
                locales_dir: config.cef.locales_dir.clone(),
                runtime_mode: config.startup.runtime_mode,
            },
            diagnostics: Vec::new(),
            next_browser_instance: 1,
            runtime_backend: if matches!(config.startup.runtime_mode, BrowserRuntimeMode::RealCef) {
                RuntimeBackend::Cef
            } else {
                RuntimeBackend::Simulated
            },
            browser_instances: HashMap::new(),
            pending_events: Vec::new(),
        }
    }

    #[must_use]
    pub fn runtime_plan(config: &AppConfig) -> CefRuntimePlan {
        CefRuntimePlan {
            selected_backend: if matches!(config.startup.runtime_mode, BrowserRuntimeMode::RealCef)
            {
                RuntimeBackend::Cef
            } else {
                RuntimeBackend::Simulated
            },
            primary_target: "linux-x86_64",
            integration_crate: "cef (tauri-apps/cef-rs)",
            ui_host: "eframe/egui",
            distribution_root: config.cef.distribution_root.clone(),
            browser_subprocess_path: config.subprocess.browser_subprocess_path.clone(),
            resources_dir: config.cef.resources_dir.clone(),
            locales_dir: config.cef.locales_dir.clone(),
            settings_summary: format!(
                "remote_debugging_port={}, javascript_flags=--max-old-space-size=8192",
                config.startup.remote_debugging_port
            ),
        }
    }

    pub fn start(&mut self) -> StartupDiagnostics {
        let diagnostic = if matches!(
            self.launch_settings.runtime_mode,
            BrowserRuntimeMode::RealCef
        ) && Path::new(&self.launch_settings.subprocess_path).exists()
            && Path::new(&self.launch_settings.resources_dir).exists()
            && Path::new(&self.launch_settings.locales_dir).exists()
        {
            let config = AppConfig::development();
            let mut bootstrap = CefRuntimeBootstrap::from_config(&config);
            if let Some(exit_code) = bootstrap.execute_subprocess_if_needed() {
                self.runtime_backend = RuntimeBackend::Cef;
                StartupDiagnostics {
                    component: "engine.execute_process",
                    detail: format!("CEF subprocess executed and returned exit code {exit_code}"),
                }
            } else if bootstrap.initialize() {
                self.runtime_backend = RuntimeBackend::Cef;
                StartupDiagnostics {
                    component: "engine.bootstrap",
                    detail: format!(
                        "Initialized real CEF runtime with subprocess '{}' and resources '{}'",
                        self.launch_settings.subprocess_path, self.launch_settings.resources_dir
                    ),
                }
            } else {
                self.runtime_backend = RuntimeBackend::Simulated;
                StartupDiagnostics {
                    component: "engine.bootstrap",
                    detail: "CEF initialization failed; falling back to simulated runtime backend"
                        .to_string(),
                }
            }
        } else {
            self.runtime_backend = RuntimeBackend::Simulated;
            StartupDiagnostics {
                component: "engine.bootstrap",
                detail: format!(
                    "Initialized simulated runtime bootstrap with subprocess '{}' on port {}",
                    self.launch_settings.subprocess_path,
                    self.launch_settings.remote_debugging_port
                ),
            }
        };
        self.state = EngineState::Started;
        self.diagnostics.push(diagnostic.clone());
        diagnostic
    }

    pub fn shutdown(&mut self) -> StartupDiagnostics {
        let open_ids = self.browser_instances.keys().cloned().collect::<Vec<_>>();
        for browser_id in open_ids {
            self.push_event(
                &browser_id,
                BrowserInstanceEventKind::Closed,
                format!("Browser instance '{browser_id}' closed during engine shutdown"),
            );
        }
        self.browser_instances.clear();
        self.state = EngineState::Stopped;
        if matches!(self.runtime_backend, RuntimeBackend::Cef) {
            cef::shutdown();
        }
        let diagnostic = StartupDiagnostics {
            component: "engine.shutdown",
            detail: format!(
                "Engine shutdown requested and {:?} runtime cleanup completed",
                self.runtime_backend
            ),
        };
        self.diagnostics.push(diagnostic.clone());
        diagnostic
    }

    pub fn create_browser_instance(
        &mut self,
        initial_url: &str,
    ) -> Result<BrowserInstanceDescriptor, EngineError> {
        if self.state != EngineState::Started {
            return Err(EngineError {
                message: "Engine must be started before creating browser instances".to_string(),
            });
        }

        let browser_id = format!("browser-instance-{}", self.next_browser_instance);
        let surface_id = format!("browser-surface-{}", self.next_browser_instance);
        self.next_browser_instance += 1;

        let surface = BrowserSurfaceState {
            surface_id,
            render_mode: match self.runtime_backend {
                RuntimeBackend::Cef => BrowserSurfaceRenderMode::CefOffscreen,
                RuntimeBackend::Simulated => BrowserSurfaceRenderMode::Placeholder,
            },
            width: 1280,
            height: 720,
            focused: false,
            frame_token: 1,
            last_frame_label: format!("Preparing {initial_url}"),
        };

        let descriptor = BrowserInstanceDescriptor {
            id: browser_id.clone(),
            initial_url: initial_url.to_string(),
            title: "Loading...".to_string(),
            is_loading: true,
            backend: self.runtime_backend,
            surface: surface.clone(),
        };

        self.browser_instances.insert(
            browser_id.clone(),
            BrowserInstanceState {
                id: browser_id.clone(),
                url: initial_url.to_string(),
                title: descriptor.title.clone(),
                is_loading: descriptor.is_loading,
                backend: descriptor.backend,
                memory_usage_bytes: 0,
                memory_indicator: None,
                failure_state: None,
                memory_attribution: None,
                surface,
                history: vec![initial_url.to_string()],
                history_index: 0,
                status_text: format!("Created live browser instance for {initial_url}"),
            },
        );

        self.diagnostics.push(StartupDiagnostics {
            component: "engine.browser-instance",
            detail: format!(
                "Prepared {:?} browser initialization flow for '{}' as {}",
                self.runtime_backend, initial_url, browser_id
            ),
        });
        self.push_event(
            &browser_id,
            BrowserInstanceEventKind::Created,
            format!("Created browser instance '{browser_id}'"),
        );
        Ok(descriptor)
    }

    pub fn navigate_browser_instance(
        &mut self,
        browser_id: &str,
        url: &str,
    ) -> Result<(), EngineError> {
        let backend = {
            let instance = self.browser_instance_mut(browser_id)?;
            instance.url = url.to_string();
            instance.title = "Loading...".to_string();
            instance.is_loading = true;
            instance.failure_state = None;
            instance.history.truncate(instance.history_index + 1);
            instance.history.push(url.to_string());
            instance.history_index = instance.history.len() - 1;
            instance.status_text = format!("Navigating to {url}");
            instance.surface.frame_token += 1;
            instance.surface.last_frame_label = format!("Loading {url}");
            instance.backend
        };
        self.diagnostics.push(StartupDiagnostics {
            component: "engine.navigation",
            detail: format!(
                "Queued {:?} navigation for '{}' to '{}'",
                backend, browser_id, url
            ),
        });
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::LoadStarted,
            format!("Navigation started for '{browser_id}' to '{url}'"),
        );
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::HistoryChanged,
            format!("History updated for '{browser_id}'"),
        );
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::SurfaceUpdated,
            format!("Surface updated for '{browser_id}' after navigation start"),
        );
        Ok(())
    }

    pub fn finish_navigation(&mut self, browser_id: &str, title: &str) -> Result<(), EngineError> {
        let instance = self.browser_instance_mut(browser_id)?;
        instance.title = title.to_string();
        instance.is_loading = false;
        instance.failure_state = None;
        instance.status_text = format!("Loaded {}", instance.url);
        instance.surface.frame_token += 1;
        instance.surface.last_frame_label = format!("{title} ({})", instance.url);
        self.diagnostics.push(StartupDiagnostics {
            component: "engine.navigation",
            detail: format!(
                "Browser instance '{}' finished navigation with title '{}'",
                browser_id, title
            ),
        });
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::TitleChanged,
            format!("Title updated for '{browser_id}' to '{title}'"),
        );
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::LoadFinished,
            format!("Navigation finished for '{browser_id}'"),
        );
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::SurfaceUpdated,
            format!("Surface updated for '{browser_id}' after load finished"),
        );
        Ok(())
    }

    pub fn fail_navigation(&mut self, browser_id: &str, message: &str) -> Result<(), EngineError> {
        let instance = self.browser_instance_mut(browser_id)?;
        instance.is_loading = false;
        instance.failure_state = Some(message.to_string());
        instance.status_text = format!("Navigation failed: {message}");
        instance.surface.frame_token += 1;
        instance.surface.last_frame_label = format!("Navigation failed: {message}");
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::NavigationFailed,
            format!("Navigation failed for '{browser_id}': {message}"),
        );
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::SurfaceUpdated,
            format!("Surface updated for '{browser_id}' after navigation failure"),
        );
        Ok(())
    }

    pub fn crash_browser_instance(
        &mut self,
        browser_id: &str,
        message: &str,
    ) -> Result<(), EngineError> {
        let instance = self.browser_instance_mut(browser_id)?;
        instance.is_loading = false;
        instance.failure_state = Some(message.to_string());
        instance.status_text = format!("Browser instance crashed: {message}");
        instance.surface.frame_token += 1;
        instance.surface.last_frame_label = format!("Crash: {message}");
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::Crashed,
            format!("Browser instance '{browser_id}' crashed: {message}"),
        );
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::SurfaceUpdated,
            format!("Surface updated for '{browser_id}' after crash"),
        );
        Ok(())
    }

    pub fn go_back_browser_instance(&mut self, browser_id: &str) -> Result<(), EngineError> {
        let instance = self.browser_instance_mut(browser_id)?;
        if instance.history_index > 0 {
            instance.history_index -= 1;
            instance.url = instance.history[instance.history_index].clone();
            instance.is_loading = false;
            instance.failure_state = None;
            instance.title = instance.url.clone();
            instance.status_text = format!("Went back to {}", instance.url);
            instance.surface.frame_token += 1;
            instance.surface.last_frame_label = format!("History: {}", instance.url);
            self.push_event(
                browser_id,
                BrowserInstanceEventKind::HistoryChanged,
                format!("Browser instance '{browser_id}' moved back in history"),
            );
            self.push_event(
                browser_id,
                BrowserInstanceEventKind::SurfaceUpdated,
                format!("Surface updated for '{browser_id}' after back navigation"),
            );
        }
        Ok(())
    }

    pub fn go_forward_browser_instance(&mut self, browser_id: &str) -> Result<(), EngineError> {
        let instance = self.browser_instance_mut(browser_id)?;
        if instance.history_index + 1 < instance.history.len() {
            instance.history_index += 1;
            instance.url = instance.history[instance.history_index].clone();
            instance.is_loading = false;
            instance.failure_state = None;
            instance.title = instance.url.clone();
            instance.status_text = format!("Went forward to {}", instance.url);
            instance.surface.frame_token += 1;
            instance.surface.last_frame_label = format!("History: {}", instance.url);
            self.push_event(
                browser_id,
                BrowserInstanceEventKind::HistoryChanged,
                format!("Browser instance '{browser_id}' moved forward in history"),
            );
            self.push_event(
                browser_id,
                BrowserInstanceEventKind::SurfaceUpdated,
                format!("Surface updated for '{browser_id}' after forward navigation"),
            );
        }
        Ok(())
    }

    pub fn resize_browser_surface(
        &mut self,
        browser_id: &str,
        width: u32,
        height: u32,
    ) -> Result<(), EngineError> {
        let instance = self.browser_instance_mut(browser_id)?;
        instance.surface.width = width;
        instance.surface.height = height;
        instance.surface.frame_token += 1;
        instance.surface.last_frame_label = format!("{} ({}x{})", instance.title, width, height);
        instance.status_text = format!("Resized surface to {width}x{height}");
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::SurfaceUpdated,
            format!("Surface resized for '{browser_id}' to {width}x{height}"),
        );
        Ok(())
    }

    pub fn set_surface_focus(
        &mut self,
        browser_id: &str,
        focused: bool,
    ) -> Result<(), EngineError> {
        let instance = self.browser_instance_mut(browser_id)?;
        instance.surface.focused = focused;
        instance.status_text = if focused {
            "Surface focused".to_string()
        } else {
            "Surface blurred".to_string()
        };
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::FocusChanged,
            format!("Surface focus for '{browser_id}' changed to {focused}"),
        );
        Ok(())
    }

    pub fn close_browser_instance(&mut self, browser_id: &str) -> Result<(), EngineError> {
        let removed = self
            .browser_instances
            .remove(browser_id)
            .ok_or_else(|| EngineError {
                message: format!("Browser instance '{}' was not found", browser_id),
            })?;
        self.diagnostics.push(StartupDiagnostics {
            component: "engine.browser-instance",
            detail: format!(
                "Disposed {:?} browser instance '{}'",
                removed.backend, browser_id
            ),
        });
        self.pending_events.push(BrowserInstanceEvent {
            browser_id: browser_id.to_string(),
            kind: BrowserInstanceEventKind::Closed,
            summary: format!("Disposed browser instance '{browser_id}'"),
            snapshot: None,
        });
        Ok(())
    }

    pub fn update_browser_memory(
        &mut self,
        browser_id: &str,
        memory_usage_bytes: u64,
        memory_indicator: Option<String>,
        failure_state: Option<String>,
        attribution: Option<String>,
    ) -> Result<(), EngineError> {
        let indicator_label = {
            let instance = self.browser_instance_mut(browser_id)?;
            instance.memory_usage_bytes = memory_usage_bytes;
            instance.memory_indicator = memory_indicator;
            instance.failure_state = failure_state;
            instance.memory_attribution = attribution;
            let indicator_label = instance
                .memory_indicator
                .clone()
                .unwrap_or_else(|| "normal".to_string());
            instance.status_text = format!(
                "Observed memory state: {} bytes ({})",
                memory_usage_bytes, indicator_label
            );
            indicator_label
        };
        self.diagnostics.push(StartupDiagnostics {
            component: "engine.memory",
            detail: format!(
                "Updated memory state for '{}' to {} bytes ({})",
                browser_id, memory_usage_bytes, indicator_label
            ),
        });
        self.push_event(
            browser_id,
            BrowserInstanceEventKind::MemoryUpdated,
            format!("Memory updated for '{browser_id}'"),
        );
        Ok(())
    }

    #[must_use]
    pub fn browser_instance(&self, browser_id: &str) -> Option<&BrowserInstanceState> {
        self.browser_instances.get(browser_id)
    }

    #[must_use]
    pub fn browser_instances(&self) -> Vec<BrowserInstanceState> {
        let mut instances = self.browser_instances.values().cloned().collect::<Vec<_>>();
        instances.sort_by(|left, right| left.id.cmp(&right.id));
        instances
    }

    #[must_use]
    pub fn state(&self) -> EngineState {
        self.state
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[StartupDiagnostics] {
        &self.diagnostics
    }

    #[must_use]
    pub fn launch_settings(&self) -> &EngineLaunchSettings {
        &self.launch_settings
    }

    #[must_use]
    pub fn runtime_backend(&self) -> RuntimeBackend {
        self.runtime_backend
    }

    pub fn drain_events(&mut self) -> Vec<BrowserInstanceEvent> {
        self.pending_events.drain(..).collect()
    }

    fn browser_instance_mut(
        &mut self,
        browser_id: &str,
    ) -> Result<&mut BrowserInstanceState, EngineError> {
        self.browser_instances
            .get_mut(browser_id)
            .ok_or_else(|| EngineError {
                message: format!("Browser instance '{}' was not found", browser_id),
            })
    }

    fn push_event(&mut self, browser_id: &str, kind: BrowserInstanceEventKind, summary: String) {
        self.pending_events.push(BrowserInstanceEvent {
            browser_id: browser_id.to_string(),
            kind,
            summary,
            snapshot: self.browser_instances.get(browser_id).cloned(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BrowserInstanceEventKind, BrowserSurfaceRenderMode, EngineState, RuntimeBackend,
        WeboxEngine,
    };
    use webox_config::AppConfig;

    #[test]
    fn engine_starts_and_creates_browser_instance() {
        let mut engine = WeboxEngine::new(&AppConfig::simulated());
        let _ = engine.start();
        let instance = engine
            .create_browser_instance("https://example.com")
            .unwrap();
        assert_eq!(engine.state(), EngineState::Started);
        assert_eq!(instance.id, "browser-instance-1");
        assert_eq!(instance.title, "Loading...");
        assert_eq!(engine.runtime_backend(), RuntimeBackend::Simulated);
        assert_eq!(
            instance.surface.render_mode,
            BrowserSurfaceRenderMode::Placeholder
        );
    }

    #[test]
    fn engine_tracks_live_browser_instance_state() {
        let mut engine = WeboxEngine::new(&AppConfig::simulated());
        let _ = engine.start();
        let instance = engine
            .create_browser_instance("https://example.com")
            .unwrap();

        engine
            .navigate_browser_instance(&instance.id, "https://example.com/dashboard")
            .unwrap();
        engine
            .finish_navigation(&instance.id, "Example Dashboard")
            .unwrap();
        engine
            .update_browser_memory(
                &instance.id,
                123,
                Some("memory warning".to_string()),
                None,
                Some("precise: renderer metrics".to_string()),
            )
            .unwrap();
        engine
            .resize_browser_surface(&instance.id, 1440, 900)
            .unwrap();
        engine.set_surface_focus(&instance.id, true).unwrap();

        let state = engine.browser_instance(&instance.id).unwrap();
        assert_eq!(state.url, "https://example.com/dashboard");
        assert_eq!(state.title, "Example Dashboard");
        assert_eq!(state.memory_indicator.as_deref(), Some("memory warning"));
        assert_eq!(state.surface.width, 1440);
        assert!(state.surface.focused);

        let events = engine.drain_events();
        assert!(
            events
                .iter()
                .any(|event| event.kind == BrowserInstanceEventKind::Created)
        );
        assert!(
            events
                .iter()
                .any(|event| event.kind == BrowserInstanceEventKind::LoadFinished)
        );
        assert!(
            events
                .iter()
                .any(|event| event.kind == BrowserInstanceEventKind::SurfaceUpdated)
        );

        engine.close_browser_instance(&instance.id).unwrap();
        assert!(engine.browser_instance(&instance.id).is_none());
    }

    #[test]
    fn engine_history_navigation_uses_live_state() {
        let mut engine = WeboxEngine::new(&AppConfig::simulated());
        let _ = engine.start();
        let instance = engine
            .create_browser_instance("https://example.com")
            .unwrap();
        engine
            .navigate_browser_instance(&instance.id, "https://example.com/one")
            .unwrap();
        engine
            .navigate_browser_instance(&instance.id, "https://example.com/two")
            .unwrap();

        engine.go_back_browser_instance(&instance.id).unwrap();
        assert_eq!(
            engine.browser_instance(&instance.id).unwrap().url,
            "https://example.com/one"
        );

        engine.go_forward_browser_instance(&instance.id).unwrap();
        assert_eq!(
            engine.browser_instance(&instance.id).unwrap().url,
            "https://example.com/two"
        );
    }

    #[test]
    fn runtime_plan_selects_real_cef_linux_target() {
        let plan = WeboxEngine::runtime_plan(&AppConfig::development());
        assert_eq!(plan.primary_target, "linux-x86_64");
        assert_eq!(plan.integration_crate, "cef (tauri-apps/cef-rs)");
        assert_eq!(plan.ui_host, "eframe/egui");
    }
}
