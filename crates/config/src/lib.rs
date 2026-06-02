use std::{env, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsupportedHostError {
    os: String,
    arch: String,
}

impl UnsupportedHostError {
    fn new(os: &str, arch: &str) -> Self {
        Self {
            os: os.to_string(),
            arch: arch.to_string(),
        }
    }
}

impl std::fmt::Display for UnsupportedHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unsupported host platform: os='{}', arch='{}'",
            self.os, self.arch
        )
    }
}

impl std::error::Error for UnsupportedHostError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserRuntimeMode {
    Simulated,
    RealCef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserUiHost {
    Eframe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformTarget {
    LinuxX64,
    LinuxArm64,
    MacosArm64,
    WindowsX64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentPaths {
    pub workspace_root: String,
    pub data_dir: String,
    pub cache_dir: String,
    pub log_dir: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CefRuntimePaths {
    pub distribution_root: String,
    pub framework_dir: String,
    pub resources_dir: String,
    pub locales_dir: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubprocessLaunchOptions {
    pub browser_subprocess_path: String,
    pub extra_args: Vec<String>,
    pub log_file_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserStartupConfig {
    pub home_page: String,
    pub max_memory_per_tab_bytes: u64,
    pub environment: String,
    pub runtime_mode: BrowserRuntimeMode,
    pub ui_host: BrowserUiHost,
    pub platform_target: PlatformTarget,
    pub enable_cef_sandbox: bool,
    pub remote_debugging_port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppConfig {
    pub startup: BrowserStartupConfig,
    pub cef: CefRuntimePaths,
    pub subprocess: SubprocessLaunchOptions,
    pub paths: EnvironmentPaths,
}

impl AppConfig {
    #[must_use]
    pub fn development() -> Self {
        let platform_target = PlatformTarget::current().unwrap_or_else(|error| panic!("{error}"));
        Self {
            startup: BrowserStartupConfig {
                home_page: "https://example.com".to_string(),
                max_memory_per_tab_bytes: 8 * 1024 * 1024 * 1024,
                environment: "development".to_string(),
                runtime_mode: BrowserRuntimeMode::RealCef,
                ui_host: BrowserUiHost::Eframe,
                platform_target,
                enable_cef_sandbox: false,
                remote_debugging_port: 9222,
            },
            cef: CefRuntimePaths {
                distribution_root: development_cef_root(platform_target),
                framework_dir: development_framework_dir(platform_target),
                // CEF Linux minimal distribution places .pak files flat alongside
                // libcef.so — there is no resources/ subdirectory on Linux.
                resources_dir: development_resources_dir(platform_target),
                locales_dir: development_locales_dir(platform_target),
            },
            subprocess: SubprocessLaunchOptions {
                // Use the current executable as the CEF subprocess (self-launch).
                // CEF re-invokes the binary with internal args; execute_process()
                // detects this and exits before the UI is initialized.
                browser_subprocess_path: std::env::current_exe()
                    .unwrap_or_default()
                    .display()
                    .to_string(),
                extra_args: vec!["--enable-logging".to_string()],
                log_file_path: ".webox/logs/webox-engine.log".to_string(),
            },
            paths: EnvironmentPaths {
                workspace_root: ".".to_string(),
                data_dir: ".webox/data".to_string(),
                cache_dir: ".webox/cache".to_string(),
                log_dir: ".webox/logs".to_string(),
            },
        }
    }

    /// Production config: resolves all paths relative to the directory containing
    /// the running executable. This allows the binary to be placed anywhere on
    /// disk and find its CEF assets via the embedded `$ORIGIN` RPATH without
    /// relying on a fixed CWD.
    ///
    /// Asset layout expected next to the binary (CEF Linux flat layout):
    /// ```text
    /// webox-browser-app
    /// libcef.so
    /// icudtl.dat
    /// resources.pak
    /// chrome_100_percent.pak
    /// chrome_200_percent.pak
    /// v8_context_snapshot.bin
    /// locales/
    ///   en-US.pak
    ///   …
    /// ```
    #[must_use]
    pub fn production() -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let exe_path = std::env::current_exe()
            .unwrap_or_default()
            .display()
            .to_string();
        let platform_target = PlatformTarget::current().unwrap_or_else(|error| panic!("{error}"));
        let paths = production_paths(platform_target, &exe_dir);

        Self {
            startup: BrowserStartupConfig {
                home_page: "https://example.com".to_string(),
                max_memory_per_tab_bytes: 8 * 1024 * 1024 * 1024,
                environment: "production".to_string(),
                runtime_mode: BrowserRuntimeMode::RealCef,
                ui_host: BrowserUiHost::Eframe,
                platform_target,
                enable_cef_sandbox: false,
                remote_debugging_port: 0,
            },
            cef: CefRuntimePaths {
                distribution_root: paths.cef_distribution_root,
                framework_dir: paths.cef_framework_dir,
                resources_dir: paths.cef_resources_dir,
                locales_dir: paths.cef_locales_dir,
            },
            subprocess: SubprocessLaunchOptions {
                // Self-launch: CEF re-invokes this binary as a subprocess.
                // execute_process() detects this and exits before UI init.
                browser_subprocess_path: exe_path,
                extra_args: vec![],
                log_file_path: paths.log_dir.join("webox-engine.log").display().to_string(),
            },
            paths: EnvironmentPaths {
                workspace_root: paths.workspace_root,
                data_dir: paths.data_dir.display().to_string(),
                cache_dir: paths.cache_dir.display().to_string(),
                log_dir: paths.log_dir.display().to_string(),
            },
        }
    }

    #[must_use]
    pub fn simulated() -> Self {
        let mut config = Self::development();
        config.startup.environment = "test".to_string();
        config.startup.runtime_mode = BrowserRuntimeMode::Simulated;
        config
    }
}

impl PlatformTarget {
    pub fn current() -> Result<Self, UnsupportedHostError> {
        match (env::consts::OS, env::consts::ARCH) {
            ("linux", "x86_64") => Ok(Self::LinuxX64),
            ("linux", "aarch64") => Ok(Self::LinuxArm64),
            ("macos", "aarch64") => Ok(Self::MacosArm64),
            ("windows", "x86_64") => Ok(Self::WindowsX64),
            (os, arch) => Err(UnsupportedHostError::new(os, arch)),
        }
    }
}

struct ProductionPaths {
    workspace_root: String,
    data_dir: PathBuf,
    cache_dir: PathBuf,
    log_dir: PathBuf,
    cef_distribution_root: String,
    cef_framework_dir: String,
    cef_resources_dir: String,
    cef_locales_dir: String,
}

fn development_cef_root(platform_target: PlatformTarget) -> String {
    format!("third_party/cef/{}", platform_target.cef_slug())
}

fn development_resources_dir(platform_target: PlatformTarget) -> String {
    match platform_target {
        PlatformTarget::MacosArm64 => development_cef_root(platform_target),
        _ => development_cef_root(platform_target),
    }
}

fn development_framework_dir(platform_target: PlatformTarget) -> String {
    match platform_target {
        PlatformTarget::MacosArm64 => development_cef_root(platform_target),
        _ => development_cef_root(platform_target),
    }
}

fn development_locales_dir(platform_target: PlatformTarget) -> String {
    format!("{}/locales", development_cef_root(platform_target))
}

fn production_paths(platform_target: PlatformTarget, exe_dir: &std::path::Path) -> ProductionPaths {
    match platform_target {
        PlatformTarget::WindowsX64 => {
            let local_app_data = env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .or_else(home_dir)
                .unwrap_or_else(|| exe_dir.to_path_buf())
                .join("WebOX Browser");

            ProductionPaths {
                workspace_root: exe_dir.display().to_string(),
                data_dir: local_app_data.join("data"),
                cache_dir: local_app_data.join("cache"),
                log_dir: local_app_data.join("logs"),
                cef_distribution_root: exe_dir.display().to_string(),
                cef_framework_dir: exe_dir.display().to_string(),
                cef_resources_dir: exe_dir.display().to_string(),
                cef_locales_dir: exe_dir.join("locales").display().to_string(),
            }
        }
        PlatformTarget::MacosArm64 => {
            let bundle_root = resolve_macos_bundle_root(exe_dir);
            let macos_dir = bundle_root.join("Contents/MacOS");
            let app_support = home_dir()
                .unwrap_or_else(|| exe_dir.to_path_buf())
                .join("Library/Application Support/WebOX Browser");
            let cache_root = home_dir()
                .unwrap_or_else(|| exe_dir.to_path_buf())
                .join("Library/Caches/WebOX Browser");

            ProductionPaths {
                workspace_root: bundle_root.display().to_string(),
                data_dir: app_support.join("data"),
                cache_dir: cache_root.join("cache"),
                log_dir: app_support.join("logs"),
                cef_distribution_root: macos_dir.display().to_string(),
                cef_framework_dir: macos_dir.display().to_string(),
                cef_resources_dir: macos_dir.display().to_string(),
                cef_locales_dir: macos_dir.join("locales").display().to_string(),
            }
        }
        PlatformTarget::LinuxX64 | PlatformTarget::LinuxArm64 => {
            let home = home_dir().unwrap_or_else(|| exe_dir.to_path_buf());
            let data_root = xdg_dir("XDG_DATA_HOME", &home, ".local/share").join("webox-browser");
            let cache_root = xdg_dir("XDG_CACHE_HOME", &home, ".cache").join("webox-browser");

            ProductionPaths {
                workspace_root: exe_dir.display().to_string(),
                data_dir: data_root.join("data"),
                cache_dir: cache_root.join("cache"),
                log_dir: data_root.join("logs"),
                cef_distribution_root: exe_dir.display().to_string(),
                cef_framework_dir: exe_dir.display().to_string(),
                cef_resources_dir: exe_dir.display().to_string(),
                cef_locales_dir: exe_dir.join("locales").display().to_string(),
            }
        }
    }
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        env::var_os("HOME").map(PathBuf::from)
    }
}

fn xdg_dir(var_name: &str, home: &std::path::Path, fallback: &str) -> PathBuf {
    env::var_os(var_name)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty() && path.is_absolute())
        .unwrap_or_else(|| home.join(fallback))
}

fn resolve_macos_bundle_root(exe_dir: &std::path::Path) -> PathBuf {
    exe_dir
        .parent()
        .and_then(|contents| contents.parent())
        .filter(|bundle_root| bundle_root.extension().is_some_and(|ext| ext == "app"))
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| exe_dir.to_path_buf())
}

impl PlatformTarget {
    fn cef_slug(self) -> &'static str {
        match self {
            Self::LinuxX64 => "linux-x64",
            Self::LinuxArm64 => "linux-arm64",
            Self::MacosArm64 => "macos-arm64",
            Self::WindowsX64 => "windows-x64",
        }
    }

    #[must_use]
    pub fn runtime_plan_target(self) -> &'static str {
        match self {
            Self::LinuxX64 => "linux-x86_64",
            Self::LinuxArm64 => "linux-aarch64",
            Self::MacosArm64 => "macos-aarch64",
            Self::WindowsX64 => "windows-x86_64",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppConfig, BrowserRuntimeMode, BrowserUiHost, PlatformTarget, production_paths,
        resolve_macos_bundle_root,
    };
    use std::path::PathBuf;

    #[test]
    fn development_config_targets_eight_gib_tabs() {
        let config = AppConfig::development();
        assert_eq!(
            config.startup.max_memory_per_tab_bytes,
            8 * 1024 * 1024 * 1024
        );
        assert_eq!(config.startup.remote_debugging_port, 9222);
        assert_eq!(config.startup.runtime_mode, BrowserRuntimeMode::RealCef);
        assert_eq!(config.startup.ui_host, BrowserUiHost::Eframe);
        assert_eq!(
            config.startup.platform_target,
            PlatformTarget::current().unwrap()
        );
    }

    #[test]
    fn simulated_config_keeps_same_memory_target() {
        let config = AppConfig::simulated();
        assert_eq!(config.startup.runtime_mode, BrowserRuntimeMode::Simulated);
        assert_eq!(
            config.startup.max_memory_per_tab_bytes,
            8 * 1024 * 1024 * 1024
        );
    }

    #[test]
    fn linux_production_paths_write_outside_install_dir() {
        let exe_dir = PathBuf::from("/opt/webox-browser");
        let paths = production_paths(PlatformTarget::LinuxX64, &exe_dir);
        assert_eq!(paths.cef_distribution_root, exe_dir.display().to_string());
        assert_ne!(paths.data_dir, exe_dir.join(".webox/data"));
        assert_ne!(paths.cache_dir, exe_dir.join(".webox/cache"));
        assert_eq!(
            paths.cef_locales_dir,
            exe_dir.join("locales").display().to_string()
        );
    }

    #[test]
    fn windows_production_paths_use_local_app_data() {
        let exe_dir = PathBuf::from(r"C:\Program Files\WebOX Browser");
        let paths = production_paths(PlatformTarget::WindowsX64, &exe_dir);
        assert!(paths.data_dir.to_string_lossy().contains("WebOX Browser"));
        assert!(!paths.data_dir.starts_with(&exe_dir));
        assert_eq!(paths.cef_resources_dir, exe_dir.display().to_string());
    }

    #[test]
    fn macos_bundle_root_resolution_uses_app_root() {
        let exe_dir = PathBuf::from("/Applications/WeboxBrowser.app/Contents/MacOS");
        assert_eq!(
            resolve_macos_bundle_root(&exe_dir),
            PathBuf::from("/Applications/WeboxBrowser.app")
        );
    }

    #[test]
    fn xdg_dir_rejects_relative_overrides() {
        let home = PathBuf::from("/home/webox");
        assert_eq!(
            super::xdg_dir("TEST_XDG_DIR_MISSING", &home, ".cache"),
            home.join(".cache")
        );
    }
}
