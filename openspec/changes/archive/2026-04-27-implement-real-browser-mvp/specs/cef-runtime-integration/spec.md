## ADDED Requirements

### Requirement: Browser runtime boots through real CEF integration
The system SHALL initialize the browser runtime through a real CEF bootstrap flow that creates usable browser instances instead of relying on stubbed engine behavior.

#### Scenario: Browser starts with real embedded engine
- **WHEN** webox launches the standalone browser application
- **THEN** the browser initializes a real CEF-backed runtime before creating user-visible browser windows or tabs

#### Scenario: Embedded engine startup failure is surfaced
- **WHEN** CEF initialization fails during startup
- **THEN** webox records structured diagnostics and prevents silent browser launch failure

### Requirement: CEF subprocess lifecycle is managed explicitly
The system SHALL configure and manage the browser subprocess lifecycle required by CEF, including subprocess executable discovery and launch configuration.

#### Scenario: Browser locates subprocess executable
- **WHEN** webox prepares the CEF runtime
- **THEN** it resolves the configured subprocess executable path needed for the embedded browser lifecycle

#### Scenario: Subprocess launch configuration is applied
- **WHEN** webox starts the embedded runtime
- **THEN** it applies configured subprocess launch options before live browser instances are created
