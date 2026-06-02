use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use webox_config::{AppConfig, BrowserRuntimeMode};
use webox_engine::{
    BrowserInstanceEvent, BrowserInstanceEventKind, HostMouseButton, HostSurfaceInputEvent,
};
use webox_runtime_api::{EmbeddedRuntime, EmbeddedRuntimeConfig, RuntimeBrowserSnapshot};

#[derive(Clone, Copy)]
struct WorkloadCase {
    id: &'static str,
    category: &'static str,
    title: &'static str,
    fixture_path: &'static str,
    renderer_bytes: u64,
    browser_bytes: u64,
    gpu_bytes: u64,
    compatibility: &'static str,
    proof: WorkloadProof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkloadProof {
    RenderOnly,
    InputInteraction,
    TabState,
    MemoryPressure,
    NavigationFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeOutcome {
    Success,
    CompatibilityFailure,
    EngineStartupFailure,
    HostSurfaceFailure,
    BrowserCrash,
    Timeout,
    ConstrainedMemory,
}

struct WorkloadObservation {
    workload: WorkloadCase,
    outcome: RuntimeOutcome,
    failure_class: &'static str,
    final_url: String,
    title_signal: String,
    render_proof: String,
    input_proof: Option<String>,
    elapsed_ms: u128,
    events: Vec<BrowserInstanceEvent>,
    snapshot: RuntimeBrowserSnapshot,
}

const GIB: u64 = 1024 * 1024 * 1024;
const MIB: u64 = 1024 * 1024;

const WORKLOADS: [WorkloadCase; 7] = [
    WorkloadCase {
        id: "standards-rendering",
        category: "standards-rendering",
        title: "Standards Rendering Fixture",
        fixture_path: "validation/workloads/standards-rendering.html",
        renderer_bytes: 512 * MIB,
        browser_bytes: 128 * MIB,
        gpu_bytes: 64 * MIB,
        compatibility: "loads deterministic HTML, CSS, DOM, media query, and paint proof",
        proof: WorkloadProof::RenderOnly,
    },
    WorkloadCase {
        id: "large-dom",
        category: "large-data-visualization",
        title: "Large DOM Fixture",
        fixture_path: "validation/workloads/large-dom.html",
        renderer_bytes: 2 * GIB,
        browser_bytes: 512 * MIB,
        gpu_bytes: 128 * MIB,
        compatibility: "loads representative large DOM and CSS grid content",
        proof: WorkloadProof::RenderOnly,
    },
    WorkloadCase {
        id: "interaction",
        category: "input-interaction",
        title: "Interaction Fixture",
        fixture_path: "validation/workloads/interaction-fixture.html",
        renderer_bytes: 768 * MIB,
        browser_bytes: 128 * MIB,
        gpu_bytes: 64 * MIB,
        compatibility: "proves routed click, text, scroll, focus, and resize evidence",
        proof: WorkloadProof::InputInteraction,
    },
    WorkloadCase {
        id: "tab-state",
        category: "tab-state-preservation",
        title: "Tab State Fixture",
        fixture_path: "validation/workloads/tab-state-preservation.html",
        renderer_bytes: 512 * MIB,
        browser_bytes: 128 * MIB,
        gpu_bytes: 64 * MIB,
        compatibility: "proves local tab state and browser storage surfaces are active",
        proof: WorkloadProof::TabState,
    },
    WorkloadCase {
        id: "webgl-canvas",
        category: "unity-webgl",
        title: "Canvas Fixture",
        fixture_path: "validation/workloads/webgl-canvas.html",
        renderer_bytes: 3 * GIB,
        browser_bytes: 768 * MIB,
        gpu_bytes: GIB,
        compatibility: "renders canvas and GPU-oriented content representative of Unity WebGL scenes",
        proof: WorkloadProof::RenderOnly,
    },
    WorkloadCase {
        id: "worker-memory",
        category: "wasm-heavy-tool",
        title: "Worker Memory Fixture",
        fixture_path: "validation/workloads/worker-memory.html",
        renderer_bytes: 6 * GIB,
        browser_bytes: GIB,
        gpu_bytes: 256 * MIB,
        compatibility: "exercises worker and memory allocation patterns found in WASM-heavy tools",
        proof: WorkloadProof::MemoryPressure,
    },
    WorkloadCase {
        id: "navigation-failure",
        category: "navigation-failure",
        title: "Intentional Navigation Failure",
        fixture_path: "http://127.0.0.1:9/webox-intentional-failure",
        renderer_bytes: 256 * MIB,
        browser_bytes: 128 * MIB,
        gpu_bytes: 32 * MIB,
        compatibility: "proves observed navigation failure classification",
        proof: WorkloadProof::NavigationFailure,
    },
];

#[derive(Clone)]
struct HarnessConfig {
    available_memory_bytes: u64,
    output_path: &'static str,
    runtime_mode: BrowserRuntimeMode,
}

fn main() {
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "supported".to_string());
    let config = match target.as_str() {
        "supported" => HarnessConfig {
            available_memory_bytes: 16 * GIB,
            output_path: ".webox/validation/harness-supported.md",
            runtime_mode: BrowserRuntimeMode::RealCef,
        },
        "constrained" => HarnessConfig {
            available_memory_bytes: 6 * GIB,
            output_path: ".webox/validation/harness-constrained.md",
            runtime_mode: BrowserRuntimeMode::RealCef,
        },
        other => {
            eprintln!("Unknown harness target '{other}'. Use 'supported' or 'constrained'.");
            std::process::exit(1);
        }
    };

    let report = run_harness(&config);
    let output_path = Path::new(config.output_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).expect("should create harness output directory");
    }
    fs::write(output_path, report).expect("should write harness report");
    println!("Wrote workload harness report to {}", output_path.display());
}

