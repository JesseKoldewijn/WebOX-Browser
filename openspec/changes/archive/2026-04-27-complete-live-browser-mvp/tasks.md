## 1. Live engine instance model

- [x] 1.1 Replace placeholder browser-instance records in `crates/engine` with live CEF-backed instance creation and stable runtime identifiers
- [x] 1.2 Add engine lifecycle event propagation for navigation, title, loading, failure, and surface updates
- [x] 1.3 Route shell and runtime APIs to consume engine-originated lifecycle events instead of manually finalizing steady-state tab values

## 2. Embedded rendering in the standalone browser

- [x] 2.1 Define the rendering/surface contract needed to bind live CEF browser content into the eframe host
- [x] 2.2 Replace the placeholder center-panel browser surface in `apps/browser` with a live embedded page surface
- [x] 2.3 Wire tab switching, resize, focus, and shutdown flows so the visible page surface stays synchronized with the active live tab

## 3. Shell and UI truthfulness

- [x] 3.1 Update `crates/shell` so navigate, reload, back, and forward operate against live engine instances instead of UI-only history transitions
- [x] 3.2 Update `crates/ui` state models to reflect engine-observed loading, title, crash, and failure data
- [x] 3.3 Add smoke and unit coverage proving browser chrome actions mutate live browser state and shut down cleanly

## 4. Live runtime memory and diagnostics

- [x] 4.1 Connect memory telemetry inputs to observed live browser execution paths and propagate results consistently through engine, shell, and runtime APIs
- [x] 4.2 Surface live memory-pressure and failure diagnostics in the standalone browser UI and embeddable runtime
- [x] 4.3 Define fallback diagnostics when precise per-tab attribution is unavailable during live execution

## 5. Real workload validation

- [x] 5.1 Refactor `tools/workload-harness` to launch and drive real browser workloads through the live runtime path
- [x] 5.2 Capture deterministic workload outcomes including success, compatibility failure, engine failure, constrained-memory outcome, and key diagnostics
- [x] 5.3 Update validation reports and commands to document the real workload evidence gathered on supported and constrained systems

## 6. Rust 2024 toolchain contract

- [x] 6.1 Add an explicit workspace Rust compiler baseline that matches the Rust 2024 edition requirements across all crates
- [x] 6.2 Update Rust workspace and developer docs to describe the required compiler/toolchain version and validation commands
- [x] 6.3 Verify the workspace builds and tests successfully under the declared Rust baseline
