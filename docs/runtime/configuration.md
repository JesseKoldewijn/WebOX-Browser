# Runtime Configuration

The Rust workspace uses `webox-config` to define startup configuration, subprocess launch options, and environment paths.

## Current Focus

- Configure a target memory headroom per tab
- Track subprocess executable and logging paths
- Keep development and runtime paths explicit for CEF integration and workload validation

## Validation Outputs

- Workload harness reports are written to `.webox/validation/`
- Summary verification notes are tracked in `validation/reports/`
- The runtime API now exposes browser instance snapshots plus system-level target reporting for harness consumers
