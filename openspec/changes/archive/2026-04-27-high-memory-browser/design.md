## Context

webox is a new browser effort focused on an unusual but deliberate product target: materially higher per-tab memory headroom than mainstream browsers while preserving the real-world compatibility users expect from Chromium-class browsing. The immediate objective is a full browser MVP named webox, with the same core also evolving into an embeddable runtime for host applications that need modern web rendering under heavier memory workloads. The repository will be organized as a monorepo so the browser application, engine/runtime, shared packages, and docs/marketing website can evolve together with clear boundaries.

The current project state began as effectively greenfield, which made early architecture decisions especially important. The browser must support modern JavaScript, HTML, CSS, media, and WebAssembly behavior comparable to Chromium rather than attempting to build a rendering engine from scratch. The selected direction remains to embed Chromium through CEF and build a Rust shell around it for application lifecycle, process orchestration, memory policy, UI integration, and future embedding APIs. This change now captures the completed foundation layer: monorepo structure, Rust workspace scaffolding, shared packages, Astro docs site with SolidJS islands, browser/runtime state models, memory policy primitives, and validation fixtures.

The central technical constraint is that raising "per-tab memory headroom" is not solved by a single compiler or language choice. Chromium imposes several practical ceilings through process architecture, JavaScript engine limits, allocator behavior, OS-level pressure, and product defaults. webox therefore needs an architecture that measures memory, configures Chromium and subprocesses aggressively where possible, isolates failures, and degrades gracefully before the operating system or engine reaches catastrophic OOM behavior.

## Goals / Non-Goals

**Goals:**
- Deliver the browser foundation required to support a later full browser MVP with Chromium-class rendering and web platform support using CEF.
- Organize the codebase as a monorepo with explicit areas for browser apps, engine/runtime code, shared packages, and the docs/marketing website.
- Use Rust for the webox shell, process-control, and memory-management orchestration layers.
- Target at least 8 GB of practical per-tab memory availability on supported systems.
- Reduce avoidable tab crashes through proactive memory pressure monitoring and policy enforcement.
- Keep the browser core reusable so the engine can later be embedded into external host applications.
- Share common configuration, branding, contracts, and developer tooling through reusable packages rather than duplicating logic across apps.
- Preserve a path to deeper engine customization and a real browser UI host later without requiring an immediate Chromium fork.

**Non-Goals:**
- Building a brand new rendering engine, JavaScript engine, CSS engine, or HTML parser.
- Guaranteeing that every workload can exceed 8 GB regardless of OS limits, hardware, or upstream engine constraints.
- Full extension ecosystem compatibility in the first MVP.
- Replacing all Chromium internals with Rust in the first implementation phase.
- Solving distributed sync, accounts, bookmarks sync, or cloud services as part of the initial architecture.
- Building a complex CMS or growth-marketing platform for the website in the first phase.

## Decisions

### 1. Use CEF as the initial Chromium integration layer

webox will use CEF first rather than forking Chromium or building directly against Chromium's content API.

Rationale:
- CEF provides a proven embedding surface with Chromium compatibility and a narrower maintenance burden than a full Chromium fork.
- It allows the project to validate the product thesis quickly: whether a custom process and memory architecture can materially improve per-tab headroom without losing compatibility.
- It keeps open the option to move deeper into Chromium internals later if CEF proves too restrictive.

Alternatives considered:
- Fork Chromium directly: rejected for the MVP because the maintenance and staffing cost is too high for an early-stage project.
- Build against Chromium content API directly: offers more control than CEF, but significantly raises integration complexity before the value hypothesis is validated.
- Build a new engine: rejected because modern web compatibility would take far too long.

### 2. Use Rust for shell, orchestration, and host-side APIs

The browser shell, process manager, memory controller, and embeddable host-facing APIs will be implemented in Rust.

Rationale:
- Rust provides strong memory safety and predictable performance without GC pauses in browser-critical control paths.
- Rust has strong FFI support for interoperating with C and C++ libraries such as CEF.
- The project goal is resilient high-memory operation, so reducing host-side memory safety bugs is strategically valuable.

