## 1. Project foundation

- [x] 1.1 Create the monorepo layout for browser app, engine/runtime modules, shared packages, and docs or marketing site
- [x] 1.2 Create the Rust workspace and core crate structure for `shell`, `engine`, `memory`, `ui`, and `runtime-api`
- [x] 1.3 Add repository-level tooling, build scripts, and local development setup documentation for multi-surface development

## 2. Shared packages and workspace contracts

- [x] 2.1 Define shared configuration types for browser startup, subprocess launch options, and environment-specific paths
- [x] 2.2 Create initial shared packages for branding, shared schemas, or cross-surface utilities needed by browser and site surfaces
- [x] 2.3 Document workspace boundaries and contribution guidance for browser, engine, packages, and site code

## 3. Browser shell and engine bootstrap

- [x] 3.1 Implement the Rust application shell that starts and shuts down the embedded browser runtime cleanly
- [x] 3.3 Add structured startup and shutdown diagnostics for host and embedded engine failures

## 4. Tab and window lifecycle

- [x] 4.1 Implement browser window creation and the root tab container model
- [x] 4.2 Implement tab creation, activation, navigation, reload, back, forward, and close commands
- [x] 4.3 Add stable identifiers and state tracking for windows, tabs, and browser instances

## 5. High-memory telemetry and control

- [x] 5.1 Implement per-tab and per-process memory telemetry collection for renderer, browser, and related subprocesses
- [x] 5.2 Define configurable memory thresholds, target headroom settings, and supported-system capability checks
- [x] 5.3 Implement a memory controller that evaluates thresholds and emits structured pressure events

## 6. OOM mitigation and diagnostics

- [x] 6.1 Implement mitigation policies for warning, deprioritizing background work, and recovery-oriented actions under memory pressure
- [x] 6.2 Capture diagnostics for suspected memory-related tab or subprocess failures
- [x] 6.3 Expose visible reporting when the configured 8 GB per-tab target cannot be met on the current system

## 7. Browser UI MVP

- [x] 7.2 Bind UI interactions to shell commands for navigation and tab lifecycle operations
- [x] 7.3 Surface loading state, tab title changes, active tab selection, and memory-pressure indicators in the UI

## 8. Embeddable runtime surface

- [x] 8.1 Define the embeddable runtime API for host applications to initialize and manage browser instances
- [x] 8.2 Expose supported runtime configuration hooks for hosts without requiring standalone browser chrome
- [x] 8.3 Expose memory diagnostics and pressure-event callbacks through the embeddable runtime API

## 9. Docs and marketing site

- [x] 9.1 Create the docs or marketing site workspace with shared branding and content foundations
- [x] 9.2 Build initial product pages that explain webox, its high-memory goals, and target use cases
- [x] 9.3 Build initial developer-facing pages for architecture overview, repository layout, and getting started guidance

## 10. Compatibility and workload validation

- [x] 10.1 Create a representative validation suite covering JavaScript, HTML, CSS, DOM, media, storage, WebGL, and WebAssembly behavior

## 11. Product hardening

- [x] 11.1 Add developer-facing documentation for architecture, runtime configuration, memory-policy tuning, and workspace usage
- [x] 11.2 Add smoke tests for startup, shutdown, tab lifecycle, core navigation flows, and workspace-level developer commands
- [x] 11.3 Review remaining MVP gaps and document follow-up work for deeper Chromium integration, extension support, and production readiness

## Follow-Up Change

- [x] The remaining real CEF embedding, standalone browser chrome, and live workload verification work has been split into a dedicated follow-up change.
