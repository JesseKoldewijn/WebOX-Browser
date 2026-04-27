## Purpose
Define the reusable embeddable runtime surface and how it exposes high-memory behavior to host applications.

## Requirements

### Requirement: Core runtime can be embedded by host applications
The system SHALL expose a reusable runtime surface that allows host applications to embed Chromium-backed browsing capabilities without depending on the standalone browser UI.

#### Scenario: Host application creates embedded browser instance
- **WHEN** a host application initializes the embeddable runtime
- **THEN** it can create and manage a browser instance through runtime APIs without launching the standalone browser chrome

#### Scenario: Embedded host configures runtime behavior
- **WHEN** a host application provides runtime configuration at initialization
- **THEN** the embeddable runtime applies supported configuration values before browser instances are created

### Requirement: Embedded runtime reuses memory policy capabilities
The system SHALL make high-memory monitoring and policy controls available to embedded hosts through the same core runtime used by the standalone browser.

#### Scenario: Embedded host enables memory diagnostics
- **WHEN** a host application enables memory diagnostics for an embedded browser instance
- **THEN** the runtime exposes memory telemetry and memory-event reporting for that instance

#### Scenario: Embedded host receives memory pressure events
- **WHEN** an embedded browser instance crosses configured memory thresholds
- **THEN** the runtime emits an event or callback that allows the host to respond to the condition
