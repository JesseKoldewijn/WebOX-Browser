use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use cef::args::Args;
use cef::rc::Rc;
use cef::{
    self, App, Browser, BrowserSettings, CefString, Client, CommandLine, ContextMenuHandler,
    ContextMenuParams, DisplayHandler, Errorcode, Frame, ImplApp, ImplBrowser, ImplBrowserHost,
    ImplClient, ImplCommandLine, ImplContextMenuHandler, ImplDisplayHandler, ImplFrame,
    ImplLifeSpanHandler, ImplLoadHandler, ImplMenuModel, ImplRenderHandler, KeyEvent, KeyEventType,
    LifeSpanHandler, LoadHandler, LogItems, LogSeverity, MenuModel, MouseButtonType, MouseEvent,
    PaintElementType, Rect, RenderHandler, RuntimeStyle, Settings, TransitionType, WindowInfo,
    WrapApp, WrapClient, WrapContextMenuHandler, WrapDisplayHandler, WrapLifeSpanHandler,
    WrapLoadHandler, WrapRenderHandler, wrap_app, wrap_client, wrap_context_menu_handler,
    wrap_display_handler, wrap_life_span_handler, wrap_load_handler, wrap_render_handler,
};
use webox_config::{AppConfig, BrowserRuntimeMode};

