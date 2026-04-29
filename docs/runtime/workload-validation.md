# Workload Validation

webox includes a Rust workload harness that exercises representative browser scenarios against the live runtime path and memory policy layers.

## Commands

- `cargo run -p webox-workload-harness -- supported`
- `cargo run -p webox-workload-harness -- constrained`

## Coverage

- Standards rendering fixture for HTML/CSS/DOM paint proof
- Large DOM and CSS stress content
- Input interaction fixture for text, click, scroll, focus, and resize routing
- Tab state preservation fixture for storage/session evidence
- Canvas / Unity-WebGL-style GPU workloads
- Worker and WASM-heavy memory allocation patterns
- Navigation failure fixture for observed failure classification
- Explicit runtime classification for success, compatibility failure, engine startup failure, host-surface failure, browser crash, timeout, and constrained-memory outcomes

## Outputs

- Generated harness reports: `.webox/validation/harness-supported.md`, `.webox/validation/harness-constrained.md`
- Persistent review notes: `validation/reports/supported-system.md`, `validation/reports/constrained-system.md`, `validation/reports/memory-pressure-behavior.md`

## Interpretation

- Supported runs should report `Meets target: true` for the configured 8 GiB per-tab target.
- Constrained runs should report `Meets target: false` and make unmet-target behavior explicit.
- Per-workload sections include compatibility notes, runtime outcome classification, observed engine events, memory pressure classification, attribution notes, mitigation actions, and failure-state details.
- Live MVP reports must include final URL, title/readiness signal, render proof, timing, memory diagnostics, failure class, and input proof where applicable.
- A report that says the runtime is simulated, not live-MVP-ready, missing render proof, or derived from scripted expectations is a failed live MVP validation.
- In the current local environment without CEF assets under `third_party/cef/linux-x64`, the generated `.webox/validation/*.md` reports are expected to show `engine-startup-failure` rather than full live execution evidence.

## Acceptance Evidence

Before archiving the live browser MVP change, capture:

- `cargo test` output for readiness diagnostics, event mapping, memory attribution metadata, shell smoke flows, and workload classification.
- `cargo run -p webox-workload-harness -- supported` output or report path.
- `cargo run -p webox-workload-harness -- constrained` output or report path.
- Environment notes explaining any missing CEF assets or host limitations that prevent full live execution.
- Confirmation that simulated mode remained explicitly non-compliant for live MVP acceptance.
