# Workload Validation

webox includes a Rust workload harness that exercises representative browser scenarios against the live runtime path and memory policy layers.

## Commands

- `cargo run -p webox-workload-harness -- supported`
- `cargo run -p webox-workload-harness -- constrained`

## Coverage

- Large DOM and CSS stress content
- Canvas / Unity-WebGL-style GPU workloads
- Worker and WASM-heavy memory allocation patterns
- Modern heavy web-app and data-visualization representatives
- Explicit runtime classification for success, compatibility failure, engine failure, and constrained-memory outcomes

## Outputs

- Generated harness reports: `.webox/validation/harness-supported.md`, `.webox/validation/harness-constrained.md`
- Persistent review notes: `validation/reports/supported-system.md`, `validation/reports/constrained-system.md`, `validation/reports/memory-pressure-behavior.md`

## Interpretation

- Supported runs should report `Meets target: true` for the configured 8 GiB per-tab target.
- Constrained runs should report `Meets target: false` and make unmet-target behavior explicit.
- Per-workload sections include compatibility notes, runtime outcome classification, observed engine events, memory pressure classification, attribution notes, mitigation actions, and failure-state details.
