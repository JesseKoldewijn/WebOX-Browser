## MODIFIED Requirements

### Requirement: Browser provides full MVP chrome
The system SHALL provide a real standalone browser interface with visible address bar, tab strip, navigation controls, window controls, and settings access suitable for a full browser MVP.

#### Scenario: User navigates with browser chrome
- **WHEN** the user enters a URL in the visible address bar and submits it
- **THEN** the browser navigates the active live tab to the requested destination and updates visible navigation state

#### Scenario: User switches between tabs
- **WHEN** the user selects a different tab in the visible tab strip
- **THEN** the browser presents the selected live tab as the active tab without losing the prior tab's session state

### Requirement: Browser UI reflects tab lifecycle and state
The system SHALL display live per-tab state including title, loading state, active selection, and tab closure behavior in the browser UI.

#### Scenario: Tab title changes after navigation
- **WHEN** a live page updates the document title for an open tab
- **THEN** the browser UI updates the visible tab label for that tab

#### Scenario: User closes a tab
- **WHEN** the user closes a tab from the browser UI
- **THEN** the browser removes the tab from the visible UI and disposes of the associated live browser instance through the shell

### Requirement: Browser UI surfaces critical memory state
The system SHALL surface critical tab or browser memory conditions in the visible UI when those conditions require user awareness or action.

#### Scenario: Active tab enters critical memory condition
- **WHEN** the active live tab crosses a critical memory threshold
- **THEN** the browser UI presents a visible indication that the tab is under memory pressure

#### Scenario: Memory-related tab failure occurs
- **WHEN** a live tab crashes or is terminated due to suspected memory exhaustion
- **THEN** the browser UI presents a failure state with enough context for the user to understand what happened
