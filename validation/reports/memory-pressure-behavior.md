# Memory Pressure Behavior

This note records the intended interpretation of live workload execution once the harness runs.

## Pressure Ladder

- `Normal`: continue observing the tab with no user-visible warning.
- `Warning`: surface a memory warning for the tab in the visible browser chrome.
- `Critical`: keep the tab alive when possible while warning the user and deprioritizing background work.
- `Exhausted`: mark the tab as at memory exhaustion risk, capture a recovery report, and preserve enough diagnostics to explain the failure path.

## Mitigation Actions

- User-visible warning through tab memory indicator.
- Background work deprioritization when pressure crosses critical thresholds.
- Recovery-report capture when simulated exhaustion occurs.

## Recovery Diagnostics

- Browser instance id
- Total memory bytes at the point of escalation
- Human-readable indicator shown in the UI
- Failure state text for suspected memory exhaustion
- Memory attribution detail showing whether the result came from observed telemetry or fallback aggregated metrics
- Harness category and workload source for correlation
