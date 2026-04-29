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
        Self {
            startup: BrowserStartupConfig {
                home_page: "https://example.com".to_string(),
                max_memory_per_tab_bytes: 8 * 1024 * 1024 * 1024,
                environment: "development".to_string(),
                runtime_mode: BrowserRuntimeMode::RealCef,
                ui_host: BrowserUiHost::Eframe,
                platform_target: PlatformTarget::LinuxX64,
                enable_cef_sandbox: false,
                remote_debugging_port: 9222,
            },
            cef: CefRuntimePaths {
                distribution_root: "third_party/cef/linux-x64".to_string(),
                framework_dir: "third_party/cef/linux-x64".to_string(),
                // CEF Linux minimal distribution places .pak files flat alongside
                // libcef.so — there is no resources/ subdirectory on Linux.
                resources_dir: "third_party/cef/linux-x64".to_string(),
                locales_dir: "third_party/cef/linux-x64/locales".to_string(),
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

        let cef_dir = exe_dir.display().to_string();
        let data_base = exe_dir.join(".webox");

        Self {
            startup: BrowserStartupConfig {
                home_page: "https://example.com".to_string(),
                max_memory_per_tab_bytes: 8 * 1024 * 1024 * 1024,
                environment: "production".to_string(),
                runtime_mode: BrowserRuntimeMode::RealCef,
                ui_host: BrowserUiHost::Eframe,
                platform_target: PlatformTarget::LinuxX64,
                enable_cef_sandbox: false,
                remote_debugging_port: 0,
            },
            cef: CefRuntimePaths {
                distribution_root: cef_dir.clone(),
                framework_dir: cef_dir.clone(),
                // CEF Linux flat layout: .pak files sit alongside libcef.so,
                // not in a resources/ subdirectory.
                resources_dir: cef_dir.clone(),
                locales_dir: exe_dir.join("locales").display().to_string(),
            },
            subprocess: SubprocessLaunchOptions {
                // Self-launch: CEF re-invokes this binary as a subprocess.
                // execute_process() detects this and exits before UI init.
                browser_subprocess_path: exe_path,
                extra_args: vec![],
                log_file_path: data_base
                    .join("logs/webox-engine.log")
                    .display()
                    .to_string(),
            },
            paths: EnvironmentPaths {
                workspace_root: cef_dir.clone(),
                data_dir: data_base.join("data").display().to_string(),
                cache_dir: data_base.join("cache").display().to_string(),
                log_dir: data_base.join("logs").display().to_string(),
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

#[cfg(test)]
mod tests {
    use super::{AppConfig, BrowserRuntimeMode, BrowserUiHost, PlatformTarget};

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
        assert_eq!(config.startup.platform_target, PlatformTarget::LinuxX64);
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
}
