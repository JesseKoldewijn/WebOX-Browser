## Why

The `high-memory-browser` change established the monorepo, Rust runtime foundation, shared packages, Astro docs site, and browser-state scaffolding, but it intentionally stopped short of the real browser implementation. webox now needs a dedicated follow-up phase that turns those foundations into an actual browser by wiring in real CEF embedding, real standalone browser chrome, and live workload validation against the embedded engine.

## What Changes

- Replace the current stub engine bootstrap with real CEF integration and subprocess lifecycle wiring.
- Implement the real standalone browser chrome, including windows, address bar, tab strip, navigation controls, and visible browser state.
- Connect the existing shell, UI state, memory policy, and runtime abstractions to live browser instances rather than in-memory placeholders.
- Add live workload validation that runs representative heavy web apps, data-visualization content, Unity WebGL scenarios, and WASM-heavy workloads inside the embedded browser.
- Verify browser behavior and diagnostics for systems that meet the configured high-memory target and for systems that fall below it.

## Capabilities

### New Capabilities
- `cef-runtime-integration`: Real CEF bootstrap, subprocess management, and embedded browser lifecycle support.
- `standalone-browser-chrome`: Real standalone browser UI host with address bar, tab strip, navigation controls, window controls, and browser state presentation.
- `live-workload-validation`: Automated workload execution and verification against the real embedded browser, including high-memory diagnostics.

### Modified Capabilities
- `chromium-embedding`: Upgrade the existing engine integration boundary from planned/stubbed behavior to real CEF-backed execution.
- `browser-ui`: Upgrade the existing browser UI capability from state-model and shell command bindings to a real visible browser chrome implementation.
- `high-memory-tabs`: Upgrade the memory capability from policy scaffolding to end-to-end validation against live browser workloads.

## Impact

- Affects the Rust engine and shell crates, browser app entrypoint, and runtime integration strategy.
- Introduces concrete CEF distribution, binding, and subprocess management requirements.
- Requires a browser UI host decision and real standalone browser chrome implementation.
- Adds browser-driven validation infrastructure that will exercise compatibility and memory-behavior goals against real workloads.