pub(crate) fn run_harness(config: &HarnessConfig) -> String {
    let mut app_config = AppConfig::development();
    app_config.startup.runtime_mode = config.runtime_mode;
    let mut runtime = EmbeddedRuntime::new(EmbeddedRuntimeConfig {
        app_config: app_config.clone(),
        available_memory_bytes: config.available_memory_bytes,
    });
    let system_report = runtime.system_report(config.available_memory_bytes);
    let readiness = runtime.runtime_readiness().clone();

    let mut lines = vec![
        "# webox workload harness report".to_string(),
        String::new(),
        format!(
            "- Available memory bytes: {}",
            config.available_memory_bytes
        ),
        format!(
            "- Configured tab target bytes: {}",
            app_config.startup.max_memory_per_tab_bytes
        ),
        format!("- Meets target: {}", system_report.meets_target),
        format!("- Summary: {}", system_report.summary),
        format!("- Runtime mode: {:?}", config.runtime_mode),
        format!("- Live MVP ready: {}", runtime.live_mvp_ready()),
        format!("- Runtime readiness: {:?}", readiness.state),
        format!("- Runtime readiness summary: {}", readiness.summary),
        format!(
            "- Missing runtime assets: {}",
            if readiness.missing_paths.is_empty() {
                "none".to_string()
            } else {
                readiness.missing_paths.join(", ")
            }
        ),
        format!(
            "- Runtime directory errors: {}",
            if readiness.readiness_errors.is_empty() {
                "none".to_string()
            } else {
                readiness.readiness_errors.join(", ")
            }
        ),
        String::new(),
        "## Workloads".to_string(),
        String::new(),
    ];

    if !runtime.live_mvp_ready() {
        lines.push(
            "Live MVP validation did not run workloads because the runtime is not live-MVP-ready."
                .to_string(),
        );
        lines.push(
            "This is a validation failure for live MVP mode, not a synthetic success.".to_string(),
        );
        lines.push(format!(
            "- Outcome: {:?}",
            RuntimeOutcome::EngineStartupFailure
        ));
        lines.push(format!(
            "- Failure class: {}",
            failure_class(RuntimeOutcome::EngineStartupFailure)
        ));
        if matches!(config.runtime_mode, BrowserRuntimeMode::Simulated) {
            lines.push(
                "- Simulation failure: simulated runtime cannot satisfy live MVP validation."
                    .to_string(),
            );
        }
        return lines.join("\n");
    }

    if matches!(config.runtime_mode, BrowserRuntimeMode::Simulated) {
        lines.push("Live MVP validation failed: runtime mode is simulated.".to_string());
        return lines.join("\n");
    }

    for workload in WORKLOADS {
        let observation = observe_workload(
            &mut runtime,
            workload,
            system_report.meets_target,
            Duration::from_secs(10),
        );
        append_workload_result(&mut lines, &observation, system_report.meets_target);
    }

    lines.join("\n")
}