Alternatives considered:
- C++: best raw alignment with Chromium internals, but weaker safety story for long-term host-side reliability.
- Go: rejected because GC behavior and weaker C++ interop make it less suitable for low-level browser orchestration.
- Zig: promising, but the ecosystem and production maturity for this scale are less proven.

### 3. Treat high-memory support as a policy and observability problem, not only a launch-flag problem

webox will not define success as merely passing larger JavaScript heap flags. It will implement a memory controller that observes tab, renderer, GPU, and host memory pressure and applies policies before catastrophic failure.

Rationale:
- Modern browser memory behavior is distributed across multiple processes and allocators.
- Large workloads can fail due to renderer exhaustion, GPU pressure, fragmentation, or host memory pressure even if a JavaScript heap limit is raised.
- A policy-driven controller creates room for tiered mitigations such as warnings, tab prioritization, renderer restarts, optional suspension of background tabs, or reduced preloading behavior.

Alternatives considered:
- Only set Chromium/V8 flags such as heap-size tuning: insufficient on its own.
- Rely entirely on the OS OOM behavior: unacceptable because it gives poor user experience and weak diagnostics.

### 4. Separate browser responsibilities into core domains

The implementation will be organized into distinct domains:
- `shell`: native app lifecycle, windows, commands, and integration boundaries
- `engine`: CEF process/bootstrap bindings and browser instance management
- `memory`: telemetry, budgeting, threshold evaluation, policy execution, crash/OOM reporting
- `ui`: browser chrome and user-visible controls
- `runtime-api`: embeddable host surface and configuration

Rationale:
- Clear domain boundaries reduce coupling between browser UX and engine mechanics.
- The embeddable runtime becomes easier to expose if shell/UI concerns are isolated from engine/core concerns.
- The memory system can evolve independently as the main differentiator of webox.

Alternatives considered:
- Single monolithic application layer: easier at the start but likely to slow later engine/runtime reuse.

### 5. Use a monorepo with app and package boundaries from the start

webox will be structured as a monorepo with top-level areas for user-facing applications, reusable packages, and project websites. The initial repository shape should support at least:
- `apps/browser` for the standalone browser application
- `apps/docs` or `apps/site` for the docs and marketing website
- `crates/engine` or equivalent Rust workspace areas for engine/runtime modules
- `packages/*` for shared assets such as config, schema definitions, branding, docs helpers, or frontend utilities when needed

Rationale:
- The project already spans multiple deliverables: standalone browser, embeddable runtime, shared modules, and a site.
- A monorepo improves consistency for tooling, versioning, CI, and shared design or configuration assets.
- It prevents the docs and marketing site from drifting away from the product architecture and developer onboarding story.

Alternatives considered:
- Separate repositories per surface: rejected because it increases coordination overhead too early.
- Single app-first repository with ad hoc folders: rejected because package boundaries become harder to enforce later.

### 6. Full browser MVP includes modern browser chrome, but this change only builds the foundation for it

The intended MVP will include tabs, address bar, navigation controls, settings, and browser windows, but this change stops at the shell-facing state model, command routing, and architecture needed to support that UI. The real browser chrome implementation is moved into a follow-up change alongside real CEF embedding.

Rationale:
- The user explicitly wants a full browser MVP, not a single-tab demo.
- A complete browser shell remains necessary to validate actual tab memory behavior and user workflows, but the current foundation work can proceed without locking the UI host too early.
- Some features such as broad extension support can be deferred without weakening the core experiment.

Alternatives considered:
- Single-tab or minimal-shell proof of concept: faster, but under-validates the browser product direction.

### 7. Define compatibility around Chromium-class behavior for modern web workloads

Capability and validation work will focus on correct behavior for JavaScript, HTML, CSS, DOM APIs, media, networking, storage, canvas, WebGL, and WebAssembly workloads expected to run in Chromium-class browsers.

