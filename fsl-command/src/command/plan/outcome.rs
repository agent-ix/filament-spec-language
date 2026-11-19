// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-824 "Outcome document": the `filament.verification-plan-outcome/v1`
//! document the plan run writes (FR-1038). The coordinator renders nothing
//! (FR-753); it hands one [`StepDisposition`] per node here.

use filament_semantic_value::catalog::Category;
use serde_json::{json, Value};

/// The decision of a `measured` step (FSpec FR-409).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    /// The measurement met its acceptance bound.
    Accepted,
    /// The measurement missed it.
    Rejected,
    /// The measurement decided nothing.
    Undecided,
}

/// A step's closed status: the FSpec FR-331 result vocabulary plus `blocked`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepStatus {
    /// Proved.
    Proved,
    /// Refuted.
    Refuted,
    /// Tested.
    Tested,
    /// Measured, with its decision.
    Measured(Decision),
    /// Inconclusive.
    Inconclusive,
    /// No registered engine, or an unsupported item.
    Unsupported,
    /// Declined.
    Declined,
    /// A charge was unavailable.
    Incomplete,
    /// An engine failed.
    Failed,
    /// A `requires` prerequisite stopped the step.
    Blocked,
}

impl StepStatus {
    /// The status name the document carries.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Proved => "proved",
            Self::Refuted => "refuted",
            Self::Tested => "tested",
            Self::Measured(_) => "measured",
            Self::Inconclusive => "inconclusive",
            Self::Unsupported => "unsupported",
            Self::Declined => "declined",
            Self::Incomplete => "incomplete",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
        }
    }

    /// Whether a step requiring this one may run: `proved`, `tested`, or
    /// `measured` with decision `accepted`.
    #[must_use]
    pub const fn satisfies_requires(self) -> bool {
        matches!(
            self,
            Self::Proved | Self::Tested | Self::Measured(Decision::Accepted)
        )
    }

    /// Whether the step is in `failed_steps`: `refuted`, `failed`, or
    /// `measured` with decision `rejected`.
    #[must_use]
    pub const fn is_failure(self) -> bool {
        matches!(
            self,
            Self::Refuted | Self::Failed | Self::Measured(Decision::Rejected)
        )
    }

    /// The step's FSpec FR-301 category.
    #[must_use]
    pub const fn category(self) -> Category {
        match self {
            Self::Proved | Self::Tested | Self::Measured(Decision::Accepted) => Category::Success,
            Self::Refuted | Self::Measured(Decision::Rejected) => Category::Violation,
            Self::Incomplete | Self::Blocked => Category::Incomplete,
            Self::Inconclusive | Self::Measured(Decision::Undecided) => Category::Inconclusive,
            Self::Unsupported => Category::Unsupported,
            Self::Declined => Category::Refusal,
            Self::Failed => Category::InternalFailure,
        }
    }
}

/// One plan node's disposition.
#[derive(Clone, Debug, PartialEq)]
pub struct StepDisposition {
    /// The step name.
    pub name: String,
    /// The settled status.
    pub status: StepStatus,
    /// The step's method (FSpec FR-192 method table).
    pub method: String,
    /// The evidence kind the method produced, `None` when it produced none.
    pub evidence: Option<String>,
    /// The first unsatisfied `requires` prerequisite in source order, for a
    /// blocked step.
    pub blocked_by: Option<String>,
    /// The step's FSpec FR-331 result entry; `None` on a blocked step.
    pub result: Option<Value>,
}

/// The plan run's whole outcome.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanRunOutcome {
    /// The checked package identity.
    pub package: String,
    /// The selected plan's declared name.
    pub plan: String,
    /// The plan document, when the request set `emit`.
    pub document: Option<Value>,
    /// The `initial` file, as the request named it.
    pub initial: Option<String>,
    /// The seed in force.
    pub seed: u64,
    /// One disposition per node in source order; `None` when `execute` was
    /// `false`.
    pub dispositions: Option<Vec<StepDisposition>>,
}

