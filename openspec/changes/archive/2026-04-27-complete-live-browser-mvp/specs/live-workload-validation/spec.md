## MODIFIED Requirements

### Requirement: Browser executes representative heavy workloads in a live validation harness
The system SHALL provide a validation harness that runs representative heavy workloads inside the real embedded browser, including large data visualization, Unity WebGL, WASM-heavy tools, and modern web application scenarios.

#### Scenario: Validation harness launches workload in browser
- **WHEN** a developer runs the workload validation flow
- **THEN** webox launches the configured workload inside the live embedded browser rather than validating only static fixtures or synthetic state transitions

#### Scenario: Multiple workload categories are covered
- **WHEN** the validation suite executes
- **THEN** it includes representative coverage for the project's target workload categories

### Requirement: Validation captures compatibility and memory diagnostics
The system SHALL capture browser compatibility outcomes and memory-related diagnostics while workloads are executed in the live browser.

#### Scenario: Workload execution records diagnostics
- **WHEN** a live workload runs in the embedded browser
- **THEN** the validation harness records diagnostics relevant to browser behavior, compatibility, navigation outcome, and memory pressure

#### Scenario: Constrained systems report unmet target
- **WHEN** the validation harness runs on a system that cannot satisfy the configured high-memory target
- **THEN** the recorded results indicate that the target was not met rather than silently reporting success

### Requirement: Validation classifies real runtime outcomes
The system SHALL distinguish successful workload execution, compatibility failure, engine failure, and constrained-memory outcomes in validation output.

#### Scenario: Workload fails because of browser incompatibility
- **WHEN** a workload launches but does not behave correctly due to browser compatibility limitations
- **THEN** the validation output records the run as a compatibility issue rather than a generic harness failure

#### Scenario: Workload fails because of engine or host failure
- **WHEN** a workload run is interrupted by engine startup failure, browser crash, or host-surface failure
- **THEN** the validation output records the run as an engine or host failure with related diagnostics
