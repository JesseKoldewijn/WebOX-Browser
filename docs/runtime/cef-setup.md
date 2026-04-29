# CEF Setup

The current real-browser bring-up targets Linux x86_64 first.

## Selected Approach

- Rust integration crate: `cef` from `tauri-apps/cef-rs`
- Standalone browser chrome host: `eframe`/`egui`
- Primary bring-up platform: Linux x86_64

## Runtime Layout

CEF assets are expected under `third_party/cef/linux-x64/`.

Expected structure:

- `third_party/cef/linux-x64/bin/webox-cef-subprocess`
- `third_party/cef/linux-x64/resources/`
- `third_party/cef/linux-x64/locales/`
- `third_party/cef/linux-x64/README.md`

The live MVP readiness check also verifies writable runtime directories for data,
cache, and logs from the active `AppConfig`. Missing assets are reported as
startup diagnostics instead of falling back to simulated browsing.

## Prepare The Staging Area

Run:

```bash
bun run setup:cef
```

This creates the expected directory layout so a chosen CEF distribution can be placed there.

## Current Status

The repository now has a real CEF runtime configuration contract, engine-driven browser instance tracking, and a live browser surface contract for the standalone eframe host, but it does not vendor or download CEF automatically yet. The current runtime can execute the shell and workload harness against simulated or real-CEF bring-up paths depending on available assets.

## Toolchain Baseline

- Rust edition: `2024`
- Minimum compiler: `rustc 1.85+`

## Validation Commands

- `cargo run -p webox-browser-app`
- `cargo run -p webox-workload-harness -- supported`
- `cargo run -p webox-workload-harness -- constrained`

## Live MVP Modes

- Live mode uses `BrowserRuntimeMode::RealCef` and requires CEF assets,
  resources, locales, subprocess executable, cache/data/log directories, and
  remote debugging configuration to be usable.
- Simulated mode uses `BrowserRuntimeMode::Simulated` for unit tests and local
  development scaffolding. It is explicitly marked `live_mvp_ready=false` and
  cannot satisfy live MVP validation.
- If CEF assets are missing, the browser diagnostics panel and workload harness
  report `engine-startup-failure` with missing-path details.

## Expected Failure Diagnostics

- Missing CEF distribution root/resources/locales/subprocess: startup reports
  `RuntimeReadinessState::LiveUnavailable` and lists each missing path.
- CEF startup or browser creation failure: runtime APIs return an error instead
  of creating synthetic live tabs.
- Host surface failure: workload validation reports `host-surface-failure` when
  rendered frame evidence is missing.
- Simulated validation: workload validation reports not live-MVP-ready and does
  not treat simulated output as acceptance evidence.