fn observe_workload(
    runtime: &mut EmbeddedRuntime,
    workload: WorkloadCase,
    system_meets_target: bool,
    timeout: Duration,
) -> WorkloadObservation {
    let started = Instant::now();
    let fixture_url = resolve_fixture_url(workload.fixture_path);
    let descriptor = runtime
        .create_browser_instance(&fixture_url)
        .expect("should create runtime browser instance");
    runtime
        .navigate_browser_instance(&descriptor.id, &fixture_url)
        .expect("should navigate runtime browser instance");
    runtime
        .resize_browser_surface(&descriptor.id, 1440, 900)
        .expect("should resize runtime browser surface");

    if matches!(workload.proof, WorkloadProof::InputInteraction) {
        dispatch_interaction_proof(runtime, &descriptor.id);
    }

    let wait_events = wait_for_observed_result(runtime, timeout);

    let snapshot = runtime
        .apply_observed_memory_sample(&descriptor.id)
        .expect("should apply observed memory sample");
    let mut events = wait_events;
    events.extend(runtime.drain_events());
    let elapsed = started.elapsed();
    let outcome = classify_observed_outcome(
        workload,
        &snapshot,
        &events,
        system_meets_target,
        elapsed >= timeout,
    );
    let failure_class = failure_class(outcome);
    let final_url = snapshot.browser.url.clone();
    let title_signal = snapshot.browser.title.clone();
    let render_proof = snapshot
        .browser
        .surface
        .render_evidence
        .clone()
        .unwrap_or_else(|| "missing render evidence".to_string());
    let input_proof = matches!(workload.proof, WorkloadProof::InputInteraction).then(|| {
        if input_proof_observed(&events) {
            "pointer, click, wheel, focus, resize, and text input were routed and observed"
                .to_string()
        } else {
            "input proof missing explicit routed input observations".to_string()
        }
    });

    runtime
        .close_browser_instance(&descriptor.id)
        .expect("should close runtime browser instance");
    let _ = runtime.drain_events();

    WorkloadObservation {
        workload,
        outcome,
        failure_class,
        final_url,
        title_signal,
        render_proof,
        input_proof,
        elapsed_ms: elapsed.as_millis(),
        events,
        snapshot,
    }
}

fn classify_observed_outcome(
    workload: WorkloadCase,
    snapshot: &RuntimeBrowserSnapshot,
    events: &[BrowserInstanceEvent],
    system_meets_target: bool,
    timed_out: bool,
) -> RuntimeOutcome {
    if timed_out {
        return RuntimeOutcome::Timeout;
    }
    if events
        .iter()
        .any(|event| event.kind == BrowserInstanceEventKind::Crashed)
    {
        return RuntimeOutcome::BrowserCrash;
    }
    if snapshot.browser.surface.host_surface_failure.is_some()
        || snapshot.browser.surface.render_evidence.is_none()
    {
        return RuntimeOutcome::HostSurfaceFailure;
    }
    if events
        .iter()
        .any(|event| event.kind == BrowserInstanceEventKind::NavigationFailed)
    {
        return RuntimeOutcome::CompatibilityFailure;
    }
    if !system_meets_target
        && workload.renderer_bytes + workload.browser_bytes + workload.gpu_bytes > 6 * GIB
    {
        return RuntimeOutcome::ConstrainedMemory;
    }
    RuntimeOutcome::Success
}

