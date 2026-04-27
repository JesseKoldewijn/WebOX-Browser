## Purpose
Define live validation coverage for representative heavy browser workloads and the diagnostics captured during execution.

## Requirements

### Requirement: Browser executes representative heavy workloads in a live validation harness
The system SHALL provide a validation harness that runs representative heavy workloads inside the real embedded browser, including large data visualization, Unity WebGL, WASM-heavy tools, and modern web application scenarios.

#### Scenario: Validation harness launches workload in browser
- **WHEN** a developer runs the workload validation flow
- **THEN** webox launches the configured workload inside the live embedded browser rather than validating only static fixtures

#### Scenario: Multiple workload categories are covered
- **WHEN** the validation suite executes
- **THEN** it includes representative coverage for the project's target workload categories

### Requirement: Validation captures compatibility and memory diagnostics
The system SHALL capture browser compatibility outcomes and memory-related diagnostics while workloads are executed in the live browser.

#### Scenario: Workload execution records diagnostics
- **WHEN** a live workload runs in the embedded browser
- **THEN** the validation harness records diagnostics relevant to browser behavior, compatibility, and memory pressure

#### Scenario: Constrained systems report unmet target
- **WHEN** the validation harness runs on a system that cannot satisfy the configured high-memory target
- **THEN** the recorded results indicate that the target was not met rather than silently reporting success
