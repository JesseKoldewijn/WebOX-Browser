## ADDED Requirements

### Requirement: Standalone browser presents real visible chrome
The system SHALL provide a real standalone browser window with visible address bar, tab strip, navigation controls, and window controls rather than only an in-memory browser state model.

#### Scenario: Browser window renders chrome
- **WHEN** the user launches webox
- **THEN** the browser presents a visible window that includes browser chrome suitable for standalone use

#### Scenario: Browser accepts address bar navigation
- **WHEN** the user enters a URL into the visible address bar and submits it
- **THEN** the active browser tab navigates to that destination and the visible chrome reflects the navigation state

### Requirement: Visible browser chrome reflects live tab state
The system SHALL connect the real browser chrome to live browser state so tab creation, selection, loading, title updates, and closure are visible to the user.

#### Scenario: Tab state updates appear in live chrome
- **WHEN** a live browser tab changes title or loading state
- **THEN** the standalone browser chrome updates the visible tab presentation accordingly

#### Scenario: User closes a visible tab
- **WHEN** the user closes a tab from the standalone browser chrome
- **THEN** webox disposes of the associated live browser instance and updates the remaining tab state in the window
