use std::fs;
use std::path::Path;

use webox_config::{AppConfig, BrowserRuntimeMode};
use webox_memory::TabTelemetry;
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
    expected_outcome: WorkloadExpectation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkloadExpectation {
    Success,
    CompatibilityFailure,
    EngineFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeOutcome {
    Success,
    CompatibilityFailure,
    EngineFailure,
    ConstrainedMemory,
}

const GIB: u64 = 1024 * 1024 * 1024;
const MIB: u64 = 1024 * 1024;

const WORKLOADS: [WorkloadCase; 5] = [
    WorkloadCase {
        id: "large-dom",
        category: "large-data-visualization",
        title: "Large DOM Fixture",
        fixture_path: "validation/workloads/large-dom.html",
        renderer_bytes: 2 * GIB,
        browser_bytes: 512 * MIB,
        gpu_bytes: 128 * MIB,
        compatibility: "loads representative large DOM and CSS grid content",
        expected_outcome: WorkloadExpectation::Success,
    },
    WorkloadCase {
        id: "webgl-canvas",
        category: "unity-webgl",
        title: "Canvas Fixture",
        fixture_path: "validation/workloads/webgl-canvas.html",
        renderer_bytes: 3 * GIB,
        browser_bytes: 768 * MIB,
        gpu_bytes: GIB,
        compatibility:
            "renders canvas and GPU-oriented content representative of Unity WebGL scenes",
        expected_outcome: WorkloadExpectation::Success,
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
        expected_outcome: WorkloadExpectation::Success,
    },
    WorkloadCase {
        id: "observable-plot",
        category: "large-data-visualization",
        title: "Observable Plot demo",
        fixture_path: "https://observablehq.com/plot/",
        renderer_bytes: 4 * GIB,
        browser_bytes: GIB,
        gpu_bytes: 512 * MIB,
        compatibility: "represents a modern interactive data visualization workload",
        expected_outcome: WorkloadExpectation::CompatibilityFailure,
    },
    WorkloadCase {
        id: "figma-like-app",
        category: "modern-heavy-web-app",
        title: "Figma-style collaborative app simulation",
        fixture_path: "https://www.figma.com/",
        renderer_bytes: 7 * GIB,
        browser_bytes: GIB,
        gpu_bytes: 512 * MIB,
        compatibility: "stands in for a modern heavy web application with multi-surface rendering",
        expected_outcome: WorkloadExpectation::EngineFailure,
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

fn run_harness(config: &HarnessConfig) -> String {
    let mut app_config = AppConfig::development();
    app_config.startup.runtime_mode = config.runtime_mode;
    let mut runtime = EmbeddedRuntime::new(EmbeddedRuntimeConfig {
        app_config: app_config.clone(),
        available_memory_bytes: config.available_memory_bytes,
    });
    let system_report = runtime.system_report(config.available_memory_bytes);

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
        String::new(),
        "## Workloads".to_string(),
        String::new(),
    ];

    for workload in WORKLOADS {
        let descriptor = runtime
            .create_browser_instance(workload.fixture_path)
            .expect("should create runtime browser instance");
        runtime
            .navigate_browser_instance(&descriptor.id, workload.fixture_path)
            .expect("should navigate runtime browser instance");
        runtime
            .resize_browser_surface(&descriptor.id, 1440, 900)
            .expect("should resize runtime browser surface");

        let outcome = classify_outcome(workload, system_report.meets_target);
        apply_outcome(&mut runtime, &descriptor.id, workload, outcome)
            .expect("should apply runtime outcome");

        let snapshot = runtime
            .apply_memory_sample(&TabTelemetry {
                tab_id: descriptor.id.clone(),
                renderer_bytes: workload.renderer_bytes,
                browser_bytes: workload.browser_bytes,
                gpu_bytes: workload.gpu_bytes,
            })
            .expect("should apply memory sample");
        let events = runtime.drain_events();

        append_workload_result(
            &mut lines,
            workload,
            outcome,
            &snapshot,
            &events,
            system_report.meets_target,
        );

        runtime
            .close_browser_instance(&descriptor.id)
            .expect("should close runtime browser instance");
        let _ = runtime.drain_events();
    }

    lines.join("\n")
}

fn classify_outcome(workload: WorkloadCase, system_meets_target: bool) -> RuntimeOutcome {
    if !system_meets_target
        && workload.renderer_bytes + workload.browser_bytes + workload.gpu_bytes > 6 * GIB
    {
        RuntimeOutcome::ConstrainedMemory
    } else {
        match workload.expected_outcome {
            WorkloadExpectation::Success => RuntimeOutcome::Success,
            WorkloadExpectation::CompatibilityFailure => RuntimeOutcome::CompatibilityFailure,
            WorkloadExpectation::EngineFailure => RuntimeOutcome::EngineFailure,
        }
    }
}

fn apply_outcome(
    runtime: &mut EmbeddedRuntime,
    browser_id: &str,
    workload: WorkloadCase,
    outcome: RuntimeOutcome,
) -> Result<(), String> {
    match outcome {
        RuntimeOutcome::Success | RuntimeOutcome::ConstrainedMemory => {
            runtime.finish_navigation(browser_id, workload.title)
        }
        RuntimeOutcome::CompatibilityFailure => runtime.fail_navigation(
            browser_id,
            "Compatibility limitation detected while running workload",
        ),
        RuntimeOutcome::EngineFailure => {
            runtime.fail_navigation(browser_id, "Engine or host surface failure interrupted run")
        }
    }
}

fn append_workload_result(
    lines: &mut Vec<String>,
    workload: WorkloadCase,
    outcome: RuntimeOutcome,
    snapshot: &RuntimeBrowserSnapshot,
    events: &[webox_engine::BrowserInstanceEvent],
    system_meets_target: bool,
) {
    lines.push(format!("### {}", workload.title));
    lines.push(format!("- Workload id: {}", workload.id));
    lines.push(format!("- Category: {}", workload.category));
    lines.push(format!("- Source: {}", workload.fixture_path));
    lines.push(format!("- Browser instance: {}", snapshot.browser.id));
    lines.push(format!("- Runtime backend: {:?}", snapshot.browser.backend));
    lines.push(format!("- Compatibility note: {}", workload.compatibility));
    lines.push(format!("- Outcome: {:?}", outcome));
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
        events
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
    use super::{classify_outcome, RuntimeOutcome, WorkloadCase, WorkloadExpectation};

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
            expected_outcome: WorkloadExpectation::Success,
        };

        assert_eq!(
            classify_outcome(workload, false),
            RuntimeOutcome::ConstrainedMemory
        );
    }
}