wrap_app! {
    struct WeboxCefApp {
        process_label: String,
    }

    impl App {
        fn on_before_command_line_processing(
            &self,
            _process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            // Apply the minimal set of CEF flags for the detected runtime
            // environment. Flags are chosen based on actual hardware support
            // rather than assumptions, so native Linux/Windows/macOS get full
            // hardware GPU while WSL gets a safe software fallback.
            if let Some(cmd) = command_line {
                let env = detect_runtime_environment();

                match env {
                    RuntimeEnvironment::Wsl => {
                        // WSL2 requires several flags that native Linux does not.
                        // Most critically, without --no-zygote the zygote process
                        // tries to set up PID namespaces which WSL blocks, causing
                        // all subprocesses (GPU, renderer) to fail with
                        // error_code=1002 (LAUNCH_RESULT_FAILED_TO_START) before
                        // they can even run our on_before_command_line_processing.
                        //
                        // Hardware GPU (dzn/D3D12) also crashes in WSL with
                        // exit_code=139 (SIGSEGV), so we disable the GPU process
                        // and use SwiftShader (CPU-based Vulkan) for WebGL/WebGPU
                        // instead.

                        // Disable hardware GPU — dzn crashes on WSL (SIGSEGV).
                        // SwiftShader below provides software WebGL/WebGPU.
                        let v = CefString::from("disable-gpu");
                        cmd.append_switch(Some(&v));
                        // Disable the GPU compositing path in renderer processes.
                        let v = CefString::from("disable-gpu-compositing");
                        cmd.append_switch(Some(&v));
                        // Disable GPU process sandbox — required so the GPU
                        // subprocess (hosting SwiftShader) can be created in WSL.
                        let v = CefString::from("disable-gpu-sandbox");
                        cmd.append_switch(Some(&v));
                        // Disable all process sandboxing — WSL blocks the seccomp
                        // / setuid mechanisms Chromium needs for sandbox setup.
                        let v = CefString::from("no-sandbox");
                        cmd.append_switch(Some(&v));
                        let v = CefString::from("disable-setuid-sandbox");
                        cmd.append_switch(Some(&v));
                        // Skip the zygote — WSL PID namespace setup in the zygote
                        // fails and prevents any subprocess from being spawned.
                        let v = CefString::from("no-zygote");
                        cmd.append_switch(Some(&v));
                        // WSL2 /dev/shm has a small fixed quota; overflow causes
                        // silent renderer crashes before on_paint fires.
                        let v = CefString::from("disable-dev-shm-usage");
                        cmd.append_switch(Some(&v));
                        // SwiftShader (CPU Vulkan) provides WebGL/WebGPU without
                        // needing a hardware GPU process. Chromium 117+ requires
                        // an explicit opt-in.
                        let v = CefString::from("enable-unsafe-swiftshader");
                        cmd.append_switch(Some(&v));
                    }
                    RuntimeEnvironment::Linux => {
                        // Native Linux with a real GPU driver stack — no process
                        // creation or GPU flags needed. Add SwiftShader as a
                        // safety net if hardware GL fails for a specific operation.
                        let v = CefString::from("enable-unsafe-swiftshader");
                        cmd.append_switch(Some(&v));
                    }
                    RuntimeEnvironment::Windows | RuntimeEnvironment::MacOs => {
                        // Native GPU support — no special flags required.
                    }
                }

                // WebGPU is still behind a flag in CEF 147 — enable on all
                // platforms so sites can use it where supported.
                let v = CefString::from("enable-unsafe-webgpu");
                cmd.append_switch(Some(&v));
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveBrowserCallbackKind {
    AfterCreated {
        cef_identifier: i32,
    },
    BeforeClose,
    TitleChanged {
        title: String,
    },
    AddressChanged {
        url: String,
    },
    LoadingStateChanged {
        is_loading: bool,
        can_go_back: bool,
        can_go_forward: bool,
    },
    LoadStarted,
    LoadFinished {
        http_status_code: i32,
    },
    LoadError {
        code: String,
        text: String,
        url: String,
    },
    Painted {
        width: i32,
        height: i32,
        dirty_rects: usize,
        buffer: Option<Vec<u8>>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveBrowserCallbackEvent {
    pub browser_id: String,
    pub kind: LiveBrowserCallbackKind,
}

#[derive(Clone, Default)]
pub struct LiveBrowserEventSink {
    events: Arc<Mutex<Vec<LiveBrowserCallbackEvent>>>,
}

impl LiveBrowserEventSink {
    fn push(&self, browser_id: &str, kind: LiveBrowserCallbackKind) {
        if let Ok(mut events) = self.events.lock() {
            events.push(LiveBrowserCallbackEvent {
                browser_id: browser_id.to_string(),
                kind,
            });
        }
    }

    fn drain(&self) -> Vec<LiveBrowserCallbackEvent> {
        self.events
            .lock()
            .map(|mut events| events.drain(..).collect())
            .unwrap_or_default()
    }
}

wrap_life_span_handler! {
    struct WeboxLifeSpanHandler {
        browser_id: String,
        event_sink: LiveBrowserEventSink,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let cef_identifier = browser.map(|browser| browser.identifier()).unwrap_or_default();
            self.event_sink.push(
                &self.browser_id,
                LiveBrowserCallbackKind::AfterCreated { cef_identifier },
            );
        }

        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            self.event_sink
                .push(&self.browser_id, LiveBrowserCallbackKind::BeforeClose);
        }
    }
}

wrap_display_handler! {
    struct WeboxDisplayHandler {
        browser_id: String,
        event_sink: LiveBrowserEventSink,
    }

    impl DisplayHandler {
        fn on_address_change(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            url: Option<&CefString>,
        ) {
            let url = url.map(ToString::to_string).unwrap_or_default();
            self.event_sink.push(
                &self.browser_id,
                LiveBrowserCallbackKind::AddressChanged { url },
            );
        }

        fn on_title_change(&self, _browser: Option<&mut Browser>, title: Option<&CefString>) {
            let title = title.map(ToString::to_string).unwrap_or_default();
            self.event_sink.push(
                &self.browser_id,
                LiveBrowserCallbackKind::TitleChanged { title },
            );
        }
    }
}

wrap_load_handler! {
    struct WeboxLoadHandler {
        browser_id: String,
        event_sink: LiveBrowserEventSink,
    }

    impl LoadHandler {
        fn on_loading_state_change(
            &self,
            _browser: Option<&mut Browser>,
            is_loading: i32,
            can_go_back: i32,
            can_go_forward: i32,
        ) {
            self.event_sink.push(
                &self.browser_id,
                LiveBrowserCallbackKind::LoadingStateChanged {
                    is_loading: is_loading != 0,
                    can_go_back: can_go_back != 0,
                    can_go_forward: can_go_forward != 0,
                },
            );
        }

        fn on_load_start(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _transition_type: TransitionType,
        ) {
            self.event_sink
                .push(&self.browser_id, LiveBrowserCallbackKind::LoadStarted);
        }

        fn on_load_end(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            http_status_code: i32,
        ) {
            self.event_sink.push(
                &self.browser_id,
                LiveBrowserCallbackKind::LoadFinished { http_status_code },
            );
        }

        fn on_load_error(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            error_code: Errorcode,
            error_text: Option<&CefString>,
            failed_url: Option<&CefString>,
        ) {
            self.event_sink.push(
                &self.browser_id,
                LiveBrowserCallbackKind::LoadError {
                    code: format!("{:?}", error_code),
                    text: error_text.map(ToString::to_string).unwrap_or_default(),
                    url: failed_url.map(ToString::to_string).unwrap_or_default(),
                },
            );
        }
    }
}

wrap_render_handler! {
    struct WeboxRenderHandler {
        browser_id: String,
        event_sink: LiveBrowserEventSink,
        width: Arc<AtomicI32>,
        height: Arc<AtomicI32>,
    }

    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            if let Some(rect) = rect {
                rect.x = 0;
                rect.y = 0;
                rect.width = self.width.load(Ordering::Relaxed);
                rect.height = self.height.load(Ordering::Relaxed);
            }
        }

        fn on_paint(
            &self,
            _browser: Option<&mut Browser>,
            _type_: PaintElementType,
            dirty_rects: Option<&[Rect]>,
            buffer: *const u8,
            width: i32,
            height: i32,
        ) {
            let buffer = if width > 0 && height > 0 && !buffer.is_null() {
                let byte_len = width as usize * height as usize * 4;
                // CEF provides BGRA bytes that are valid for the duration of this callback.
                Some(unsafe { std::slice::from_raw_parts(buffer, byte_len) }.to_vec())
            } else {
                None
            };
            self.event_sink.push(
                &self.browser_id,
                LiveBrowserCallbackKind::Painted {
                    width,
                    height,
                    dirty_rects: dirty_rects.map(|rects| rects.len()).unwrap_or_default(),
                    buffer,
                },
            );
        }
    }
}

wrap_context_menu_handler! {
    struct WeboxContextMenuHandler;

    impl ContextMenuHandler {
        fn on_before_context_menu(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _params: Option<&mut ContextMenuParams>,
            model: Option<&mut MenuModel>,
        ) {
            // OSR mode has no native window handle. Clear the model so CEF
            // never attempts to open an Aura/X11 context-menu window.
            if let Some(m) = model {
                m.clear();
            }
        }
    }
}

wrap_client! {
    struct WeboxCefClient {
        browser_id: String,
        life_span_handler: LifeSpanHandler,
        display_handler: DisplayHandler,
        load_handler: LoadHandler,
        render_handler: RenderHandler,
        context_menu_handler: ContextMenuHandler,
    }

    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(self.life_span_handler.clone())
        }

        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(self.display_handler.clone())
        }

        fn load_handler(&self) -> Option<LoadHandler> {
            Some(self.load_handler.clone())
        }

        fn render_handler(&self) -> Option<RenderHandler> {
            Some(self.render_handler.clone())
        }

        fn context_menu_handler(&self) -> Option<ContextMenuHandler> {
            Some(self.context_menu_handler.clone())
        }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeReadinessState {
    NotStarted,
    Simulated,
    LiveReady,
    LiveUnavailable,
    InitializationFailed,
    SubprocessExited,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeReadiness {
    pub state: RuntimeReadinessState,
    pub live_mvp_ready: bool,
    pub simulated: bool,
    pub summary: String,
    pub missing_paths: Vec<String>,
    pub checked_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineLaunchSettings {
    pub framework: BrowserFramework,
    pub distribution_root: String,
    pub subprocess_path: String,
    pub subprocess_args: Vec<String>,
    pub remote_debugging_port: u16,
    pub resources_dir: String,
    pub locales_dir: String,
    pub data_dir: String,
    pub cache_dir: String,
    pub log_dir: String,
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
pub struct BrowserFrameBuffer {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
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
    pub render_evidence: Option<String>,
    pub frame_buffer: Option<BrowserFrameBuffer>,
    pub damage_events: u64,
    pub host_surface_failure: Option<String>,
}

pub struct LiveCefBrowserInstance {
    browser: Browser,
    _client: Client,
    event_sink: LiveBrowserEventSink,
    view_width: Arc<AtomicI32>,
    view_height: Arc<AtomicI32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostMouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostSurfaceInputEvent {
    PointerMove {
        x: i32,
        y: i32,
    },
    PointerButton {
        x: i32,
        y: i32,
        button: HostMouseButton,
        pressed: bool,
        click_count: i32,
    },
    Wheel {
        x: i32,
        y: i32,
        delta_x: i32,
        delta_y: i32,
    },
    Key {
        key_code: i32,
        pressed: bool,
    },
    Text {
        text: String,
    },
    Focus {
        focused: bool,
    },
    Resize {
        width: u32,
        height: u32,
    },
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
    pub can_go_back: bool,
    pub can_go_forward: bool,
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

/// Runtime environment detected at startup, used to select the minimal set of
/// CEF command-line flags needed for the current platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeEnvironment {
    /// Windows Subsystem for Linux — GPU process cannot launch (error_code=1002),
    /// sandbox/zygote work but /dev/shm is too small.
    Wsl,
    /// Native Linux with a real GPU driver stack.
    Linux,
    /// Native Windows.
    #[allow(dead_code)]
    Windows,
    /// Native macOS.
    #[allow(dead_code)]
    MacOs,
}

fn detect_runtime_environment() -> RuntimeEnvironment {
    #[cfg(target_os = "macos")]
    {
        return RuntimeEnvironment::MacOs;
    }
    #[cfg(target_os = "windows")]
    {
        return RuntimeEnvironment::Windows;
    }
    #[cfg(target_os = "linux")]
    {
        // WSL sets "microsoft" (case-insensitive) in /proc/version.
        // This is the canonical detection method used by most tooling.
        if let Ok(version) = std::fs::read_to_string("/proc/version")
            && version.to_ascii_lowercase().contains("microsoft")
        {
            return RuntimeEnvironment::Wsl;
        }
        RuntimeEnvironment::Linux
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        RuntimeEnvironment::Linux
    }
}

pub struct CefRuntimeBootstrap {
    args: Args,
    app: App,
    settings: Settings,
}

impl CefRuntimeBootstrap {
    fn from_config(config: &AppConfig) -> Self {
        // Configure the CEF API version before any other CEF call.
        // Without this, cef_api_version() returns -1, causing a crash in
        // CefApp_0_CToCpp with "invalid version -1" when execute_process is called.
        cef::api_hash(cef::sys::CEF_API_VERSION, 0);

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
            log_severity: LogSeverity::VERBOSE,
            log_items: LogItems::DEFAULT,
            javascript_flags: "--max-old-space-size=8192".into(),
            remote_debugging_port: i32::from(config.startup.remote_debugging_port),
            windowless_rendering_enabled: 1,
            uncaught_exception_stack_size: 32,
            multi_threaded_message_loop: 0,
            // External message pump: we call cef::do_message_loop_work() manually
            // from the host UI frame tick so CEF can process messages without
            // owning the main thread event loop (which eframe already owns).
            external_message_pump: 1,
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
    config: AppConfig,
    state: EngineState,
    launch_settings: EngineLaunchSettings,
    diagnostics: Vec<StartupDiagnostics>,
    runtime_readiness: RuntimeReadiness,
    next_browser_instance: usize,
    runtime_backend: RuntimeBackend,
    browser_instances: HashMap<String, BrowserInstanceState>,
    live_browser_instances: HashMap<String, LiveCefBrowserInstance>,
    pending_events: Vec<BrowserInstanceEvent>,
}

impl WeboxEngine {
    #[must_use]
    pub fn new(config: &AppConfig) -> Self {
        Self {
            config: config.clone(),
            state: EngineState::Created,
            launch_settings: EngineLaunchSettings {
                framework: BrowserFramework::Cef,
                distribution_root: config.cef.distribution_root.clone(),
                subprocess_path: config.subprocess.browser_subprocess_path.clone(),
                subprocess_args: config.subprocess.extra_args.clone(),
                remote_debugging_port: config.startup.remote_debugging_port,
                resources_dir: config.cef.resources_dir.clone(),
                locales_dir: config.cef.locales_dir.clone(),
                data_dir: config.paths.data_dir.clone(),
                cache_dir: config.paths.cache_dir.clone(),
                log_dir: config.paths.log_dir.clone(),
                runtime_mode: config.startup.runtime_mode,
            },
            diagnostics: Vec::new(),
            runtime_readiness: RuntimeReadiness {
                state: RuntimeReadinessState::NotStarted,
                live_mvp_ready: false,
                simulated: false,
                summary: "Engine has not started".to_string(),
                missing_paths: Vec::new(),
                checked_paths: Vec::new(),
            },
            next_browser_instance: 1,
            runtime_backend: if matches!(config.startup.runtime_mode, BrowserRuntimeMode::RealCef) {
                RuntimeBackend::Cef
            } else {
                RuntimeBackend::Simulated
            },
            browser_instances: HashMap::new(),
            live_browser_instances: HashMap::new(),
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
        ) {
            let readiness = self.check_live_runtime_readiness();
            if !readiness.missing_paths.is_empty() {
                self.runtime_backend = RuntimeBackend::Cef;
                self.runtime_readiness = readiness.clone();
                StartupDiagnostics {
                    component: "engine.readiness",
                    detail: format!(
                        "Live CEF runtime unavailable; missing required paths: {}",
                        readiness.missing_paths.join(", ")
                    ),
                }
            } else {
                let mut bootstrap = CefRuntimeBootstrap::from_config(&self.config);
                if let Some(exit_code) = bootstrap.execute_subprocess_if_needed() {
                    // We are a CEF subprocess (renderer, utility, GPU, etc.).
                    // cef::execute_process() has already run all subprocess logic
                    // and returned the exit code. We MUST exit immediately here —
                    // if we fall through, the process continues and creates a full
                    // eframe GUI window, which is the root cause of the "additional
                    // windows" appearing and crashing after a renderer respawn.
                    std::process::exit(exit_code);
                } else if bootstrap.initialize() {
                    self.runtime_backend = RuntimeBackend::Cef;
                    self.runtime_readiness = RuntimeReadiness {
                        state: RuntimeReadinessState::LiveReady,
                        live_mvp_ready: true,
                        simulated: false,
                        summary: "Real CEF runtime initialized for live MVP mode".to_string(),
                        missing_paths: Vec::new(),
                        checked_paths: readiness.checked_paths,
                    };
                    StartupDiagnostics {
                        component: "engine.bootstrap",
                        detail: format!(
                            "Initialized real CEF runtime with subprocess '{}' and resources '{}'",
                            self.launch_settings.subprocess_path,
                            self.launch_settings.resources_dir
                        ),
                    }
                } else {
                    self.runtime_backend = RuntimeBackend::Cef;
                    self.runtime_readiness = RuntimeReadiness {
                        state: RuntimeReadinessState::InitializationFailed,
                        live_mvp_ready: false,
                        simulated: false,
                        summary: "CEF initialization failed; live browser mode unavailable"
                            .to_string(),
                        missing_paths: Vec::new(),
                        checked_paths: readiness.checked_paths,
                    };
                    StartupDiagnostics {
                        component: "engine.bootstrap",
                        detail: "CEF initialization failed; live browser mode unavailable"
                            .to_string(),
                    }
                }
            }
        } else {
            self.runtime_backend = RuntimeBackend::Simulated;
            self.runtime_readiness = RuntimeReadiness {
                state: RuntimeReadinessState::Simulated,
                live_mvp_ready: false,
                simulated: true,
                summary: "Explicit simulated runtime mode; not live-MVP-ready".to_string(),
                missing_paths: Vec::new(),
                checked_paths: Vec::new(),
            };
            StartupDiagnostics {
                component: "engine.bootstrap",
                detail: format!(
                    "Initialized explicit simulated runtime bootstrap with subprocess '{}' on port {}; not live-MVP-ready",
                    self.launch_settings.subprocess_path,
                    self.launch_settings.remote_debugging_port
                ),
            }
        };
        self.state = EngineState::Started;
        self.diagnostics.push(diagnostic.clone());
        diagnostic
    }

    /// Drive the CEF message loop for one iteration.
    ///
    /// Must be called on every host UI frame when `external_message_pump = 1`.
    /// This gives CEF the opportunity to process network requests, IPC messages,
    /// JavaScript timers, and rendering callbacks (`on_paint`).
    /// Has no effect when the engine is not in `LiveReady` state.
    pub fn tick(&mut self) {
        if matches!(
            self.runtime_readiness.state,
            RuntimeReadinessState::LiveReady
        ) {
            // Pump the CEF message loop once per frame. CEF is not designed for
            // rapid re-entrant pumping — calling do_message_loop_work() multiple
            // times per frame causes crashes during WebGL context creation and
            // rapid resize because in-flight SwiftShader / compositor operations
            // are interrupted mid-pipeline. Resize lag is handled separately by
            // pumping once immediately after was_resized() in resize_browser_surface.
            cef::do_message_loop_work();
        }
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
        if matches!(
            self.launch_settings.runtime_mode,
            BrowserRuntimeMode::RealCef
        ) && !self.runtime_readiness.live_mvp_ready
        {
            return Err(EngineError {
                message: format!(
                    "Live CEF browser instance cannot be created: {}",
                    self.runtime_readiness.summary
                ),
            });
        }

        let browser_id = format!("browser-instance-{}", self.next_browser_instance);
        let surface_id = format!("browser-surface-{}", self.next_browser_instance);
        self.next_browser_instance += 1;

        let live_instance = if matches!(self.runtime_backend, RuntimeBackend::Cef) {
            Some(self.create_live_cef_browser_instance(&browser_id, initial_url)?)
        } else {
            None
        };

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
            render_evidence: live_instance.as_ref().map(|_| {
                format!("CEF browser host created for {initial_url}; awaiting first paint")
            }),
            frame_buffer: None,
            damage_events: 0,
            host_surface_failure: None,
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
                can_go_back: false,
                can_go_forward: false,
            },
        );

        if let Some(live_instance) = live_instance {
            self.live_browser_instances
                .insert(browser_id.clone(), live_instance);
        }

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
        let is_live = self.live_browser_instances.contains_key(browser_id);
        if let Some(live) = self.live_browser_instances.get(browser_id)
            && let Some(frame) = live.browser.main_frame()
        {
            let cef_url = CefString::from(url);
            frame.load_url(Some(&cef_url));
        }
        let backend = {
            let instance = self.browser_instance_mut(browser_id)?;
            instance.url = url.to_string();
            instance.title = "Loading...".to_string();
            instance.is_loading = true;
            instance.failure_state = None;
            instance.history.truncate(instance.history_index + 1);
            instance.history.push(url.to_string());
            instance.history_index = instance.history.len() - 1;
            instance.can_go_back = instance.history_index > 0;
            instance.can_go_forward = false;
            instance.status_text = format!("Navigating to {url}");
            instance.surface.frame_token += 1;
            instance.surface.last_frame_label = format!("Loading {url}");
            instance.surface.render_evidence = match instance.backend {
                RuntimeBackend::Cef => Some(format!(
                    "CEF navigation requested for {url}; awaiting renderer callback"
                )),
                RuntimeBackend::Simulated => None,
            };
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
        if !is_live {
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
        }
        Ok(())
    }

    pub fn dispatch_surface_input(
        &mut self,
        browser_id: &str,
        event: HostSurfaceInputEvent,
    ) -> Result<(), EngineError> {
        match event.clone() {
            HostSurfaceInputEvent::Resize { width, height } => {
                self.resize_browser_surface(browser_id, width, height)?;
                self.push_event(
                    browser_id,
                    BrowserInstanceEventKind::SurfaceUpdated,
                    format!("Surface resize input routed for '{browser_id}'"),
                );
            }
            HostSurfaceInputEvent::Focus { focused } => {
                self.set_surface_focus(browser_id, focused)?;
                self.push_event(
                    browser_id,
                    BrowserInstanceEventKind::SurfaceUpdated,
                    format!("Surface focus input routed for '{browser_id}'"),
                );
            }
            other => {
                if let Some(live) = self.live_browser_instances.get(browser_id)
                    && let Some(host) = live.browser.host()
                {
                    match other {
                        HostSurfaceInputEvent::PointerMove { x, y } => {
                            host.send_mouse_move_event(Some(&MouseEvent { x, y, modifiers: 0 }), 0);
                        }
                        HostSurfaceInputEvent::PointerButton {
                            x,
                            y,
                            button,
                            pressed,
                            click_count,
                        } => {
                            host.send_mouse_click_event(
                                Some(&MouseEvent { x, y, modifiers: 0 }),
                                match button {
                                    HostMouseButton::Left => MouseButtonType::LEFT,
                                    HostMouseButton::Middle => MouseButtonType::MIDDLE,
                                    HostMouseButton::Right => MouseButtonType::RIGHT,
                                },
                                (!pressed) as i32,
                                click_count,
                            );
                        }
                        HostSurfaceInputEvent::Wheel {
                            x,
                            y,
                            delta_x,
                            delta_y,
                        } => {
                            host.send_mouse_wheel_event(
                                Some(&MouseEvent { x, y, modifiers: 0 }),
                                delta_x,
                                delta_y,
                            );
                        }
                        HostSurfaceInputEvent::Key { key_code, pressed } => {
                            host.send_key_event(Some(&KeyEvent {
                                type_: if pressed {
                                    KeyEventType::KEYDOWN
                                } else {
                                    KeyEventType::KEYUP
                                },
                                windows_key_code: key_code,
                                native_key_code: key_code,
                                ..KeyEvent::default()
                            }));
                        }
                        HostSurfaceInputEvent::Text { text } => {
                            for character in text.encode_utf16() {
                                host.send_key_event(Some(&KeyEvent {
                                    type_: KeyEventType::CHAR,
                                    character,
                                    unmodified_character: character,
                                    windows_key_code: i32::from(character),
                                    native_key_code: i32::from(character),
                                    ..KeyEvent::default()
                                }));
                            }
                        }
                        HostSurfaceInputEvent::Focus { .. }
                        | HostSurfaceInputEvent::Resize { .. } => {}
                    }
                }
                if let Ok(instance) = self.browser_instance_mut(browser_id) {
                    instance.status_text = format!("Forwarded host surface input: {:?}", event);
                }
                self.push_event(
                    browser_id,
                    BrowserInstanceEventKind::SurfaceUpdated,
                    format!("{} routed for '{browser_id}'", input_event_label(&event)),
                );
            }
        }
        self.capture_live_callbacks();
        Ok(())
    }

    pub fn finish_navigation(&mut self, browser_id: &str, title: &str) -> Result<(), EngineError> {
        if self.live_browser_instances.contains_key(browser_id) {
            return Err(EngineError {
                message: format!(
                    "Live browser instance '{browser_id}' must finish navigation from CEF callbacks"
                ),
            });
        }
        let instance = self.browser_instance_mut(browser_id)?;
        instance.title = title.to_string();
        instance.is_loading = false;
        instance.failure_state = None;
        instance.status_text = format!("Loaded {}", instance.url);
        instance.surface.frame_token += 1;
        instance.surface.last_frame_label = format!("{title} ({})", instance.url);
        instance.surface.render_evidence = Some(format!(
            "Engine observed rendered state for '{}' at frame {}",
            instance.url, instance.surface.frame_token
        ));
        instance.surface.damage_events += 1;
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
        if self.live_browser_instances.contains_key(browser_id) {
            return Err(EngineError {
                message: format!(
                    "Live browser instance '{browser_id}' must report navigation failure from CEF callbacks or engine diagnostics"
                ),
            });
        }
        let instance = self.browser_instance_mut(browser_id)?;
        instance.is_loading = false;
        instance.failure_state = Some(message.to_string());
        instance.status_text = format!("Navigation failed: {message}");
        instance.surface.frame_token += 1;
        instance.surface.last_frame_label = format!("Navigation failed: {message}");
        instance.surface.host_surface_failure = Some(message.to_string());
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
        if let Some(live) = self.live_browser_instances.get(browser_id)
            && let Some(host) = live.browser.host()
        {
            host.was_resized();
        }
        let instance = self.browser_instance_mut(browser_id)?;
        instance.is_loading = false;
        instance.failure_state = Some(message.to_string());
        instance.status_text = format!("Browser instance crashed: {message}");
        instance.surface.frame_token += 1;
        instance.surface.last_frame_label = format!("Crash: {message}");
        instance.surface.host_surface_failure = Some(message.to_string());
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
        if let Some(live) = self.live_browser_instances.get(browser_id) {
            live.browser.go_back();
            if let Ok(instance) = self.browser_instance_mut(browser_id) {
                instance.is_loading = true;
                instance.status_text = "CEF back navigation requested".to_string();
            }
            self.push_event(
                browser_id,
                BrowserInstanceEventKind::LoadStarted,
                format!("Back navigation requested for live browser instance '{browser_id}'"),
            );
            return Ok(());
        }
        let instance = self.browser_instance_mut(browser_id)?;
        if instance.history_index > 0 {
            instance.history_index -= 1;
            instance.url = instance.history[instance.history_index].clone();
            instance.is_loading = false;
            instance.failure_state = None;
            instance.title = instance.url.clone();
            instance.can_go_back = instance.history_index > 0;
            instance.can_go_forward = instance.history_index + 1 < instance.history.len();
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

    pub fn reload_browser_instance(&mut self, browser_id: &str) -> Result<(), EngineError> {
        if let Some(live) = self.live_browser_instances.get(browser_id) {
            live.browser.reload();
            if let Ok(instance) = self.browser_instance_mut(browser_id) {
                instance.is_loading = true;
                instance.failure_state = None;
                instance.status_text = "CEF reload requested".to_string();
            }
            self.push_event(
                browser_id,
                BrowserInstanceEventKind::LoadStarted,
                format!("Reload requested for live browser instance '{browser_id}'"),
            );
            return Ok(());
        }

        let current_url = self
            .browser_instance(browser_id)
            .map(|instance| instance.url.clone())
            .ok_or_else(|| EngineError {
                message: format!("Browser instance '{}' was not found", browser_id),
            })?;
        self.navigate_browser_instance(browser_id, &current_url)
    }

    pub fn go_forward_browser_instance(&mut self, browser_id: &str) -> Result<(), EngineError> {
        if let Some(live) = self.live_browser_instances.get(browser_id) {
            live.browser.go_forward();
            if let Ok(instance) = self.browser_instance_mut(browser_id) {
                instance.is_loading = true;
                instance.status_text = "CEF forward navigation requested".to_string();
            }
            self.push_event(
                browser_id,
                BrowserInstanceEventKind::LoadStarted,
                format!("Forward navigation requested for live browser instance '{browser_id}'"),
            );
            return Ok(());
        }
        let instance = self.browser_instance_mut(browser_id)?;
        if instance.history_index + 1 < instance.history.len() {
            instance.history_index += 1;
            instance.url = instance.history[instance.history_index].clone();
            instance.is_loading = false;
            instance.failure_state = None;
            instance.title = instance.url.clone();
            instance.can_go_back = instance.history_index > 0;
            instance.can_go_forward = instance.history_index + 1 < instance.history.len();
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
        if let Some(live) = self.live_browser_instances.get(browser_id) {
            let w = width as i32;
            let h = height as i32;
            // Only notify CEF of a resize when the dimensions actually changed.
            // Calling was_resized() every frame (even at the same size) spams the
            // CEF compositor and triggers repeated GPU context re-init in WSL.
            if live.view_width.load(Ordering::Relaxed) != w
                || live.view_height.load(Ordering::Relaxed) != h
            {
                live.view_width.store(w, Ordering::Relaxed);
                live.view_height.store(h, Ordering::Relaxed);
                if let Some(host) = live.browser.host() {
                    host.was_resized();
                    // Pump once immediately so CEF can process the resize and
                    // schedule the repaint within the current frame rather than
                    // waiting until the next tick(). A single targeted pump here
                    // replaces the previous 4x-per-frame approach, which caused
                    // crashes during concurrent WebGL / SwiftShader operations.
                    // Do NOT call host.invalidate() here — was_resized() already
                    // queues a full repaint; a second invalidate() races with it
                    // and corrupts the compositor pipeline under load.
                    if matches!(
                        self.runtime_readiness.state,
                        RuntimeReadinessState::LiveReady
                    ) {
                        cef::do_message_loop_work();
                    }
                }
            }
        }
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
        if let Some(live) = self.live_browser_instances.get(browser_id)
            && let Some(host) = live.browser.host()
        {
            host.set_focus(focused as i32);
        }
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
        if let Some(live) = self.live_browser_instances.remove(browser_id)
            && let Some(host) = live.browser.host()
        {
            host.close_browser(1);
        }
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
    pub fn live_browser_instance_count(&self) -> usize {
        self.live_browser_instances.len()
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

    #[must_use]
    pub fn runtime_readiness(&self) -> &RuntimeReadiness {
        &self.runtime_readiness
    }

    #[must_use]
    pub fn live_mvp_ready(&self) -> bool {
        self.runtime_readiness.live_mvp_ready
    }

    pub fn drain_events(&mut self) -> Vec<BrowserInstanceEvent> {
        self.capture_live_callbacks();
        self.pending_events.drain(..).collect()
    }

    fn capture_live_callbacks(&mut self) {
        let browser_ids = self
            .live_browser_instances
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for browser_id in browser_ids {
            let Some(live) = self.live_browser_instances.get(&browser_id) else {
                continue;
            };
            let callbacks = live.event_sink.drain();
            for callback in callbacks {
                self.apply_live_callback(callback);
            }
        }
    }

    fn apply_live_callback(&mut self, callback: LiveBrowserCallbackEvent) {
        let browser_id = callback.browser_id;
        match callback.kind {
            LiveBrowserCallbackKind::AfterCreated { cef_identifier } => {
                if let Ok(instance) = self.browser_instance_mut(&browser_id) {
                    instance.status_text =
                        format!("CEF browser instance created with native id {cef_identifier}");
                }
                self.push_event(
                    &browser_id,
                    BrowserInstanceEventKind::Created,
                    format!("CEF browser host created for '{browser_id}'"),
                );
            }
            LiveBrowserCallbackKind::BeforeClose => {
                self.push_event(
                    &browser_id,
                    BrowserInstanceEventKind::Closed,
                    format!("CEF browser host closing for '{browser_id}'"),
                );
            }
            LiveBrowserCallbackKind::TitleChanged { title } => {
                if let Ok(instance) = self.browser_instance_mut(&browser_id) {
                    instance.title = title.clone();
                    instance.status_text = format!("Title updated by CEF: {title}");
                }
                self.push_event(
                    &browser_id,
                    BrowserInstanceEventKind::TitleChanged,
                    format!("CEF title changed for '{browser_id}' to '{title}'"),
                );
            }
            LiveBrowserCallbackKind::AddressChanged { url } => {
                if let Ok(instance) = self.browser_instance_mut(&browser_id) {
                    instance.url = url.clone();
                    if instance.history.last() != Some(&url) {
                        instance.history.truncate(instance.history_index + 1);
                        instance.history.push(url.clone());
                        instance.history_index = instance.history.len() - 1;
                    }
                    instance.can_go_back = instance.history_index > 0;
                    instance.can_go_forward = instance.history_index + 1 < instance.history.len();
                    instance.status_text = format!("Address updated by CEF: {url}");
                }
                self.push_event(
                    &browser_id,
                    BrowserInstanceEventKind::HistoryChanged,
                    format!("CEF address changed for '{browser_id}' to '{url}'"),
                );
            }
            LiveBrowserCallbackKind::LoadingStateChanged {
                is_loading,
                can_go_back,
                can_go_forward,
            } => {
                if let Ok(instance) = self.browser_instance_mut(&browser_id) {
                    instance.is_loading = is_loading;
                    instance.can_go_back = can_go_back;
                    instance.can_go_forward = can_go_forward;
                    instance.status_text = format!(
                        "CEF loading state changed: loading={is_loading}, back={can_go_back}, forward={can_go_forward}"
                    );
                }
                self.push_event(
                    &browser_id,
                    BrowserInstanceEventKind::HistoryChanged,
                    format!("CEF loading state changed for '{browser_id}'"),
                );
            }
            LiveBrowserCallbackKind::LoadStarted => {
                if let Ok(instance) = self.browser_instance_mut(&browser_id) {
                    instance.is_loading = true;
                    instance.status_text = "CEF load started".to_string();
                }
                self.push_event(
                    &browser_id,
                    BrowserInstanceEventKind::LoadStarted,
                    format!("CEF load started for '{browser_id}'"),
                );
            }
            LiveBrowserCallbackKind::LoadFinished { http_status_code } => {
                if let Ok(instance) = self.browser_instance_mut(&browser_id) {
                    instance.is_loading = false;
                    instance.failure_state = None;
                    instance.status_text =
                        format!("CEF load finished with HTTP {http_status_code}");
                    instance.surface.render_evidence = Some(format!(
                        "CEF load finished for {} with HTTP {http_status_code}",
                        instance.url
                    ));
                }
                self.push_event(
                    &browser_id,
                    BrowserInstanceEventKind::LoadFinished,
                    format!("CEF load finished for '{browser_id}'"),
                );
            }
            LiveBrowserCallbackKind::LoadError { code, text, url } => {
                let message = format!("CEF load error {code} for {url}: {text}");
                if let Ok(instance) = self.browser_instance_mut(&browser_id) {
                    instance.is_loading = false;
                    instance.failure_state = Some(message.clone());
                    instance.status_text = message.clone();
                    instance.surface.host_surface_failure = Some(message.clone());
                }
                self.push_event(
                    &browser_id,
                    BrowserInstanceEventKind::NavigationFailed,
                    message,
                );
            }
            LiveBrowserCallbackKind::Painted {
                width,
                height,
                dirty_rects,
                buffer,
            } => {
                if let Ok(instance) = self.browser_instance_mut(&browser_id) {
                    instance.surface.width = width.max(0) as u32;
                    instance.surface.height = height.max(0) as u32;
                    instance.surface.frame_token += 1;
                    instance.surface.damage_events += dirty_rects as u64;
                    instance.surface.render_evidence = Some(format!(
                        "CEF painted frame {}x{} with {} dirty rect(s)",
                        width, height, dirty_rects
                    ));
                    instance.surface.last_frame_label = format!(
                        "Live CEF frame {} ({}x{})",
                        instance.surface.frame_token, width, height
                    );
                    instance.surface.frame_buffer = buffer.map(|bgra| BrowserFrameBuffer {
                        width: width.max(0) as u32,
                        height: height.max(0) as u32,
                        bgra,
                    });
                }
                self.push_event(
                    &browser_id,
                    BrowserInstanceEventKind::SurfaceUpdated,
                    format!("CEF painted surface for '{browser_id}'"),
                );
            }
        }
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

    fn check_live_runtime_readiness(&self) -> RuntimeReadiness {
        let mut missing_paths = Vec::new();
        let mut checked_paths = Vec::new();

        // Check distribution root and locales directory exist.
        for (label, path) in [
            (
                "CEF distribution root",
                self.launch_settings.distribution_root.as_str(),
            ),
            ("CEF locales", self.launch_settings.locales_dir.as_str()),
        ] {
            checked_paths.push(format!("{label}: {path}"));
            if !Path::new(path).exists() {
                missing_paths.push(format!("{label} ({path})"));
            }
        }

        // Validate resources directory by checking for icudtl.dat — CEF's
        // required Unicode data file. On Linux the .pak files live flat
        // alongside libcef.so, so this confirms the correct directory is
        // configured (not an empty resources/ subdirectory).
        let icudtl = Path::new(self.launch_settings.resources_dir.as_str()).join("icudtl.dat");
        checked_paths.push(format!("CEF resources (icudtl.dat): {}", icudtl.display()));
        if !icudtl.exists() {
            missing_paths.push(format!(
                "CEF resources ({}) — icudtl.dat not found; resources_dir may be wrong",
                icudtl.display()
            ));
        }

        // Validate subprocess by checking the current executable is accessible.
        // We use self-launch (current exe = subprocess), so it must be readable.
        let subprocess = self.launch_settings.subprocess_path.as_str();
        checked_paths.push(format!("CEF subprocess: {subprocess}"));
        if !Path::new(subprocess).exists() {
            missing_paths.push(format!("CEF subprocess ({subprocess})"));
        }

        for runtime_dir in [
            self.launch_settings.data_dir.as_str(),
            self.launch_settings.cache_dir.as_str(),
            self.launch_settings.log_dir.as_str(),
        ] {
            checked_paths.push(format!("runtime dir: {runtime_dir}"));
            if let Err(error) = fs::create_dir_all(runtime_dir) {
                missing_paths.push(format!("runtime dir {runtime_dir}: {error}"));
            }
        }

        RuntimeReadiness {
            state: if missing_paths.is_empty() {
                RuntimeReadinessState::LiveReady
            } else {
                RuntimeReadinessState::LiveUnavailable
            },
            live_mvp_ready: missing_paths.is_empty(),
            simulated: false,
            summary: if missing_paths.is_empty() {
                format!(
                    "Live CEF prerequisites present; subprocess='{}', resources='{}', locales='{}', remote_debugging_port={}",
                    self.launch_settings.subprocess_path,
                    self.launch_settings.resources_dir,
                    self.launch_settings.locales_dir,
                    self.launch_settings.remote_debugging_port
                )
            } else {
                format!(
                    "Live CEF prerequisites missing: {}",
                    missing_paths.join(", ")
                )
            },
            missing_paths,
            checked_paths,
        }
    }

    fn create_live_cef_browser_instance(
        &self,
        browser_id: &str,
        initial_url: &str,
    ) -> Result<LiveCefBrowserInstance, EngineError> {
        let window_info = WindowInfo {
            bounds: Rect {
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            },
            windowless_rendering_enabled: 1,
            runtime_style: RuntimeStyle::ALLOY,
            ..WindowInfo::default()
        };
        let event_sink = LiveBrowserEventSink::default();
        let life_span_handler =
            WeboxLifeSpanHandler::new(browser_id.to_string(), event_sink.clone());
        let display_handler = WeboxDisplayHandler::new(browser_id.to_string(), event_sink.clone());
        let load_handler = WeboxLoadHandler::new(browser_id.to_string(), event_sink.clone());
        let view_width = Arc::new(AtomicI32::new(1280));
        let view_height = Arc::new(AtomicI32::new(720));
        let render_handler = WeboxRenderHandler::new(
            browser_id.to_string(),
            event_sink.clone(),
            view_width.clone(),
            view_height.clone(),
        );
        let context_menu_handler = WeboxContextMenuHandler::new();
        let mut client = WeboxCefClient::new(
            browser_id.to_string(),
            life_span_handler,
            display_handler,
            load_handler,
            render_handler,
            context_menu_handler,
        );
        let url = CefString::from(initial_url);
        let browser = cef::browser_host_create_browser_sync(
            Some(&window_info),
            Some(&mut client),
            Some(&url),
            Some(&BrowserSettings::default()),
            None,
            None,
        )
        .ok_or_else(|| EngineError {
            message: format!("CEF failed to create live browser instance '{browser_id}'"),
        })?;

        Ok(LiveCefBrowserInstance {
            browser,
            _client: client,
            event_sink,
            view_width,
            view_height,
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

fn input_event_label(event: &HostSurfaceInputEvent) -> &'static str {
    match event {
        HostSurfaceInputEvent::PointerMove { .. } => "pointer move input",
        HostSurfaceInputEvent::PointerButton { .. } => "pointer button input",
        HostSurfaceInputEvent::Wheel { .. } => "wheel input",
        HostSurfaceInputEvent::Key { .. } => "key input",
        HostSurfaceInputEvent::Text { .. } => "text input",
        HostSurfaceInputEvent::Focus { .. } => "focus input",
        HostSurfaceInputEvent::Resize { .. } => "resize input",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BrowserInstanceEventKind, BrowserSurfaceRenderMode, EngineState, RuntimeBackend,
        RuntimeReadinessState, WeboxEngine,
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
    fn engine_event_mapping_covers_navigation_surface_and_memory() {
        let mut engine = WeboxEngine::new(&AppConfig::simulated());
        let _ = engine.start();
        let instance = engine
            .create_browser_instance("https://example.com")
            .unwrap();

        engine
            .navigate_browser_instance(&instance.id, "https://example.com/ready")
            .unwrap();
        engine.finish_navigation(&instance.id, "Ready").unwrap();
        engine
            .resize_browser_surface(&instance.id, 1024, 768)
            .unwrap();
        engine
            .update_browser_memory(
                &instance.id,
                42,
                Some("memory warning".to_string()),
                None,
                Some(
                    "source=SyntheticSample; confidence=TestOnly; live_mvp_evidence=false"
                        .to_string(),
                ),
            )
            .unwrap();

        let events = engine.drain_events();
        assert!(
            events
                .iter()
                .any(|event| event.kind == BrowserInstanceEventKind::LoadStarted)
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
        assert!(
            events
                .iter()
                .any(|event| event.kind == BrowserInstanceEventKind::MemoryUpdated)
        );
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

    #[test]
    fn live_mode_reports_missing_cef_assets_without_simulated_readiness() {
        let mut engine = WeboxEngine::new(&AppConfig::development());
        let diagnostic = engine.start();

        assert_eq!(diagnostic.component, "engine.readiness");
        assert_eq!(engine.runtime_backend(), RuntimeBackend::Cef);
        assert_eq!(
            engine.runtime_readiness().state,
            RuntimeReadinessState::LiveUnavailable
        );
        assert!(!engine.live_mvp_ready());
        assert!(!engine.runtime_readiness().missing_paths.is_empty());
        assert!(
            engine
                .create_browser_instance("https://example.com")
                .is_err()
        );
    }

    #[test]
    fn simulated_mode_is_explicitly_not_live_mvp_ready() {
        let mut engine = WeboxEngine::new(&AppConfig::simulated());
        let diagnostic = engine.start();

        assert_eq!(engine.runtime_backend(), RuntimeBackend::Simulated);
        assert_eq!(
            engine.runtime_readiness().state,
            RuntimeReadinessState::Simulated
        );
        assert!(engine.runtime_readiness().simulated);
        assert!(!engine.live_mvp_ready());
        assert!(diagnostic.detail.contains("not live-MVP-ready"));
        assert!(
            engine
                .create_browser_instance("https://example.com")
                .is_ok()
        );
    }
}
