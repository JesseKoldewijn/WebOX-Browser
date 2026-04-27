## Context

The `high-memory-browser` change completed the webox foundation layer: monorepo structure, Rust workspace scaffolding, shared packages, Astro docs site, browser-state models, memory policy primitives, runtime API scaffolding, and validation fixtures. What remains is the work that turns those foundations into a real browser: actual CEF embedding, a real standalone browser chrome, and live workload validation executed inside the embedded engine.

This follow-up change is where webox stops being a scaffolded architecture and becomes a functioning browser prototype. The browser must launch a real CEF-backed renderer, create actual browser windows and tabs, present usable chrome to the user, and run representative workloads so the memory-target and compatibility claims can be validated with real evidence.

The biggest constraints are practical rather than conceptual. Real CEF integration requires distribution strategy, subprocess wiring, startup sequencing, and platform-aware configuration. Real browser chrome requires a host UI decision that can present windows, navigation controls, and tab state while staying aligned with the Rust-centric architecture. Live validation requires a harness that can launch representative workloads, collect diagnostics, and prove behavior under both supported and constrained memory conditions.

## Goals / Non-Goals

**Goals:**
- Replace the current stubbed engine boundary with real CEF bootstrap and subprocess lifecycle management.
- Implement a real standalone browser chrome that exposes windows, address bar, tab strip, navigation controls, and live browser state.
- Connect shell, runtime, UI, and memory systems to actual embedded browser instances.
- Run representative workloads inside the real browser and capture diagnostics about compatibility and memory behavior.
- Verify behavior for systems that can satisfy the configured high-memory target and systems that cannot.

**Non-Goals:**
- Replacing Chromium or CEF with a different engine in this phase.
- Building a full extension platform or complete browser ecosystem parity in this phase.
- Reworking the monorepo foundation, shared package strategy, or Astro site architecture unless necessary for the implementation.
- Guaranteeing universal 8 GB per-tab behavior on unsupported hardware or operating systems.

## Decisions

### 1. Use the completed foundation change as the contract for real implementation

This change will build on the Rust shell, memory controller, runtime API, and UI state already created rather than replacing them wholesale.

Rationale:
- The completed foundation already defines the intended boundaries between shell, engine, UI, memory, and runtime API.
- Reusing those boundaries reduces churn and keeps the implementation focused on substituting real behavior for stubs.
- It preserves traceability from foundation planning into actual implementation.

Alternatives considered:
- Rewrite the foundation from scratch during implementation: rejected because it would blur responsibilities and waste the completed scaffold work.

### 2. Treat CEF integration as a product dependency, not just a code dependency

This change will explicitly decide how CEF is obtained, versioned, configured, and launched, including subprocess binaries and platform-specific runtime setup.

Rationale:
- Real browser execution depends on more than bindings; it also depends on shipping and locating native runtime assets correctly.
- The implementation needs a repeatable development path before workload validation can be trusted.
- Failure to make this explicit would push critical integration work into ad hoc scripts and undocumented environment assumptions.

Alternatives considered:
- Add CEF piecemeal during coding without a defined distribution strategy: rejected because it increases fragility and slows onboarding.

### 3. Choose a real browser chrome host before broad UI implementation

This change will select a concrete UI host strategy for standalone browser chrome and then implement the real address bar, tab strip, navigation controls, and window shell on top of it.

Rationale:
- The current state model is useful, but it does not answer how actual browser windows are rendered and managed.
- A deliberate host choice is required to keep the browser shell, engine, and UI integration coherent.
- A host decision affects window lifecycle, event routing, rendering model, and packaging.

Alternatives considered:
- Continue adding abstract UI state without selecting a host: rejected because the next work requires visible browser chrome and real event handling.

### 4. Validate live workloads with an explicit harness

This change will define and run a workload harness that drives real browser pages and captures compatibility and memory diagnostics.

Rationale:
- The project's differentiator is not abstract architecture; it is whether real heavy workloads behave better in practice.
- A live harness provides reproducible validation and prevents claims from depending on manual spot checks.
- The same harness can grow into regression coverage later.

Alternatives considered:
- Continue with only static fixtures or manual checks: rejected because they are insufficient for validating an embedded browser product.

### 5. Keep high-memory verification tied to observable diagnostics

This change will not treat memory verification as a pass/fail based only on raw allocation. It will validate pressure handling, degradation behavior, and diagnostics when targets are met or missed.

Rationale:
- The design goal is resilient browser behavior under pressure, not only a larger number in configuration.
- Users need both performance and understandable failure modes.
- Diagnostic visibility is necessary to compare supported and constrained systems meaningfully.

Alternatives considered:
- Focus only on peak-memory experiments: rejected because they under-represent user-facing browser behavior.

## Risks / Trade-offs

- [CEF packaging and subprocess setup may vary significantly by platform] -> Establish a clear initial platform target and document asset/bootstrap expectations early.
- [The selected UI host may constrain future browser chrome evolution] -> Evaluate the host choice against actual browser-window and tab-management needs before implementing broad UI surfaces.
- [Real workload validation may reveal gaps in the current shell or runtime boundaries] -> Limit architectural changes to targeted adjustments and keep the foundation boundary model intact unless a clear blocker emerges.
- [Live browser validation can consume time without yielding actionable results if the harness is weak] -> Define concrete workload scenarios and expected telemetry before broad test execution.
- [Memory-target claims may still be bounded by upstream engine realities] -> Validate supported-system behavior and make fallback/diagnostic outcomes part of the success criteria.

## Migration Plan

1. Select and integrate the real CEF distribution and binding strategy.
2. Replace stub engine startup with actual CEF bootstrap and subprocess lifecycle wiring.
3. Choose the standalone browser UI host and implement real browser chrome on top of the current shell/runtime boundaries.
4. Connect live browser instances, tab state, and memory telemetry end to end.
5. Build and run the workload harness against representative heavy scenarios.
6. Document observed behavior for systems that satisfy and fail the configured memory target.

Rollback strategy:
- Preserve the current foundation interfaces while implementing real integration, so regressions can fall back to the scaffolded state model if needed during development.
- Keep workload validation infrastructure isolated from the main browser runtime so failed experiments can be adjusted without discarding core browser work.

## Open Questions

- Which UI host should power the standalone browser chrome in this phase?
- Which initial operating system should be treated as the primary bring-up target for real CEF integration?
- Which CEF binding or integration strategy best fits the current Rust workspace structure?
- What exact workload list will serve as the first pass/fail validation suite for live browser execution?
- What level of DevTools support is required before the browser is considered usable for debugging heavy workloads?
