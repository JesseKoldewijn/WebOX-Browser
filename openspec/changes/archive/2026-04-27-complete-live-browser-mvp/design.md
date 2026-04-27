## Context

The repository now has a coherent browser architecture, but the core execution path still contains simulation seams. `crates/engine` initializes CEF and tracks browser instances as Rust state, `crates/shell` coordinates windows and tabs, `apps/browser` renders visible browser chrome through eframe, and `tools/workload-harness` exercises representative scenarios. However, the engine does not yet create a live rendered browser surface, the shell and UI still author parts of browser state manually, and the workload harness records synthetic outcomes rather than proving behavior against real pages.

This change is the bridge from prototype to functioning MVP. It has to connect real engine objects to the standalone browser, make engine-originated events the source of truth for tab state, and turn validation into evidence rather than simulation. It also needs to tighten the Rust toolchain contract so the workspace consistently advertises and enforces the Rust 2024 edition baseline it already assumes.

## Goals / Non-Goals

**Goals:**
- Create real CEF-backed browser instances that drive navigation, title, loading, history, crash, and failure state.
- Render a live embedded browser surface inside the standalone eframe host so the browser UI is not limited to placeholder content.
- Shift shell, runtime API, and memory reporting flows from host-authored state transitions to engine-observed state transitions.
- Upgrade the workload harness to run real browser pages and record actual compatibility, navigation, and memory diagnostics.
- Explicitly declare the Rust workspace compiler baseline needed for the Rust 2024 edition across all crates and developer workflows.

**Non-Goals:**
- Replacing eframe as the standalone browser host in this phase.
- Expanding beyond the current Linux-first CEF bring-up target in this phase.
- Delivering full browser-product parity features such as extensions, complete DevTools integration, or multi-window session persistence.
- Redesigning the existing crate boundaries unless a specific integration seam proves they are insufficient.

## Decisions

### 1. Make the engine the source of truth for browser lifecycle state

The engine will own live browser instance creation and emit lifecycle updates that the shell and UI consume, rather than relying on the host to manually finalize navigation or synthesize steady-state tab values.

Rationale:
- Real browser behavior depends on title, load, crash, and navigation events coming from the embedded engine.
- Keeping lifecycle truth in one place prevents UI-only state from drifting away from actual browser state.
- This approach aligns the standalone shell and embeddable runtime around the same event model.

Alternatives considered:
- Continue mixing engine-backed state with host-authored transitions: rejected because it preserves the current simulation seam and makes validation untrustworthy.

### 2. Treat embedded rendering as a first-class integration boundary

The browser app will integrate a real content surface abstraction between the CEF browser instance and the eframe host, rather than leaving rendering implicit inside engine startup.

Rationale:
- Rendering is the missing center of the MVP and needs an explicit contract for paint, resize, focus, and input flow.
- A first-class rendering boundary keeps `apps/browser`, `crates/shell`, and `crates/engine` responsibilities understandable.
- It allows the standalone host and embeddable runtime to share live-instance concepts without forcing them to share the same UI implementation.

Alternatives considered:
- Hide page-surface behavior behind the existing in-memory browser descriptor types: rejected because rendering introduces lifecycle and event concerns that deserve explicit modeling.

### 3. Upgrade validation only after live runtime truth exists

The workload harness will be refactored to drive the same live runtime/browser paths used by the standalone app, and it will collect diagnostics from those live runs rather than continuing to inject synthetic end states.

Rationale:
- Validation claims are only meaningful if they are derived from the same engine behavior used by the product.
- Reusing the live runtime path reduces the risk of having one codepath for demos and another for tests.
- This sequencing makes harness failures actionable because they reflect actual runtime behavior.

Alternatives considered:
- Make the harness more elaborate while still relying on simulated state: rejected because it improves reporting without improving truthfulness.

### 4. Keep memory policy tied to observed browser process data

The memory controller will continue to own threshold logic, but its inputs and resulting failure states will be sourced from observed live browser execution and surfaced consistently in shell, UI, and runtime APIs.

Rationale:
- The differentiator is not just policy configuration; it is how the browser behaves under real pressure.
- Observed memory data is required to make diagnostics and recovery reporting credible.
- This preserves the current memory-policy boundary while making it operate on real signals.

Alternatives considered:
- Move memory logic into the engine entirely: rejected because it would blur the existing separation between observation and policy.

### 5. Explicitly declare the Rust 2024 compiler baseline at the workspace level

The Rust workspace will continue using the 2024 edition and will add an explicit minimum supported Rust version contract that all crates inherit.

Rationale:
- The repository already depends on the 2024 edition semantically, but developers and CI need a concrete compiler baseline.
- A workspace-level contract avoids per-crate drift.
- Tooling and contributor onboarding become more predictable when the language edition and compiler floor are both explicit.

Alternatives considered:
- Rely on edition selection without `rust-version`: rejected because it leaves the compiler requirement implicit and easier to misconfigure.

## Risks / Trade-offs

- [CEF rendering integration inside eframe may expose platform-specific event-loop or texture-handling complexity] -> isolate the rendering contract and verify Linux-first behavior before broadening scope.
- [Moving browser state authority into the engine may require touching several crate boundaries at once] -> keep the event model explicit and adapt existing interfaces rather than rewriting crate ownership wholesale.
- [Real workload runs may be slower and flakier than synthetic harness runs] -> define deterministic readiness, timeout, and diagnostics capture rules early so failures remain actionable.
- [Observed memory data may not map cleanly to per-tab attribution in all browser/process cases] -> preserve existing policy thresholds but document attribution confidence and fallback diagnostics where precise mapping is unavailable.
- [Pinning a Rust compiler baseline may require updating local toolchains or CI images] -> document the baseline and adjust developer/CI setup in the same change.

## Migration Plan

1. Introduce the live browser-instance and rendering contracts in the engine/shell/runtime boundary.
2. Integrate the content surface into `apps/browser` and replace placeholder center-panel behavior with live page rendering.
3. Convert navigation, title, loading, history, crash, and shutdown handling to engine-driven event propagation.
4. Connect memory telemetry and failure reporting to observed live runtime behavior.
5. Upgrade the workload harness to drive real browser workloads and capture actual diagnostics.
6. Pin the Rust compiler baseline and update workspace documentation and validation commands.

Rollback strategy:
- Preserve the simulated path behind explicit configuration until the live path is proven stable, so bring-up regressions can be isolated without discarding the existing architecture.
- Keep workload reporting formats stable so synthetic and live runs can be compared during migration.

## Open Questions

- What exact CEF rendering mode and host-surface integration fits best with eframe on the current Linux-first target?
- Which engine events need to be modeled explicitly at the shell boundary versus remaining internal to the engine?
- What minimum diagnostic set defines a successful “real workload” run for the first MVP pass?
- How should the workload harness distinguish engine bugs, site incompatibilities, and expected constrained-memory failures in its output?
- Which exact Rust compiler version should be declared as the workspace minimum for the 2024 edition baseline?
