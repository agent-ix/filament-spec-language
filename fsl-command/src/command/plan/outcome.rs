// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-824 "Outcome document": the `filament.verification-plan-outcome/v1`
//! document the plan run writes (FR-1038). The coordinator renders nothing
//! (FR-753); it hands one [`StepDisposition`] per node here.

use filament_semantic_value::catalog::Category;
use fsl_analyze::plan::{PlanDecision, PlanResult, PlanStatus};
use fsl_eval::simulation::model::{ModelBasedTestEvidence, ModelBasedTestRecord};
use serde_json::{Value, json};

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
    /// The model-based trace bound, when supplied.
    pub max_trace_length: Option<u64>,
    /// The model-based value-size bound, when supplied.
    pub max_value_bits: Option<u64>,
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
    /// `failed_steps`, otherwise `incomplete`; `None` (the document's `null`)
    /// when `execute` was `false` and no node settled.
    #[must_use]
    pub fn status(&self) -> Option<&'static str> {
        let steps = self.dispositions.as_ref()?;
        Some(if steps.iter().any(|step| step.status.is_failure()) {
            "failed"
        } else if steps.iter().all(|step| step.status.satisfies_requires()) {
            "passed"
        } else {
            "incomplete"
        })
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
            "inputs": {
                "initial": self.initial,
                "seed": self.seed,
                "max_trace_length": self.max_trace_length,
                "max_value_bits": self.max_value_bits,
            },
            "dispositions": dispositions,
            "failed_steps": self.failed_steps(),
        })
    }
}

fn step_value(step: &StepDisposition) -> Value {
    json!({
        "status": step.status.name(),
        "method": step.method,
        "evidence": step.evidence,
        "blocked_by": step.blocked_by,
        "result": step.result,
    })
}

/// The coordinator's dispositions as the document writes them, in source
/// order (FR-753: `fsl-command` renders what `execute_plan` settled; the
/// `requires` gate is the coordinator's alone).
///
/// # Errors
///
/// A terminal record that does not serialize.
pub fn dispositions(
    outcome: fsl_analyze::plan::PlanOutcome,
) -> Result<Vec<StepDisposition>, serde_json::Error> {
    outcome
        .dispositions
        .into_iter()
        .map(|(name, disposition)| {
            let status = status(&disposition);
            Ok(StepDisposition {
                name,
                status,
                method: disposition.method,
                evidence: disposition.evidence,
                blocked_by: disposition.blocked_by,
                result: disposition.result.as_ref().map(result_value).transpose()?,
            })
        })
        .collect()
}

fn status(disposition: &fsl_analyze::plan::PlanDisposition) -> StepStatus {
    match disposition.status {
        PlanStatus::Proved => StepStatus::Proved,
        PlanStatus::Refuted => StepStatus::Refuted,
        PlanStatus::Tested => StepStatus::Tested,
        PlanStatus::Measured(decision) => StepStatus::Measured(match decision {
            PlanDecision::Accepted => Decision::Accepted,
            PlanDecision::Rejected => Decision::Rejected,
            PlanDecision::Undecided => Decision::Undecided,
        }),
        PlanStatus::Inconclusive => StepStatus::Inconclusive,
        PlanStatus::Unsupported => StepStatus::Unsupported,
        PlanStatus::Declined => StepStatus::Declined,
        PlanStatus::Incomplete => StepStatus::Incomplete,
        PlanStatus::Failed => StepStatus::Failed,
        PlanStatus::Blocked => StepStatus::Blocked,
    }
}

/// The step's FSpec FR-331 result entry: a negotiated item's terminal
/// record as `analyze` writes it, a `measured` record with its FR-410
/// decision at `verdict.decision.type`, simulation runs as FR-181 writes
/// them, and a `model_based_test` step's FR-190 record.
fn result_value(result: &PlanResult) -> Result<Value, serde_json::Error> {
    Ok(match result {
        PlanResult::Analysis(record) => {
            let mut entry = serde_json::to_value(fsl_replay::OutcomeItem::from_terminal(record))?;
            if let (fsl_replay::TerminalValue::Measured(verdict), Some(members)) =
                (record.value(), entry.as_object_mut())
            {
                let name = match verdict.decision {
                    fsl_replay::statistical::Decision::Accepted => "accepted",
                    fsl_replay::statistical::Decision::Rejected => "rejected",
                    fsl_replay::statistical::Decision::Undecided(_) => "undecided",
                };
                members.insert(
                    "verdict".to_owned(),
                    json!({"decision": {"type": name}}),
                );
            }
            entry
        }
        PlanResult::Simulation(runs) => runs.clone(),
        PlanResult::ModelBasedTest(record) => model_based_test_record(record),
        PlanResult::NotRun {
            code,
            cause,
            member,
        } => json!({"refusal": {"code": code, "cause": cause, "member": member}}),
    })
}

