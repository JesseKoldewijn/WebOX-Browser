## Why

webox now has a solid browser shell prototype, but the core browser path is still simulated in the places that matter most: real page rendering, engine-driven tab state, and workload validation against live browser behavior. This change closes the remaining MVP gaps so the standalone browser and embeddable engine can be evaluated as a functioning product rather than a scaffold.

## What Changes

- Replace in-memory browser instance placeholders with real CEF-backed browser creation, navigation lifecycle, and engine-originated state updates.
- Embed a live browser content surface into the standalone eframe host so tabs render real pages instead of placeholder status content.
- Route title, loading, history, crash, and failure signals from the embedded engine back into the shell and visible browser chrome.
- Upgrade embeddable runtime and memory reporting flows to reflect live browser instances and observed runtime behavior instead of synthetic-only samples.
- Replace the synthetic workload harness path with live workload execution that records real compatibility, navigation, and memory diagnostics.
- Pin the Rust workspace toolchain contract so all crates consistently target the Rust 2024 edition with an explicit compiler baseline.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `browser-ui`: change browser UI requirements from shell-only chrome presentation to a real embedded page surface with engine-driven tab, loading, and failure state.
- `browser-shell`: change shell requirements so browser commands, lifecycle, and shutdown behavior are backed by live engine instances rather than UI-only state transitions.
- `chromium-embedding`: change engine requirements from bootstrap/configuration only to full live browser instance creation, event propagation, and embedded rendering integration.
- `embeddable-runtime`: change runtime requirements so host applications interact with live browser instances and real runtime diagnostics.
- `high-memory-tabs`: change memory requirements so pressure signals and failure reporting are derived from live browser execution paths.
- `live-workload-validation`: change validation requirements from representative harness scaffolding to real workload execution and diagnostics collection against the embedded browser.
- `workspace-monorepo`: change workspace tooling requirements to include an explicit Rust 2024 toolchain baseline shared across workspace crates.

## Impact

- Affects `crates/engine`, `crates/shell`, `crates/runtime-api`, `crates/memory`, `crates/ui`, and `apps/browser`.
- Affects workload validation tooling in `tools/workload-harness` and supporting runtime docs.
- Tightens the repository toolchain contract in workspace Rust manifests and developer setup guidance.
- Raises the MVP bar from simulated browser behavior to real embedded-browser behavior, which will influence testing, diagnostics, and developer workflows.
