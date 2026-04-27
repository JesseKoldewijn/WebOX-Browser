## MODIFIED Requirements

### Requirement: Workspace tooling supports multi-surface development
The system SHALL provide workspace-level tooling or conventions that support developing, building, and validating the browser, engine/runtime modules, shared packages, and docs or marketing site within one repository, including an explicit Rust 2024 toolchain baseline shared across workspace crates.

#### Scenario: Developer runs workspace setup
- **WHEN** a developer follows repository setup steps
- **THEN** they can prepare the required toolchains and dependencies for the repository's defined workspaces, including the Rust compiler baseline required by the workspace

#### Scenario: Developer targets a specific workspace surface
- **WHEN** a developer runs a build or development command for a specific workspace area
- **THEN** the repository provides a clear path to run that workflow without requiring unrelated surfaces to be launched first

#### Scenario: Contributor validates Rust toolchain compatibility
- **WHEN** a contributor uses the repository Rust toolchain for any workspace crate
- **THEN** the workspace manifests and documentation make the required Rust 2024 compiler baseline explicit and consistent across crates