fn evidence_name(evidence: ModelBasedTestEvidence) -> &'static str {
    match evidence {
        ModelBasedTestEvidence::Tested => "tested",
        ModelBasedTestEvidence::Refuted => "refuted",
        ModelBasedTestEvidence::Inconclusive => "inconclusive",
        ModelBasedTestEvidence::Incomplete => "incomplete",
        ModelBasedTestEvidence::Failed => "failed",
        ModelBasedTestEvidence::Unsupported => "unsupported",
    }
}

/// FSpec FR-190's result record of a `model_based_test` step: its `result`
/// and `cause` carry the step's FR-331 settlement.
fn model_based_test_record(record: &ModelBasedTestRecord) -> Value {
    use fsl_eval::simulation::model::ModelBasedTestCause;
    let cause = record.cause.as_ref().map(|cause| match cause {
        ModelBasedTestCause::NoEvaluatedCase => json!({"type": "no-evaluated-case"}),
        ModelBasedTestCause::LimitReached {
            limit,
            value,
            setting,
        } => json!({
            "type": "limit-reached",
            "limit": limit.as_str(),
            "value": value,
            "setting": setting,
        }),
        ModelBasedTestCause::ToolFailure { defect } => json!({
            "type": "tool-failure",
            "defect": format!("{defect:?}"),
        }),
        ModelBasedTestCause::NoContractLevelReading => {
            json!({"type": "no-contract-level-reading"})
        }
    });
    json!({
        "clause": record.clause,
        "result": evidence_name(record.result),
        "cause": cause,
        "evidence": evidence_name(record.evidence),
        "seed": record.seed,
        "max_cases": record.max_cases,
        "max_trace_length": record.max_trace_length,
        "max_value_bits": record.max_value_bits,
        "cases": {
            "generated": record.counts.generated,
            "passed": record.counts.passing,
            "failed": record.counts.failing,
            "discarded": record.counts.discarded,
        },
        "checks": {"evaluated": record.checks.evaluated},
        "provenance": {
            "seed": record.seed,
            "max_value_bits": record.provenance.max_value_bits,
        },
        "counterexample": record.counterexample.is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// A settled node; the `requires` gate that decides which nodes run is
    /// the coordinator's (`fsl_analyze::plan`), so each case states the
    /// dispositions the coordinator hands the writer.
    fn step(name: &str, status: StepStatus, evidence: Option<&str>, result: Value) -> StepDisposition {
        StepDisposition {
            name: name.to_owned(),
            status,
            method: "model_check".to_owned(),
            evidence: evidence.map(str::to_owned),
            blocked_by: None,
            result: Some(result),
        }
    }

    fn blocked(name: &str, by: &str) -> StepDisposition {
        StepDisposition {
            name: name.to_owned(),
            status: StepStatus::Blocked,
            method: "model_check".to_owned(),
            evidence: None,
            blocked_by: Some(by.to_owned()),
            result: None,
        }
    }

    fn outcome(dispositions: Option<Vec<StepDisposition>>) -> PlanRunOutcome {
        PlanRunOutcome {
            package: "pkg".to_owned(),
            plan: "Release".to_owned(),
            document: None,
            initial: None,
            seed: 7,
            max_trace_length: None,
            max_value_bits: None,
            dispositions,
        }
    }

    #[test]
    #[trace("FSpec-TC-2039", "FSpec-FR-824-AC-1")]
    #[trace("TC-3018", "FR-1038-AC-1")]
    fn a_refuted_step_blocks_its_dependent_and_the_run_writes_failed_and_exit_22() {
        let run = outcome(Some(vec![
            step("safe", StepStatus::Refuted, Some("refutation"), json!({})),
            step("tested", StepStatus::Tested, Some("tested"), json!({})),
            blocked("timely", "safe"),
        ]));
        let document = run.document();
        assert_eq!(document["format"], "filament.verification-plan-outcome/v1");
        assert_eq!(document["status"], "failed");
        assert_eq!(document["failed_steps"], json!(["safe"]));
        assert_eq!(
            document["inputs"],
            json!({
                "initial": null,
                "seed": 7,
                "max_trace_length": null,
                "max_value_bits": null
            })
        );
        assert_eq!(document["dispositions"]["safe"]["evidence"], "refutation");
        assert_eq!(document["dispositions"]["tested"]["evidence"], "tested");
        assert_eq!(document["dispositions"]["timely"]["status"], "blocked");
        assert_eq!(document["dispositions"]["timely"]["blocked_by"], "safe");
        assert_eq!(document["dispositions"]["timely"]["result"], Value::Null);
        assert_eq!(run.exit_code(), 22);
    }

    #[test]
    #[trace("FSpec-TC-2039", "FSpec-FR-824-AC-1")]
    #[trace("TC-3018", "FR-1038-AC-1")]
    #[trace("TC-3032", "FR-1038-AC-3")]
    #[trace("FSpec-TC-2046", "FSpec-FR-824-AC-6")]
    fn an_emit_only_run_writes_null_status_and_dispositions_and_exits_0() {
        let mut run = outcome(None);
        run.document = Some(json!({"format": "filament.verification-plan/v1"}));
        let document = run.document();
        assert_eq!(document["status"], Value::Null);
        assert_eq!(document["dispositions"], Value::Null);
        assert_eq!(document["failed_steps"], json!([]));
        assert_eq!(
            document["document"]["format"],
            "filament.verification-plan/v1"
        );
        assert_eq!(run.exit_code(), 0);
    }

    #[test]
    #[trace("FSpec-TC-2045", "FSpec-FR-824-AC-4")]
    fn an_unsupported_step_and_its_blocked_dependent_write_incomplete_and_exit_21() {
        let diagnostic = json!({"refusal": {
            "code": "unsupported_projection",
            "cause": "unsupported-requested-capability",
            "capability": "temporal-satisfaction", "extent": "bounded"}});
        let run = outcome(Some(vec![
            step("safe", StepStatus::Unsupported, None, diagnostic),
            step("tested", StepStatus::Tested, Some("tested"), json!({})),
            blocked("timely", "safe"),
        ]));
        let document = run.document();
        let safe = &document["dispositions"]["safe"];
        assert_eq!(safe["status"], "unsupported");
        assert_eq!(safe["evidence"], Value::Null);
        assert_eq!(safe["result"]["refusal"]["code"], "unsupported_projection");
        assert_eq!(
            safe["result"]["refusal"]["cause"],
            "unsupported-requested-capability"
        );
        assert_eq!(document["dispositions"]["tested"]["status"], "tested");
        assert_eq!(document["dispositions"]["timely"]["blocked_by"], "safe");
        assert_eq!(document["status"], "incomplete");
        assert_eq!(document["failed_steps"], json!([]));
        assert_eq!(run.exit_code(), 21);
    }

    #[test]
    #[trace("FSpec-TC-2045", "FSpec-FR-824-AC-5")]
    #[trace("FSpec-TC-2046", "FSpec-FR-824-AC-6")]
    fn a_measured_step_writes_its_decision_only_in_its_result() {
        for (decision, name, dependent_runs, status, failed, exit) in [
            (Decision::Accepted, "accepted", true, "passed", vec![], 0),
            (
                Decision::Rejected,
                "rejected",
                false,
                "failed",
                vec!["s"],
                22,
            ),
            (
                Decision::Undecided,
                "undecided",
                false,
                "incomplete",
                vec![],
                22,
            ),
        ] {
            assert_eq!(StepStatus::Measured(decision).satisfies_requires(), dependent_runs);
            let measured = step(
                "s",
                StepStatus::Measured(decision),
                Some("statistical"),
                json!({"verdict": {"decision": {"type": name}}}),
            );
            let dependent = if dependent_runs {
                step("t", StepStatus::Proved, Some("proof"), json!({}))
            } else {
                blocked("t", "s")
            };
            let run = outcome(Some(vec![measured, dependent]));
            let document = run.document();
            assert_eq!(document["status"], status);
            assert_eq!(run.failed_steps(), failed);
            assert_eq!(run.exit_code(), exit);
            let measured = document["dispositions"]["s"].as_object().unwrap();
            let mut members: Vec<&str> = measured.keys().map(String::as_str).collect();
            members.sort_unstable();
            assert_eq!(
                members,
                ["blocked_by", "evidence", "method", "result", "status"]
            );
            assert_eq!(measured["status"], "measured");
            assert_eq!(measured["result"]["verdict"]["decision"]["type"], name);
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
                .map(|(index, status)| StepDisposition {
                    name: format!("s{index}"),
                    status,
                    method: "model_check".to_owned(),
                    evidence: None,
                    blocked_by: None,
                    result: None,
                })
                .collect();
            assert_eq!(outcome(Some(steps)).exit_code(), exit);
        }
    }
}
