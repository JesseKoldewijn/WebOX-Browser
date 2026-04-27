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
    pub fn capture_recovery_report(&self, telemetry: &TabTelemetry) -> RecoveryReport {
        RecoveryReport {
            tab_id: telemetry.tab_id.clone(),
            suspected_cause: "suspected-memory-exhaustion".to_string(),
            captured_bytes: telemetry.renderer_bytes
                + telemetry.browser_bytes
                + telemetry.gpu_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryController, MemoryPressureLevel, TabTelemetry};

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
}
