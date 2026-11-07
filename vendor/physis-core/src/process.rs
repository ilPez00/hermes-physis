//! Process semantics for industrial, operational, and reasoning workflows.
//!
//! Provides first-class models for:
//!   - Goal, Plan, Task, Process
//!   - Event, State, Transition
//!   - Resource, Dependency, Constraint
//!   - Measurement, Outcome, Failure, Intervention
//!
//! Bridges the loop:
//!   PLAN → ACTION → OBSERVATION → DEVIATION → INTERVENTION → OUTCOME

use serde::{Deserialize, Serialize};

use crate::models::Score;
use crate::temporal::TemporalValidity;

/// Operational or strategic goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessGoal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub target_metric: Option<String>,
    pub target_value: Option<f64>,
    pub achieved: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A structured plan containing tasks and dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPlan {
    pub id: String,
    pub goal_id: Option<String>,
    pub name: String,
    pub tasks: Vec<ProcessTask>,
    pub constraints: Vec<ProcessConstraint>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A task within a process or plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTask {
    pub id: String,
    pub title: String,
    pub state: TaskState,
    pub assigned_resources: Vec<String>,
    pub dependencies: Vec<String>,
    pub expected_duration_secs: Option<u64>,
}

/// State of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Pending,
    InProgress,
    Completed,
    Blocked,
    Failed,
    Intervened,
}

/// A discrete state of a system, machine, or process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessState {
    pub id: String,
    pub name: String,
    pub parameters: std::collections::HashMap<String, f64>,
    pub active_since: chrono::DateTime<chrono::Utc>,
    pub valid_until: Option<chrono::DateTime<chrono::Utc>>,
}

/// A state transition caused by an event or action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub id: String,
    pub from_state: String,
    pub to_state: String,
    pub trigger_event: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration_secs: Option<f64>,
}

/// Resource used in a process (machine, operator, material, tooling).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResource {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub capacity: f64,
    pub current_utilization: f64,
}

/// Operational constraint (e.g. max temperature, deadline, dependency).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConstraint {
    pub id: String,
    pub name: String,
    pub metric: String,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub is_hard_constraint: bool,
}

/// A physical or operational measurement recorded during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMeasurement {
    pub id: String,
    pub metric: String,
    pub value: f64,
    pub unit: String,
    pub machine_or_source: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub is_nominal: bool,
}

impl ProcessMeasurement {
    /// Compare this measurement against a constraint. Returns a `ProcessDeviation`
    /// when `value` falls outside `[min_value, max_value]`; `severity` is a
    /// normalized 0..1 overshoot beyond the nearest bound (clamped). A constraint
    /// with neither bound set can never flag.
    pub fn deviation_against(
        &self,
        constraint: &ProcessConstraint,
        task_or_process_id: &str,
    ) -> Option<ProcessDeviation> {
        let (lo, hi) = (constraint.min_value, constraint.max_value);
        let out_of = match (lo, hi) {
            (Some(l), Some(h)) => self.value < l || self.value > h,
            (Some(l), None) => self.value < l,
            (None, Some(h)) => self.value > h,
            (None, None) => false,
        };
        if !out_of {
            return None;
        }
        let (overshoot, expected) = match (lo, hi) {
            (Some(l), _) if self.value < l => (l - self.value, l),
            (_, Some(h)) if self.value > h => (self.value - h, h),
            _ => (0.0, self.value),
        };
        let span = match (lo, hi) {
            (Some(l), Some(h)) => (h - l).max(1e-9),
            _ => 1.0,
        };
        let severity: Score = ((overshoot / span).min(1.0)) as f32;
        Some(ProcessDeviation {
            id: format!("{}-dev", self.id),
            task_or_process_id: task_or_process_id.to_string(),
            metric: self.metric.clone(),
            expected_value: expected,
            observed_value: self.value,
            severity,
            detected_at: self.timestamp,
            description: format!(
                "{} out of nominal range for {} (observed {}{})",
                self.metric, constraint.id, self.value, self.unit
            ),
        })
    }
}

/// Observed deviation from plan or expected nominal state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDeviation {
    pub id: String,
    pub task_or_process_id: String,
    pub metric: String,
    pub expected_value: f64,
    pub observed_value: f64,
    pub severity: Score,
    pub detected_at: chrono::DateTime<chrono::Utc>,
    pub description: String,
}

/// An intervention performed in response to a deviation or failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessIntervention {
    pub id: String,
    pub deviation_id: Option<String>,
    pub operator: String,
    pub action_taken: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub expected_recovery: String,
    pub actual_recovery: Option<String>,
    pub resolved: bool,
}

/// Final observed outcome of a process cycle or PDCA iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOutcome {
    pub id: String,
    pub plan_id: Option<String>,
    pub success: bool,
    pub output_produced: Option<String>,
    pub scrap_or_error_count: usize,
    pub completion_time: chrono::DateTime<chrono::Utc>,
    pub summary: String,
}

