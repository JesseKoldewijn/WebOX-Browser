use std::fs;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryThresholds {
    pub warning_bytes: u64,
    pub critical_bytes: u64,
    pub hard_limit_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabTelemetry {
    pub tab_id: String,
    pub renderer_bytes: u64,
    pub browser_bytes: u64,
    pub gpu_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetrySource {
    SyntheticSample,
    LinuxProcessRss,
    AggregateProcessRss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributionConfidence {
    TestOnly,
    Aggregate,
    ProcessObserved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryAttribution {
    pub source: TelemetrySource,
    pub confidence: AttributionConfidence,
    pub live_mvp_evidence: bool,
    pub detail: String,
}

impl MemoryAttribution {
    #[must_use]
    pub fn synthetic() -> Self {
        Self {
            source: TelemetrySource::SyntheticSample,
            confidence: AttributionConfidence::TestOnly,
            live_mvp_evidence: false,
            detail: "injected memory sample for tests or simulated development".to_string(),
        }
    }

    #[must_use]
    pub fn aggregate_process(process_count: usize) -> Self {
        Self {
            source: TelemetrySource::AggregateProcessRss,
            confidence: AttributionConfidence::Aggregate,
            live_mvp_evidence: true,
            detail: format!(
                "observed aggregate Linux RSS across {process_count} browser-related process(es)"
            ),
        }
    }

    #[must_use]
    pub fn label(&self) -> String {
        format!(
            "source={:?}; confidence={:?}; live_mvp_evidence={}; {}",
            self.source, self.confidence, self.live_mvp_evidence, self.detail
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessMemoryObservation {
    pub pid: u32,
    pub command: String,
    pub rss_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedMemoryTelemetry {
    pub telemetry: TabTelemetry,
    pub attribution: MemoryAttribution,
    pub processes: Vec<ProcessMemoryObservation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    Normal,
    Warning,
    Critical,
    Exhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MitigationAction {
    Observe,
    WarnUser,
    DeprioritizeBackgroundWork,
    CaptureRecoveryReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryEvent {
    pub tab_id: String,
    pub level: MemoryPressureLevel,
    pub total_bytes: u64,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDecision {
    pub event: MemoryEvent,
    pub actions: Vec<MitigationAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupportedSystemReport {
    pub target_bytes: u64,
    pub available_bytes: u64,
    pub meets_target: bool,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryReport {
    pub tab_id: String,
    pub suspected_cause: String,
    pub captured_bytes: u64,
    pub attribution: MemoryAttribution,
    pub process_count: usize,
}

pub struct MemoryController {
    thresholds: MemoryThresholds,
    target_headroom_bytes: u64,
}

impl MemoryController {
    #[must_use]
    pub fn new(target_headroom_bytes: u64) -> Self {
        Self {
            thresholds: MemoryThresholds {
                warning_bytes: target_headroom_bytes * 3 / 4,
                critical_bytes: target_headroom_bytes * 9 / 10,
                hard_limit_bytes: target_headroom_bytes,
            },
            target_headroom_bytes,
        }
    }

    #[must_use]
    pub fn thresholds(&self) -> &MemoryThresholds {
        &self.thresholds
    }

    #[must_use]
    pub fn system_report(&self, available_bytes: u64) -> SupportedSystemReport {
        SupportedSystemReport {
            target_bytes: self.target_headroom_bytes,
            available_bytes,
            meets_target: available_bytes >= self.target_headroom_bytes,
            summary: if available_bytes >= self.target_headroom_bytes {
                "System can support the configured high-memory tab target".to_string()
            } else {
                "System memory is below the configured high-memory tab target".to_string()
            },
        }
    }

    #[must_use]
    pub fn evaluate(&self, telemetry: &TabTelemetry) -> PolicyDecision {
        let total_bytes = telemetry.renderer_bytes + telemetry.browser_bytes + telemetry.gpu_bytes;
        let level = if total_bytes >= self.thresholds.hard_limit_bytes {
            MemoryPressureLevel::Exhausted
        } else if total_bytes >= self.thresholds.critical_bytes {
            MemoryPressureLevel::Critical
        } else if total_bytes >= self.thresholds.warning_bytes {
            MemoryPressureLevel::Warning
        } else {
            MemoryPressureLevel::Normal
        };

        let actions = match level {
            MemoryPressureLevel::Normal => vec![MitigationAction::Observe],
            MemoryPressureLevel::Warning => vec![MitigationAction::WarnUser],
            MemoryPressureLevel::Critical => {
                vec![
                    MitigationAction::WarnUser,
                    MitigationAction::DeprioritizeBackgroundWork,
                ]
            }
            MemoryPressureLevel::Exhausted => vec![
                MitigationAction::WarnUser,
                MitigationAction::DeprioritizeBackgroundWork,
                MitigationAction::CaptureRecoveryReport,
            ],
        };

        PolicyDecision {
            event: MemoryEvent {
                tab_id: telemetry.tab_id.clone(),
                level,
                total_bytes,
                summary: format!(
                    "Tab '{}' is at {:?} memory pressure",
                    telemetry.tab_id, level
                ),
            },
            actions,
        }
    }

    #[must_use]
    pub fn capture_recovery_report(
        &self,
        telemetry: &TabTelemetry,
        attribution: MemoryAttribution,
    ) -> RecoveryReport {
        RecoveryReport {
            tab_id: telemetry.tab_id.clone(),
            suspected_cause: "suspected-memory-exhaustion".to_string(),
            captured_bytes: telemetry.renderer_bytes
                + telemetry.browser_bytes
                + telemetry.gpu_bytes,
            process_count: 0,
            attribution,
        }
    }
}

/// Minimum time between full `/proc` scans. The result is reused for
/// callers that arrive within this window.
const MEMORY_CACHE_TTL: Duration = Duration::from_secs(2);

pub struct LinuxProcessMemoryCollector {
    /// Cached result from the most recent `/proc` scan.
    cached: Option<ObservedMemoryTelemetry>,
    /// When the cached result was last populated.
    last_collected: Option<Instant>,
}

impl LinuxProcessMemoryCollector {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cached: None,
            last_collected: None,
        }
    }

    pub fn collect_for_tab(&mut self, tab_id: &str) -> ObservedMemoryTelemetry {
        // Return cached telemetry if it is still fresh. Replace the tab_id in
        // the cached result since the caller may differ from the original scan.
        if let (Some(cached), Some(last)) = (&self.cached, self.last_collected) {
            if last.elapsed() < MEMORY_CACHE_TTL {
                let mut fresh = cached.clone();
                fresh.telemetry.tab_id = tab_id.to_string();
                return fresh;
            }
        }

        let mut processes = self.browser_processes();
        if processes.is_empty() {
            processes.push(self.current_process_observation());
        }

        let total_rss = processes.iter().map(|process| process.rss_bytes).sum();
        let result = ObservedMemoryTelemetry {
            telemetry: TabTelemetry {
                tab_id: tab_id.to_string(),
                renderer_bytes: total_rss,
                browser_bytes: 0,
                gpu_bytes: 0,
            },
            attribution: MemoryAttribution::aggregate_process(processes.len()),
            processes,
        };

        self.cached = Some(result.clone());
        self.last_collected = Some(Instant::now());
        result
    }

    fn browser_processes(&self) -> Vec<ProcessMemoryObservation> {
        let Ok(entries) = fs::read_dir("/proc") else {
            return Vec::new();
        };

        let mut observations = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let pid = entry.file_name().to_string_lossy().parse::<u32>().ok()?;
                let command = fs::read_to_string(format!("/proc/{pid}/cmdline"))
                    .ok()
                    .map(|value| value.replace('\0', " "))
                    .unwrap_or_default();
                let command_lower = command.to_ascii_lowercase();
                if !command_lower.contains("webox") && !command_lower.contains("cef") {
                    return None;
                }
                let rss_bytes = read_proc_status_rss_bytes(pid).unwrap_or_default();
                Some(ProcessMemoryObservation {
                    pid,
                    command: command.trim().to_string(),
                    rss_bytes,
                })
            })
            .collect::<Vec<_>>();
        observations.sort_by_key(|process| process.pid);
        observations
    }

    fn current_process_observation(&self) -> ProcessMemoryObservation {
        let pid = std::process::id();
        ProcessMemoryObservation {
            pid,
            command: "current webox process".to_string(),
            rss_bytes: read_proc_status_rss_bytes(pid).unwrap_or_default(),
        }
    }
}

impl Default for LinuxProcessMemoryCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn read_proc_status_rss_bytes(pid: u32) -> Option<u64> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status.lines().find_map(|line| {
        let rest = line.strip_prefix("VmRSS:")?;
        let kib = rest.split_whitespace().next()?.parse::<u64>().ok()?;
        Some(kib * 1024)
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AttributionConfidence, LinuxProcessMemoryCollector, MemoryAttribution, MemoryController,
        MemoryPressureLevel, TabTelemetry, TelemetrySource,
    };

    #[test]
    fn controller_escalates_to_critical_pressure() {
        let controller = MemoryController::new(100);
        let decision = controller.evaluate(&TabTelemetry {
            tab_id: "tab-1".to_string(),
            renderer_bytes: 70,
            browser_bytes: 15,
            gpu_bytes: 10,
        });
        assert_eq!(decision.event.level, MemoryPressureLevel::Critical);
    }

    #[test]
    fn linux_collector_observes_current_process_memory() {
        let mut collector = LinuxProcessMemoryCollector::new();
        let observed = collector.collect_for_tab("tab-1");

        assert_eq!(observed.telemetry.tab_id, "tab-1");
        assert!(observed.attribution.live_mvp_evidence);
        assert!(!observed.processes.is_empty());
    }

    #[test]
    fn attribution_metadata_distinguishes_synthetic_from_observed() {
        let synthetic = MemoryAttribution::synthetic();
        let observed = MemoryAttribution::aggregate_process(2);

        assert_eq!(synthetic.source, TelemetrySource::SyntheticSample);
        assert_eq!(synthetic.confidence, AttributionConfidence::TestOnly);
        assert!(!synthetic.live_mvp_evidence);
        assert_eq!(observed.source, TelemetrySource::AggregateProcessRss);
        assert_eq!(observed.confidence, AttributionConfidence::Aggregate);
        assert!(observed.live_mvp_evidence);
        assert!(observed.label().contains("live_mvp_evidence=true"));
    }

    #[test]
    fn linux_collector_returns_cached_result_within_ttl() {
        let mut collector = LinuxProcessMemoryCollector::new();
        let first = collector.collect_for_tab("tab-1");
        // Second call within the 2 s TTL must return the same process list
        // (only the tab_id field is overridden by the caller).
        let second = collector.collect_for_tab("tab-2");

        // Process observations are identical — no second /proc scan occurred.
        assert_eq!(first.processes, second.processes);
        // The returned tab_id reflects the caller's argument, not the cached one.
        assert_eq!(second.telemetry.tab_id, "tab-2");
    }

    #[test]
    fn linux_collector_cache_is_invalidated_after_ttl() {
        use crate::MEMORY_CACHE_TTL;
        let mut collector = LinuxProcessMemoryCollector::new();
        collector.collect_for_tab("tab-1");

        // Artificially age the cache past the TTL.
        collector.last_collected = Some(
            std::time::Instant::now() - MEMORY_CACHE_TTL - std::time::Duration::from_millis(1),
        );

        // A stale cache should trigger a fresh /proc scan; result is still valid.
        let fresh = collector.collect_for_tab("tab-1");
        assert!(!fresh.processes.is_empty());
        // Cache timestamp must have been refreshed.
        assert!(
            collector
                .last_collected
                .is_some_and(|t| t.elapsed() < MEMORY_CACHE_TTL)
        );
    }
}
