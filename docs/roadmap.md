# MVP Gaps And Follow-Up Work

This document captures the known follow-up work after the initial monorepo scaffolding and Rust browser prototype.

## Still Pending

- Complete real CEF-backed page rendering instead of the current tracked live-instance contract.
- Expand workload execution from simulated representative scenarios to true live browser content coverage across more platforms.
- Deepen embeddable runtime APIs for richer external control and observability.
- Evaluate deeper Chromium integration if CEF prevents meeting memory or observability goals.
- Decide how far extension and DevTools support must go in the first browser MVP.

## Near-Term Priorities

1. Replace simulated browser content execution with fully rendered embedded CEF surfaces.
2. Expand workload validation to cover more real-world hosted applications and platform variants.
3. Add stronger crash, OOM, and smoke automation around the live browser MVP.
4. Evaluate follow-up compatibility, DevTools, and platform expansion work.
