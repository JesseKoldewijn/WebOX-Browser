## MODIFIED Requirements

### Requirement: Embedded engine provides Chromium-class web compatibility
The system SHALL embed Chromium through CEF so that webox renders and executes modern web content with compatibility expectations aligned with Chromium for JavaScript, HTML, CSS, DOM, media, networking, storage, canvas, WebGL, and WebAssembly through live browser instances.

#### Scenario: Standards-based page loads correctly
- **WHEN** a user navigates to a modern standards-based website
- **THEN** the page renders and behaves using Chromium-compatible web platform behavior through a live embedded browser instance

#### Scenario: WebAssembly-heavy application runs in embedded engine
- **WHEN** a user opens a WebAssembly-heavy web application
- **THEN** the embedded engine loads and executes the application using Chromium-compatible runtime support inside a live embedded browser instance

### Requirement: Engine bootstrap supports controlled configuration
The system SHALL allow webox to configure the embedded Chromium runtime at startup with browser-level options required for process management, observability, memory policy, and live browser-surface integration.

#### Scenario: Browser launches with custom runtime settings
- **WHEN** webox starts the embedded engine
- **THEN** it applies configured engine startup settings before creating user-visible browser instances and their host rendering surfaces

#### Scenario: Engine startup failure is diagnosable
- **WHEN** the embedded engine fails to initialize
- **THEN** webox records a structured startup failure signal that identifies the failing component and prevents silent launch failure

### Requirement: Embedded runtime subprocess lifecycle is managed explicitly
The system SHALL configure and manage the browser subprocess lifecycle required by the embedded Chromium runtime, including subprocess executable discovery and launch configuration.

#### Scenario: Browser locates subprocess executable
- **WHEN** webox prepares the embedded Chromium runtime
- **THEN** it resolves the configured subprocess executable path needed for the browser lifecycle

#### Scenario: Subprocess launch configuration is applied
- **WHEN** webox starts the embedded runtime
- **THEN** it applies configured subprocess launch options before browser instances are created

### Requirement: Embedded engine exposes live browser instance events
The system SHALL create live browser instances and expose lifecycle events needed by the standalone browser and embeddable runtime, including navigation, title, loading, failure, and surface-update signals.

#### Scenario: Engine creates live browser instance
- **WHEN** the shell or embeddable runtime requests a new browser instance
- **THEN** the engine creates a live embedded browser instance and returns an identifier that remains stable for that instance lifecycle

#### Scenario: Engine reports lifecycle updates
- **WHEN** a live browser instance changes title, loading status, navigation state, or failure status
- **THEN** the engine emits those updates to host-side consumers rather than requiring the host to synthesize them manually
