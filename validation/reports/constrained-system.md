# Constrained-System Validation

- Host profile: constrained machine with about 6 GiB available memory for browser testing
- Configured per-tab target: 8 GiB practical headroom
- Validation command: `cargo run -p webox-workload-harness -- constrained`
- Expected result: workload report states `Meets target: false`

## Expected Diagnostic Behavior

- webox must report that the system is below the configured target instead of implying success.
- Large DOM scenarios may stay below exhaustion, but heavier WebGL, WASM, and modern app simulations should escalate sooner.
- Memory-related failure states should be attached to the affected browser instance and included in the generated report.

## Review Focus

- Confirm the workload harness output under `.webox/validation/harness-constrained.md` explicitly records unmet target conditions.
- Confirm UI memory indicators map to runtime memory pressure levels.
- Confirm recovery-oriented diagnostics remain available for post-run analysis.