Rationale:
- The project differentiator is memory headroom, not alternate standards behavior.
- Users targeting Unity WebGL, heavy web apps, and large visualizations need compatibility more than novelty.

Alternatives considered:
- Limit initial support to select workloads only: rejected because it would weaken the product premise of being a browser users can rely on broadly.

## Risks / Trade-offs

- [CEF may expose insufficient control over some memory ceilings] -> Start with CEF for speed, but preserve an architectural path toward deeper Chromium integration if validation shows hard blockers.
- [8 GB per-tab may be constrained by upstream V8, renderer, or OS realities] -> Frame the requirement as supported-system headroom with explicit observability, diagnostics, and fallback behavior rather than an unconditional guarantee.
- [High-memory tabs can starve the rest of the system] -> Add policy thresholds, user-visible warnings, configurable ceilings, and background-tab protections.
- [A full browser MVP increases scope substantially] -> Separate the work into a completed foundation phase and a follow-up implementation phase for real CEF embedding, live browser chrome, and end-to-end workload validation.
- [Rust and CEF integration complexity can slow initial progress] -> Keep binding layers narrow, isolate unsafe FFI, and defer nonessential abstractions.
- [Embeddable runtime and standalone browser goals can pull architecture in different directions] -> Maintain domain separation so host APIs sit on core runtime primitives rather than UI-specific logic.
- [Monorepo scope can sprawl across app, engine, packages, and site work] -> Define clear workspace ownership, boundaries, and shared-package rules up front.
- [Docs/marketing work can distract from core engine progress] -> Keep the initial site focused on product positioning, architecture communication, and onboarding rather than broad content ambitions.

## Migration Plan

Because this is a greenfield initiative, migration is primarily a phased delivery plan rather than a production replacement strategy.

1. Stand up the monorepo layout, workspace tooling, and shared-package boundaries.
2. Stand up the Rust application shell, shared runtime models, and engine integration boundaries.
3. Establish tab lifecycle and browser-state instrumentation hooks.
4. Add memory telemetry, budgeting, threshold enforcement, and recovery scaffolding.
5. Build the docs/marketing site with Astro and SolidJS, tied to the shared workspace.
6. Expose embeddable runtime APIs and shared packages that future browser and site surfaces can reuse.
7. Carry real CEF embedding, live browser chrome, and end-to-end workload validation into a dedicated follow-up change.

Rollback strategy:
- Keep memory-policy features configurable so aggressive experiments can be disabled without removing the browser shell.
- Treat deeper Chromium integration as a later escalation path only if the CEF-based architecture fails the validation criteria.

## Open Questions

- What exact workload suite will define success for the 8 GB per-tab target, and on which operating systems and hardware tiers?
- How much of Chromium's existing multi-process model can be tuned through CEF versus requiring deeper upstream customization?
- Which UI host should power the real browser chrome in the follow-up change: native toolkit, GPU UI layer, or Rust desktop framework?
- Which allocator and telemetry stack should be preferred for memory introspection across Rust and Chromium subprocesses?
- What extension, devtools, and debugging support thresholds are required for the first browser MVP?
- The docs and marketing site is now established at `apps/docs` with Astro and SolidJS islands; what additional shared design-token or content infrastructure should move into `packages/*` next?

## Current Execution State

The following foundation work is complete in this change:

- Monorepo layout for browser app, docs site, Rust crates, shared packages, and validation fixtures
- Rust workspace crates for shell, engine boundary, memory policy, UI state, runtime API, and configuration
- Shared branding and contract packages consumed by the docs site
- Astro docs/marketing site with SolidJS used for interactive workload exploration
- Browser state models, shell command routing, startup and shutdown diagnostics, memory thresholds, and smoke tests
- Static validation fixtures for representative modern web workloads

The following work is intentionally deferred to a follow-up change:

- Real CEF bootstrap and subprocess lifecycle wiring
- Real standalone browser chrome implementation
- Live workload execution against the embedded browser
- End-to-end verification of high-memory behavior and diagnostics
