## MODIFIED Requirements

### Requirement: Core runtime can be embedded by host applications
The system SHALL expose a reusable runtime surface that allows host applications to embed Chromium-backed browsing capabilities without depending on the standalone browser UI, and SHALL back those capabilities with live browser instances rather than simulated-only state.

#### Scenario: Host application creates embedded browser instance
- **WHEN** a host application initializes the embeddable runtime
- **THEN** it can create and manage a live browser instance through runtime APIs without launching the standalone browser chrome

#### Scenario: Embedded host configures runtime behavior
- **WHEN** a host application provides runtime configuration at initialization
- **THEN** the embeddable runtime applies supported configuration values before live browser instances are created

### Requirement: Embedded runtime reuses memory policy capabilities
The system SHALL make high-memory monitoring and policy controls available to embedded hosts through the same core runtime used by the standalone browser.

#### Scenario: Embedded host enables memory diagnostics
- **WHEN** a host application enables memory diagnostics for an embedded browser instance
- **THEN** the runtime exposes memory telemetry and memory-event reporting for that live instance

#### Scenario: Embedded host receives memory pressure events
- **WHEN** an embedded browser instance crosses configured memory thresholds during live execution
- **THEN** the runtime emits an event or callback that allows the host to respond to the condition

### Requirement: Embedded runtime exposes live navigation and failure state
The system SHALL expose live navigation, loading, title, and failure state for embedded browser instances through runtime APIs.

#### Scenario: Embedded host observes page lifecycle
- **WHEN** an embedded browser instance navigates or finishes loading a page
- **THEN** the runtime exposes the resulting title, loading state, and resolved navigation state to the host

#### Scenario: Embedded host observes runtime failure
- **WHEN** an embedded browser instance crashes, fails navigation, or is terminated under pressure
- **THEN** the runtime exposes that failure state and related diagnostics to the host
