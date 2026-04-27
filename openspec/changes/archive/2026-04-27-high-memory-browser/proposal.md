## Why

Modern browsers still fail under extreme per-tab memory pressure, especially for heavy web applications, large data visualization workloads, Unity WebGL games, and WebAssembly-based tools. webox (web-browser overdrive experience) is intended to raise the practical memory ceiling to at least 8 GB per tab while preserving Chromium-class web compatibility so advanced workloads can run without avoidable out-of-memory failures.

## What Changes

- Introduce `webox`, a browser initiative focused on much higher per-tab memory headroom than mainstream browsers.
- Establish webox as a monorepo that contains the browser application, the embeddable engine/runtime, shared packages, and a docs/marketing website.
- Define the browser foundation architecture based on a Rust shell and process-control layer intended to pair with Chromium compatibility through CEF in a follow-up implementation change.
- Add a memory architecture foundation that establishes the target of at least 8 GB of available memory per tab, along with monitoring, policy, and diagnostics scaffolding.
- Define a standalone browser MVP direction and shell-facing browser state model while deferring the real browser chrome implementation to a follow-up implementation change.
- Define an embeddable engine/runtime path and initial reusable runtime scaffolding so the same core can later be integrated into other products in addition to the standalone browser.
- Define a docs and marketing website that explains the product vision, architecture, developer onboarding, and future product narrative.
- Establish compatibility requirements for JavaScript, HTML, CSS, WASM, media, and other modern web platform features expected from Chromium-class browsing.

## Capabilities

### New Capabilities
- `workspace-monorepo`: Monorepo structure that organizes browser apps, engine/runtime modules, shared packages, and the docs/marketing site.
- `browser-shell`: Rust-based application shell for launching, controlling, and presenting the webox browser experience.
- `chromium-embedding`: CEF integration boundary and bootstrap plan that prepares for real Chromium embedding in a follow-up implementation change.
- `high-memory-tabs`: Per-tab memory orchestration, monitoring, and policies that target at least 8 GB available memory per tab.
- `browser-ui`: Browser UI state model, shell command bindings, and MVP browser chrome plan, with real window chrome deferred to a follow-up implementation change.
- `embeddable-runtime`: Reusable embedding surface that exposes the core webox runtime for non-browser host applications.
- `docs-marketing-site`: Public-facing website for product messaging, technical documentation, and developer onboarding.

### Modified Capabilities
- None.

## Impact

- Affects overall project architecture, runtime choice, process model, and browser engine strategy.
- Establishes monorepo expectations for project layout, package boundaries, shared code reuse, and site/application coexistence.
- Introduces Rust as the primary implementation language for the host/runtime foundation and establishes CEF as the planned browser engine dependency for the next implementation phase.
- Requires capability specs for monorepo workspace management, browser shell, Chromium embedding, high-memory tab handling, browser UI, embeddable runtime support, and the docs/marketing site.
- Sets a high-performance, high-compatibility expectation that will influence future decisions around process isolation, memory allocators, crash recovery, browser UI hosting, and test strategy.
