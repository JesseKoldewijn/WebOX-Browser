# Supported-System Validation

- Host profile: development workstation with at least 16 GiB available memory for browser testing
- Configured per-tab target: 8 GiB practical headroom
- Validation command: `cargo run -p webox-workload-harness -- supported`
- Expected result: workload report states `Meets target: true`

## Observed Workloads

- Large DOM and data visualization scenarios remain within the configured target and should stay active without browser-level OOM termination.
- Canvas / Unity-WebGL-style scenarios can enter warning or critical pressure while still reporting that the system satisfies the configured target.
- WASM-heavy and modern heavy web-app scenarios may approach exhaustion, but diagnostics remain visible and attributable to a specific browser instance.
- The generated harness output should classify each run as success, compatibility failure, engine failure, or constrained-memory outcome rather than treating every failure as equivalent.

## Recovery And Mitigation Expectations

- Warning and critical states should surface visible memory indicators in the browser UI.
- Exhausted states should capture recovery diagnostics instead of failing silently.
- The harness report under `.webox/validation/harness-supported.md` provides per-workload compatibility notes, runtime outcome classification, observed engine events, and pressure classification.
