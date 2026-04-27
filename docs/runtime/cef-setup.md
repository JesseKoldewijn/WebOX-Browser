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
