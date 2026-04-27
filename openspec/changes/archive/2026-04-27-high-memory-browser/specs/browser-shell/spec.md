## ADDED Requirements

### Requirement: Rust host shell manages browser lifecycle
The system SHALL provide a Rust-based host shell that initializes the browser runtime, manages application lifecycle, and coordinates browser windows and tab containers for webox.

#### Scenario: Application startup initializes runtime
- **WHEN** the user launches webox
- **THEN** the Rust host shell initializes the required browser runtime components and opens a usable browser window

#### Scenario: Application shutdown closes browser cleanly
- **WHEN** the user exits webox
- **THEN** the Rust host shell shuts down browser processes and releases owned runtime resources without leaving orphaned browser subprocesses

### Requirement: Shell exposes browser commands to the UI layer
The system SHALL expose commands for navigation, reload, back, forward, new tab creation, tab closure, and browser window management through host-side interfaces owned by the shell.

#### Scenario: UI requests navigation command
- **WHEN** the UI layer requests navigation to a URL
- **THEN** the shell routes the command to the appropriate browser instance and updates the targeted tab

#### Scenario: UI requests tab creation
- **WHEN** the UI layer requests a new tab
- **THEN** the shell creates a new browser-backed tab container and returns a stable identifier for that tab
