# Validation Plan

This directory contains workload fixtures and plans for validating Chromium-class behavior plus high-memory browser diagnostics.

## Workload Categories

- Large DOM and CSS stress pages
- Canvas and WebGL rendering workloads
- Worker and WebAssembly memory pressure fixtures
- Large data visualization datasets

## Current Status

- Fixtures created for workload execution in the harness
- `webox-workload-harness` now runs representative workload categories against the runtime and records compatibility plus memory diagnostics
- Validation reports live under `validation/reports/` and generated run output is written under `.webox/validation/`
