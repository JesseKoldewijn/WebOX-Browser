use webox_config::AppConfig;
use webox_memory::TabTelemetry;
use webox_shell::HostShell;
use webox_ui::BrowserCommand;

#[test]
fn browser_smoke_flow_covers_startup_navigation_and_shutdown() {
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
    shell.finish_navigation(&window, &tab, "Example").unwrap();
    shell.shutdown();

    assert_eq!(shell.windows()[&window].tabs[0].title, "Example");
    assert_eq!(shell.startup_diagnostics().len(), 2);
}

#[test]
fn browser_smoke_flow_surfaces_memory_pressure_in_visible_state() {
    let mut shell = HostShell::new(AppConfig::simulated());
    shell.start();

    let window = shell.create_window("window-1");
    let tab = shell.open_tab(&window, "https://webox.dev").unwrap();
    shell.finish_navigation(&window, &tab, "webox").unwrap();
    shell
        .record_tab_telemetry(
            &window,
            &TabTelemetry {
                tab_id: tab.clone(),
                renderer_bytes: 8 * 1024 * 1024 * 1024,
                browser_bytes: 256 * 1024 * 1024,
                gpu_bytes: 128 * 1024 * 1024,
            },
        )
        .unwrap();

    let tab_state = shell.windows()[&window]
        .tabs
        .iter()
        .find(|candidate| candidate.id == tab)
        .unwrap();
    assert_eq!(
        tab_state.memory_indicator.as_deref(),
        Some("memory exhaustion risk")
    );
    assert_eq!(
        tab_state.failure_state.as_deref(),
        Some("Tab ended due to suspected memory exhaustion")
    );
}