/// Full execution trace of the PDCA / Lean cycle:
/// PLAN → ACTION → OBSERVATION → DEVIATION → INTERVENTION → OUTCOME
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessCycle {
    pub id: String,
    pub plan: Option<ProcessPlan>,
    pub measurements: Vec<ProcessMeasurement>,
    pub deviations: Vec<ProcessDeviation>,
    pub interventions: Vec<ProcessIntervention>,
    pub outcome: Option<ProcessOutcome>,
    pub temporal: Option<TemporalValidity>,
}

impl ProcessCycle {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            plan: None,
            measurements: Vec::new(),
            deviations: Vec::new(),
            interventions: Vec::new(),
            outcome: None,
            temporal: Some(TemporalValidity::permanent()),
        }
    }

    pub fn record_measurement(&mut self, measurement: ProcessMeasurement) {
        self.measurements.push(measurement);
    }

    pub fn record_deviation(&mut self, deviation: ProcessDeviation) {
        self.deviations.push(deviation);
    }

    pub fn record_intervention(&mut self, intervention: ProcessIntervention) {
        self.interventions.push(intervention);
    }

    pub fn complete_outcome(&mut self, outcome: ProcessOutcome) {
        self.outcome = Some(outcome);
    }

    /// Scan recorded measurements against a set of constraints, pushing a
    /// `ProcessDeviation` for each out-of-bounds reading (matched on `metric`).
    /// Returns the number of deviations added. Idempotent only if the caller
    /// clears `deviations` first — callers usually run this once before review.
    pub fn scan_deviations(&mut self, constraints: &[ProcessConstraint]) -> usize {
        let before = self.deviations.len();
        for m in &self.measurements {
            for c in constraints {
                if c.metric == m.metric {
                    if let Some(d) = m.deviation_against(c, &self.id) {
                        self.deviations.push(d);
                    }
                }
            }
        }
        self.deviations.len() - before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constraint(metric: &str, lo: Option<f64>, hi: Option<f64>) -> ProcessConstraint {
        ProcessConstraint {
            id: format!("c-{metric}"),
            name: metric.to_string(),
            metric: metric.to_string(),
            min_value: lo,
            max_value: hi,
            is_hard_constraint: true,
        }
    }

    fn measurement(metric: &str, value: f64) -> ProcessMeasurement {
        ProcessMeasurement {
            id: format!("m-{metric}-{value}"),
            metric: metric.to_string(),
            value,
            unit: "C".to_string(),
            machine_or_source: "sensor-1".to_string(),
            timestamp: chrono::Utc::now(),
            is_nominal: true,
        }
    }

    #[test]
    fn test_in_range_is_no_deviation() {
        let c = constraint("temp", Some(0.0), Some(100.0));
        let m = measurement("temp", 50.0);
        assert!(m.deviation_against(&c, "cyc-1").is_none());
    }

    #[test]
    fn test_over_max_flags_with_severity() {
        let c = constraint("temp", Some(0.0), Some(100.0));
        let m = measurement("temp", 150.0);
        let d = m.deviation_against(&c, "cyc-1").expect("should flag");
        assert_eq!(d.metric, "temp");
        assert_eq!(d.expected_value, 100.0);
        assert!(
            (d.severity - 0.5).abs() < 1e-9,
            "50 over a span of 100 → 0.5"
        );
        assert!(d.description.contains("150"));
    }

    #[test]
    fn test_under_min_flags_with_expected_lo() {
        let c = constraint("temp", Some(10.0), Some(100.0));
        let m = measurement("temp", 5.0);
        let d = m.deviation_against(&c, "cyc-1").expect("should flag");
        assert_eq!(d.expected_value, 10.0);
        assert!(((d.severity as f64) - 5.0 / 90.0).abs() < 1e-6);
    }

    #[test]
    fn test_unbounded_constraint_never_flags() {
        let c = constraint("temp", None, None);
        let m = measurement("temp", 999.0);
        assert!(m.deviation_against(&c, "cyc-1").is_none());
    }

    #[test]
    fn test_cycle_scan_deviations_counts() {
        let mut cyc = ProcessCycle::new("cyc-1");
        cyc.record_measurement(measurement("temp", 50.0)); // ok
        cyc.record_measurement(measurement("temp", 150.0)); // over
        cyc.record_measurement(measurement("press", 5.0)); // no constraint match
        let c_temp = constraint("temp", Some(0.0), Some(100.0));
        let added = cyc.scan_deviations(&[c_temp]);
        assert_eq!(added, 1);
        assert_eq!(cyc.deviations.len(), 1);
        assert_eq!(cyc.deviations[0].metric, "temp");
    }
}