impl PlanRunOutcome {
    /// The step names in `failed_steps`, in source order.
    #[must_use]
    pub fn failed_steps(&self) -> Vec<&str> {
        self.dispositions
            .iter()
            .flatten()
            .filter(|step| step.status.is_failure())
            .map(|step| step.name.as_str())
            .collect()
    }

    /// `passed` when every node is satisfied, `failed` when any node is in
    /// `failed_steps`, otherwise `incomplete`. A run with no dispositions
    /// settled no node and is `incomplete`.
    #[must_use]
    pub fn status(&self) -> &'static str {
        let Some(steps) = &self.dispositions else {
            return "incomplete";
        };
        if steps.iter().any(|step| step.status.is_failure()) {
            "failed"
        } else if steps.iter().all(|step| step.status.satisfies_requires()) {
            "passed"
        } else {
            "incomplete"
        }
    }

    /// The process exit: the highest-severity FSpec FR-301 code of the
    /// nodes' categories (`30`, `20`, `21`, `22`, `10`, `11`, `0`); `0` when
    /// no node settled.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        let codes = self
            .dispositions
            .iter()
            .flatten()
            .map(|step| step.status.category().exit_code());
        Category::most_severe(codes).unwrap_or(0)
    }

    /// The `filament.verification-plan-outcome/v1` document.
    #[must_use]
    pub fn document(&self) -> Value {
        let dispositions = self.dispositions.as_ref().map(|steps| {
            steps
                .iter()
                .map(|step| (step.name.clone(), step_value(step)))
                .collect::<serde_json::Map<_, _>>()
        });
        json!({
            "format": "filament.verification-plan-outcome/v1",
            "package": self.package,
            "plan": self.plan,
            "document": self.document,
            "status": self.status(),
            "inputs": {"initial": self.initial, "seed": self.seed},
            "dispositions": dispositions,
            "failed_steps": self.failed_steps(),
        })
    }
}

