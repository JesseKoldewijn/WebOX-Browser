## 1. CEF bring-up

- [x] 1.1 Select the initial CEF distribution, versioning strategy, and Rust integration approach for the primary development platform
- [x] 1.2 Add repository configuration and scripts for locating or provisioning the required CEF runtime assets and subprocess binaries
- [x] 1.3 Replace the stub engine startup path with a real CEF bootstrap flow that initializes the embedded runtime

## 2. Subprocess lifecycle and diagnostics

- [x] 2.1 Implement subprocess executable discovery and launch configuration for CEF child processes
- [x] 2.2 Wire startup and shutdown diagnostics to the real engine lifecycle so CEF failures are observable
- [x] 2.3 Verify the standalone browser app can launch and shut down a real embedded browser instance cleanly

## 3. Browser chrome host selection and setup

- [x] 3.1 Choose the standalone browser UI host strategy for windows and browser chrome
- [x] 3.2 Create the host-level window shell that can render browser chrome and receive browser state updates
- [x] 3.3 Document the chosen browser chrome host approach and how it integrates with the existing Rust workspace

## 4. Standalone browser chrome

- [x] 4.1 Implement a visible address bar, tab strip, navigation controls, and window controls in the standalone browser
- [x] 4.2 Connect live tab selection, title updates, loading state, and tab closure to the visible browser chrome
- [x] 4.3 Verify user actions in the visible browser chrome drive the live browser shell and engine correctly

## 5. Live browser integration

- [x] 5.1 Connect shell tab creation and navigation flows to real embedded browser instances instead of placeholders
- [x] 5.2 Route memory telemetry and critical memory indicators from live browser processes into the visible browser UI
- [x] 5.3 Ensure embeddable runtime APIs operate against the real engine lifecycle and browser instances

## 6. Workload harness

- [x] 6.1 Build a workload harness that launches representative scenarios inside the live embedded browser
- [x] 6.2 Add workload entries for large data visualization, Unity WebGL, WASM-heavy tools, and modern heavy web apps
- [x] 6.3 Capture compatibility and runtime diagnostics for each workload execution

## 7. High-memory verification

- [x] 7.1 Run workload scenarios on systems that satisfy the configured high-memory target and record outcomes
- [x] 7.2 Run workload scenarios on constrained systems and verify unmet-target diagnostics are surfaced clearly
- [x] 7.3 Document observed memory-pressure behavior, mitigation actions, and recovery diagnostics from live execution

## 8. Hardening and follow-up docs

- [x] 8.1 Add automated smoke coverage for real engine bring-up and visible browser chrome flows where practical
- [x] 8.2 Update developer documentation for real CEF setup, browser chrome host usage, and workload validation commands
- [x] 8.3 Review remaining MVP gaps after live browser bring-up and capture follow-up work for deeper compatibility or platform expansion