fn wait_for_observed_result(
    runtime: &mut EmbeddedRuntime,
    timeout: Duration,
) -> Vec<BrowserInstanceEvent> {
    let started = Instant::now();
    let mut events = Vec::new();
    while started.elapsed() < timeout {
        let batch = runtime.drain_events();
        let reached_terminal_observation = batch.iter().any(|event| {
            matches!(
                event.kind,
                BrowserInstanceEventKind::LoadFinished
                    | BrowserInstanceEventKind::NavigationFailed
                    | BrowserInstanceEventKind::Crashed
                    | BrowserInstanceEventKind::SurfaceUpdated
            )
        });
        events.extend(batch);
        if reached_terminal_observation {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    events
}

fn dispatch_interaction_proof(runtime: &mut EmbeddedRuntime, browser_id: &str) {
    let _ =
        runtime.dispatch_surface_input(browser_id, HostSurfaceInputEvent::Focus { focused: true });
    let _ = runtime.dispatch_surface_input(
        browser_id,
        HostSurfaceInputEvent::Resize {
            width: 1440,
            height: 900,
        },
    );
    let _ = runtime.dispatch_surface_input(
        browser_id,
        HostSurfaceInputEvent::PointerMove { x: 96, y: 96 },
    );
    let _ = runtime.dispatch_surface_input(
        browser_id,
        HostSurfaceInputEvent::PointerButton {
            x: 96,
            y: 96,
            button: HostMouseButton::Left,
            pressed: true,
            click_count: 1,
        },
    );
    let _ = runtime.dispatch_surface_input(
        browser_id,
        HostSurfaceInputEvent::PointerButton {
            x: 96,
            y: 96,
            button: HostMouseButton::Left,
            pressed: false,
            click_count: 1,
        },
    );
    let _ = runtime.dispatch_surface_input(
        browser_id,
        HostSurfaceInputEvent::Text {
            text: "webox-input-proof".to_string(),
        },
    );
    let _ = runtime.dispatch_surface_input(
        browser_id,
        HostSurfaceInputEvent::Wheel {
            x: 128,
            y: 128,
            delta_x: 0,
            delta_y: -240,
        },
    );
}

fn input_proof_observed(events: &[BrowserInstanceEvent]) -> bool {
    let summaries = events
        .iter()
        .map(|event| event.summary.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    summaries.contains("focus input")
        && summaries.contains("resize input")
        && summaries.contains("pointer move")
        && summaries.contains("pointer button")
        && summaries.contains("text input")
        && summaries.contains("wheel input")
}

fn failure_class(outcome: RuntimeOutcome) -> &'static str {
    match outcome {
        RuntimeOutcome::Success => "success",
        RuntimeOutcome::CompatibilityFailure => "compatibility-failure",
        RuntimeOutcome::EngineStartupFailure => "engine-startup-failure",
        RuntimeOutcome::HostSurfaceFailure => "host-surface-failure",
        RuntimeOutcome::BrowserCrash => "browser-crash",
        RuntimeOutcome::Timeout => "timeout",
        RuntimeOutcome::ConstrainedMemory => "constrained-memory",
    }
}

fn resolve_fixture_url(fixture_path: &str) -> String {
    if fixture_path.starts_with("http://") || fixture_path.starts_with("https://") {
        return fixture_path.to_string();
    }
    let path = PathBuf::from(fixture_path);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    format!("file://{}", absolute.display())
}

fn append_workload_result(
    lines: &mut Vec<String>,
    observation: &WorkloadObservation,
    system_meets_target: bool,
) {
    let workload = observation.workload;
    let snapshot = &observation.snapshot;
    lines.push(format!("### {}", workload.title));
    lines.push(format!("- Workload id: {}", workload.id));
    lines.push(format!("- Category: {}", workload.category));
    lines.push(format!("- Source: {}", workload.fixture_path));
    lines.push(format!("- Browser instance: {}", snapshot.browser.id));
    lines.push(format!("- Runtime backend: {:?}", snapshot.browser.backend));
    lines.push(format!("- Compatibility note: {}", workload.compatibility));
    lines.push(format!("- Outcome: {:?}", observation.outcome));
    lines.push(format!("- Failure class: {}", observation.failure_class));
    lines.push(format!("- Final URL: {}", observation.final_url));
    lines.push(format!(
        "- Title/readiness signal: {}",
        observation.title_signal
    ));
    lines.push(format!("- Render proof: {}", observation.render_proof));
    if let Some(input_proof) = &observation.input_proof {
        lines.push(format!("- Input proof: {input_proof}"));
    }
    lines.push(format!("- Elapsed ms: {}", observation.elapsed_ms));
    lines.push(format!(
        "- Memory total bytes: {}",
        snapshot.policy_decision.event.total_bytes
    ));
    lines.push(format!(
        "- Pressure level: {:?}",
        snapshot.policy_decision.event.level
    ));
    lines.push(format!(
        "- Actions: {}",
        snapshot
            .policy_decision
            .actions
            .iter()
            .map(|action| format!("{:?}", action))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    lines.push(format!(
        "- UI indicator: {}",
        snapshot
            .browser
            .memory_indicator
            .clone()
            .unwrap_or_else(|| "none".to_string())
    ));
    lines.push(format!(
        "- Failure state: {}",
        snapshot
            .browser
            .failure_state
            .clone()
            .unwrap_or_else(|| "none".to_string())
    ));
    lines.push(format!(
        "- Memory attribution: {}",
        snapshot
            .browser
            .memory_attribution
            .clone()
            .unwrap_or_else(|| "none".to_string())
    ));
    lines.push(format!(
        "- Surface frame: {} ({})",
        snapshot.browser.surface.frame_token, snapshot.browser.surface.last_frame_label
    ));
    lines.push(format!(
        "- Observed events: {}",
        observation
            .events
            .iter()
            .map(|event| format!("{:?}", event.kind))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    lines.push(format!(
        "- Target outcome: {}",
        if system_meets_target {
            "system satisfies configured headroom target"
        } else {
            "system is below configured headroom target"
        }
    ));
    lines.push(String::new());
}

#[cfg(test)]
mod tests {
    use super::{
        HarnessConfig, RuntimeOutcome, WorkloadCase, WorkloadProof, classify_observed_outcome,
        run_harness,
    };
    use webox_config::BrowserRuntimeMode;
    use webox_engine::{
        BrowserInstanceState, BrowserSurfaceRenderMode, BrowserSurfaceState, RuntimeBackend,
    };
    use webox_memory::{MemoryAttribution, MemoryController, TabTelemetry};
    use webox_runtime_api::RuntimeBrowserSnapshot;

    #[test]
    fn constrained_systems_are_classified_explicitly() {
        let workload = WorkloadCase {
            id: "heavy",
            category: "heavy",
            title: "Heavy",
            fixture_path: "https://example.com",
            renderer_bytes: 7 * 1024 * 1024 * 1024,
            browser_bytes: 1024,
            gpu_bytes: 1024,
            compatibility: "heavy",
            proof: WorkloadProof::MemoryPressure,
        };
        let snapshot = test_snapshot("tab-1", Some("painted".to_string()));

        assert_eq!(
            classify_observed_outcome(workload, &snapshot, &[], false, false),
            RuntimeOutcome::ConstrainedMemory
        );
    }

    #[test]
    fn missing_render_evidence_is_host_surface_failure() {
        let workload = WorkloadCase {
            id: "render",
            category: "render",
            title: "Render",
            fixture_path: "https://example.com",
            renderer_bytes: 1,
            browser_bytes: 1,
            gpu_bytes: 1,
            compatibility: "render",
            proof: WorkloadProof::RenderOnly,
        };
        let snapshot = test_snapshot("tab-1", None);

        assert_eq!(
            classify_observed_outcome(workload, &snapshot, &[], true, false),
            RuntimeOutcome::HostSurfaceFailure
        );
    }

    #[test]
    fn navigation_failure_requires_observed_event() {
        let workload = WorkloadCase {
            id: "navigation-failure",
            category: "navigation-failure",
            title: "Navigation Failure",
            fixture_path: "http://127.0.0.1:9/failure",
            renderer_bytes: 1,
            browser_bytes: 1,
            gpu_bytes: 1,
            compatibility: "failure",
            proof: WorkloadProof::NavigationFailure,
        };
        let snapshot = test_snapshot("tab-1", Some("painted".to_string()));

        assert_eq!(
            classify_observed_outcome(workload, &snapshot, &[], true, false),
            RuntimeOutcome::Success
        );
    }

    #[test]
    fn harness_report_classifies_missing_live_runtime_as_startup_failure() {
        let report = run_harness(&HarnessConfig {
            available_memory_bytes: 16 * 1024 * 1024 * 1024,
            output_path: ".webox/validation/test.md",
            runtime_mode: BrowserRuntimeMode::RealCef,
        });

        assert!(report.contains("Live MVP validation did not run workloads"));
        assert!(report.contains("EngineStartupFailure"));
        assert!(report.contains("engine-startup-failure"));
        assert!(report.contains("not a synthetic success"));
    }

    fn test_snapshot(tab_id: &str, render_evidence: Option<String>) -> RuntimeBrowserSnapshot {
        let telemetry = TabTelemetry {
            tab_id: tab_id.to_string(),
            renderer_bytes: 1,
            browser_bytes: 1,
            gpu_bytes: 1,
        };
        RuntimeBrowserSnapshot {
            browser: BrowserInstanceState {
                id: tab_id.to_string(),
                url: "https://example.com".to_string(),
                title: "Example".to_string(),
                is_loading: false,
                backend: RuntimeBackend::Cef,
                memory_usage_bytes: 3,
                memory_indicator: None,
                failure_state: None,
                memory_attribution: Some(MemoryAttribution::aggregate_process(1).label()),
                surface: BrowserSurfaceState {
                    surface_id: format!("surface-{tab_id}"),
                    render_mode: BrowserSurfaceRenderMode::CefOffscreen,
                    width: 1440,
                    height: 900,
                    focused: true,
                    frame_token: 1,
                    last_frame_label: "frame".to_string(),
                    render_evidence,
                    frame_buffer: None,
                    damage_events: 1,
                    host_surface_failure: None,
                },
                history: vec!["https://example.com".to_string()],
                history_index: 0,
                status_text: "loaded".to_string(),
                can_go_back: false,
                can_go_forward: false,
            },
            policy_decision: MemoryController::new(100).evaluate(&telemetry),
        }
    }
}