fn step_value(step: &StepDisposition) -> Value {
    let mut value = json!({
        "status": step.status.name(),
        "method": step.method,
        "evidence": step.evidence,
        "blocked_by": step.blocked_by,
        "result": step.result,
    });
    if let StepStatus::Measured(decision) = step.status {
        value["decision"] = json!(match decision {
            Decision::Accepted => "accepted",
            Decision::Rejected => "rejected",
            Decision::Undecided => "undecided",
        });
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn step(name: &str, status: StepStatus, blocked_by: Option<&str>) -> StepDisposition {
        StepDisposition {
            name: name.to_owned(),
            status,
            method: "model_check".to_owned(),
            evidence: None,
            blocked_by: blocked_by.map(str::to_owned),
            result: (status != StepStatus::Blocked).then(|| json!({})),
        }
    }

    fn outcome(steps: Vec<StepDisposition>) -> PlanRunOutcome {
        PlanRunOutcome {
            package: "pkg".to_owned(),
            plan: "Release".to_owned(),
            document: None,
            initial: None,
            seed: 7,
            dispositions: Some(steps),
        }
    }

    /// Whether `dependent` runs after `prerequisite` settled `status`.
    fn runs(status: StepStatus) -> bool {
        status.satisfies_requires()
    }

    #[test]
    #[trace("FSpec-TC-2039", "FSpec-FR-824-AC-1")]
    #[trace("TC-3018", "FR-1038-AC-1")]
    fn a_refuted_step_and_its_blocked_dependent_write_failed_and_exit_22() {
        let mut tested = step("tested", StepStatus::Tested, None);
        tested.evidence = Some("tested".to_owned());
        let run = outcome(vec![
            step("safe", StepStatus::Refuted, None),
            tested,
            step("timely", StepStatus::Blocked, Some("safe")),
        ]);
        let document = run.document();
        assert_eq!(document["format"], "filament.verification-plan-outcome/v1");
        assert_eq!(document["status"], "failed");
        assert_eq!(document["failed_steps"], json!(["safe"]));
        assert_eq!(document["inputs"], json!({"initial": null, "seed": 7}));
        assert_eq!(document["dispositions"]["timely"]["blocked_by"], "safe");
        assert_eq!(document["dispositions"]["timely"]["result"], Value::Null);
        assert_eq!(document["dispositions"]["tested"]["evidence"], "tested");
        assert_eq!(run.exit_code(), 22);
    }

    #[test]
    #[trace("FSpec-TC-2039", "FSpec-FR-824-AC-1")]
    #[trace("TC-3018", "FR-1038-AC-1")]
    fn an_emit_only_request_writes_null_dispositions_and_exits_0() {
        let mut run = outcome(Vec::new());
        run.dispositions = None;
        run.document = Some(json!({"format": "filament.verification-plan/v1"}));
        let document = run.document();
        assert_eq!(document["dispositions"], Value::Null);
        assert_eq!(document["document"]["format"], "filament.verification-plan/v1");
        assert_eq!(run.exit_code(), 0);
        assert!(run.failed_steps().is_empty());
    }

    #[test]
    #[trace("FSpec-TC-2045", "FSpec-FR-824-AC-4")]
    fn an_unsupported_step_leaves_the_plan_incomplete_with_no_failed_step_and_exit_21() {
        let run = outcome(vec![
            step("safe", StepStatus::Unsupported, None),
            step("tested", StepStatus::Tested, None),
            step("timely", StepStatus::Blocked, Some("safe")),
        ]);
        assert!(!runs(StepStatus::Unsupported));
        assert_eq!(run.document()["status"], "incomplete");
        assert!(run.failed_steps().is_empty());
        assert_eq!(run.exit_code(), 21);
    }

    #[test]
    #[trace("FSpec-TC-2045", "FSpec-FR-824-AC-5")]
    fn a_measured_step_gates_and_settles_by_its_decision() {
        for (decision, dependent_runs, status, failed, exit) in [
            (Decision::Accepted, true, "passed", vec![], 0),
            (Decision::Rejected, false, "failed", vec!["s"], 22),
            (Decision::Undecided, false, "incomplete", vec![], 22),
        ] {
            let measured = StepStatus::Measured(decision);
            assert_eq!(runs(measured), dependent_runs);
            let dependent = if dependent_runs {
                step("t", StepStatus::Proved, None)
            } else {
                step("t", StepStatus::Blocked, Some("s"))
            };
            let run = outcome(vec![step("s", measured, None), dependent]);
            let document = run.document();
            assert_eq!(document["status"], status);
            assert_eq!(run.failed_steps(), failed);
            assert_eq!(run.exit_code(), exit);
            assert_eq!(
                document["dispositions"]["s"]["decision"],
                match decision {
                    Decision::Accepted => "accepted",
                    Decision::Rejected => "rejected",
                    Decision::Undecided => "undecided",
                }
            );
        }
    }

    #[test]
    #[trace("FSpec-TC-2045", "FSpec-FR-824-AC-4")]
    fn the_exit_is_the_highest_severity_code_in_fr_301_order() {
        for (statuses, exit) in [
            (vec![StepStatus::Failed, StepStatus::Declined], 30),
            (vec![StepStatus::Declined, StepStatus::Unsupported], 20),
            (vec![StepStatus::Unsupported, StepStatus::Incomplete], 21),
            (vec![StepStatus::Refuted, StepStatus::Inconclusive], 22),
            (vec![StepStatus::Refuted, StepStatus::Proved], 10),
        ] {
            let steps = statuses
                .into_iter()
                .enumerate()
                .map(|(index, status)| step(&format!("s{index}"), status, None))
                .collect();
            assert_eq!(outcome(steps).exit_code(), exit);
        }
    }
}
