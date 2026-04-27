## MODIFIED Requirements

### Requirement: Browser provides full MVP chrome
The system SHALL provide a standalone browser interface with address bar, tab strip, navigation controls, window controls, settings access, and a live embedded page surface suitable for a full browser MVP.

#### Scenario: User navigates with browser chrome
- **WHEN** the user enters a URL in the address bar and submits it
- **THEN** the browser navigates the active tab to the requested destination, renders the resulting page in the embedded content surface, and updates visible navigation state

#### Scenario: User switches between tabs
- **WHEN** the user selects a different tab in the tab strip
- **THEN** the browser presents the selected tab as the active tab without losing the prior tab's session state and swaps the visible embedded page surface to the selected live tab

### Requirement: Browser UI reflects tab lifecycle and state
The system SHALL display per-tab state including title, loading state, active selection, tab closure behavior, and engine-originated failure state in the browser UI.

#### Scenario: Tab title changes after navigation
- **WHEN** a page updates the document title for an open tab
- **THEN** the browser UI updates the visible tab label for that tab from engine-reported tab state

#### Scenario: User closes a tab
- **WHEN** the user closes a tab from the browser UI
- **THEN** the browser removes the tab from the UI and disposes of the associated live browser instance through the shell

#### Scenario: Navigation fails for the active tab
- **WHEN** the embedded engine reports that the active tab failed to load or crashed
- **THEN** the browser UI presents the failure state in the visible browser surface and tab chrome without requiring the host to synthesize the result manually

### Requirement: Browser UI surfaces critical memory state
The system SHALL surface critical tab or browser memory conditions in the UI when those conditions require user awareness or action.

#### Scenario: Active tab enters critical memory condition
- **WHEN** the active tab crosses a critical memory threshold during live execution
- **THEN** the browser UI presents a visible indication that the tab is under memory pressure

#### Scenario: Memory-related tab failure occurs
- **WHEN** a tab crashes or is terminated due to suspected memory exhaustion during live execution
- **THEN** the browser UI presents a failure state with enough context for the user to understand what happened
