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
            "inputs": {"initial": self.initial, "seed": self.seed},
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

/// One plan node as the gate takes it: its name, method and `requires`
/// prerequisites.
#[derive(Clone, Debug)]
pub struct PlannedStep {
    /// The step name.
    pub name: String,
    /// The step's method.
    pub method: String,
    /// The prerequisite step names (`depends`).
    pub requires: Vec<String>,
}

/// What an engine settled a step as.
#[derive(Clone, Debug)]
pub struct Settled {
    /// The settled status.
    pub status: StepStatus,
    /// The evidence kind produced, if any.
    pub evidence: Option<String>,
    /// The step's FR-331 result entry.
    pub result: Option<Value>,
}

/// Settle `steps` in `requires` order (FSpec FR-824 "Outcome document"; FSpec
/// FR-192 lets `depends` name a later step, so a step runs once its
/// prerequisites have settled, the earliest ready step in source order first).
/// A step whose prerequisite is not satisfied is `blocked` by the first such
/// prerequisite in source order, is not handed to `run` and gets no result;
/// every other step is settled by `run`. The dispositions come back in source
/// order. A prerequisite cycle is refused upstream; were one to reach here its
/// steps settle `blocked` by their first prerequisite.
pub fn settle_by_requires(
    steps: &[PlannedStep],
    mut run: impl FnMut(&PlannedStep) -> Settled,
) -> Vec<StepDisposition> {
    let mut settled: Vec<Option<StepDisposition>> = vec![None; steps.len()];
    let status_of = |settled: &[Option<StepDisposition>], name: &str| {
        steps
            .iter()
            .position(|step| step.name == name)
            .and_then(|index| settled[index].as_ref().map(|done| done.status))
    };
    while settled.iter().any(Option::is_none) {
        let ready = steps.iter().enumerate().find(|(index, step)| {
            settled[*index].is_none()
                && step.requires.iter().all(|name| {
                    steps.iter().all(|other| other.name != *name)
                        || status_of(&settled, name).is_some()
                })
        });
        let (index, step) = match ready {
            Some(found) => found,
            None => match steps
                .iter()
                .enumerate()
                .find(|(index, _)| settled[*index].is_none())
            {
                Some(found) => found,
                None => break,
            },
        };
        let blocker = steps
            .iter()
            .find(|other| {
                step.requires.contains(&other.name)
                    && status_of(&settled, &other.name)
                        .is_none_or(|status| !status.satisfies_requires())
            })
            .map(|other| other.name.clone());
        settled[index] = Some(match blocker {
            Some(name) => StepDisposition {
                name: step.name.clone(),
                status: StepStatus::Blocked,
                method: step.method.clone(),
                evidence: None,
                blocked_by: Some(name),
                result: None,
            },
            None => {
                let outcome = run(step);
                StepDisposition {
                    name: step.name.clone(),
                    status: outcome.status,
                    method: step.method.clone(),
                    evidence: outcome.evidence,
                    blocked_by: None,
                    result: outcome.result,
                }
            }
        });
    }
    settled.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn planned(name: &str, requires: &[&str]) -> PlannedStep {
        PlannedStep {
            name: name.to_owned(),
            method: "model_check".to_owned(),
            requires: requires.iter().map(|name| (*name).to_owned()).collect(),
        }
    }

    fn settled(status: StepStatus, evidence: Option<&str>, result: Value) -> Settled {
        Settled {
            status,
            evidence: evidence.map(str::to_owned),
            result: Some(result),
        }
    }

    fn outcome(dispositions: Option<Vec<StepDisposition>>) -> PlanRunOutcome {
        PlanRunOutcome {
            package: "pkg".to_owned(),
            plan: "Release".to_owned(),
            document: None,
            initial: None,
            seed: 7,
            dispositions,
        }
    }

    /// Run `Release`-shaped steps through the gate; returns the dispositions
    /// and the names `run` was handed (the steps that were negotiated).
    fn gate(
        steps: &[PlannedStep],
        mut engine: impl FnMut(&str) -> Settled,
    ) -> (Vec<StepDisposition>, Vec<String>) {
        let mut handed = Vec::new();
        let dispositions = settle_by_requires(steps, |step| {
            handed.push(step.name.clone());
            engine(&step.name)
        });
        (dispositions, handed)
    }

    fn release() -> Vec<PlannedStep> {
        vec![
            planned("safe", &[]),
            planned("tested", &[]),
            planned("timely", &["safe"]),
        ]
    }

    #[test]
    #[trace("FSpec-TC-2039", "FSpec-FR-824-AC-1")]
    #[trace("TC-3018", "FR-1038-AC-1")]
    fn a_refuted_step_blocks_its_dependent_and_the_run_writes_failed_and_exit_22() {
        let (dispositions, handed) = gate(&release(), |name| match name {
            "safe" => settled(StepStatus::Refuted, Some("refutation"), json!({})),
            _ => settled(StepStatus::Tested, Some("tested"), json!({})),
        });
        assert_eq!(handed, ["safe", "tested"]);
        let run = outcome(Some(dispositions));
        let document = run.document();
        assert_eq!(document["format"], "filament.verification-plan-outcome/v1");
        assert_eq!(document["status"], "failed");
        assert_eq!(document["failed_steps"], json!(["safe"]));
        assert_eq!(document["inputs"], json!({"initial": null, "seed": 7}));
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
    fn an_unsupported_step_is_negotiated_and_blocks_its_dependent_unnegotiated() {
        let diagnostic = json!({"refusal": {
            "code": "unsupported_projection",
            "cause": "unsupported-requested-capability",
            "capability": "temporal-satisfaction", "extent": "bounded"}});
        let (dispositions, handed) = gate(&release(), |name| match name {
            "safe" => settled(StepStatus::Unsupported, None, diagnostic.clone()),
            _ => settled(StepStatus::Tested, Some("tested"), json!({})),
        });
        assert_eq!(handed, ["safe", "tested"], "timely is never handed on");
        let run = outcome(Some(dispositions));
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
    fn a_measured_step_gates_its_dependent_by_its_decision() {
        let steps = [planned("s", &[]), planned("t", &["s"])];
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
            let (dispositions, handed) = gate(&steps, |step| match step {
                "s" => settled(
                    StepStatus::Measured(decision),
                    Some("measured"),
                    json!({"verdict": {"decision": {"type": name}}}),
                ),
                _ => settled(StepStatus::Proved, Some("proof"), json!({})),
            });
            assert_eq!(handed.contains(&"t".to_owned()), dependent_runs);
            let run = outcome(Some(dispositions));
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
            assert_eq!(
                document["dispositions"]["t"]["status"],
                if dependent_runs { "proved" } else { "blocked" }
            );
        }
    }

    #[test]
    #[trace("FSpec-TC-2045", "FSpec-FR-824-AC-4")]
    fn a_step_requiring_a_later_step_waits_for_it_and_is_blocked_when_it_fails() {
        let steps = [planned("early", &["late"]), planned("late", &[])];
        let (dispositions, handed) = gate(&steps, |_| {
            settled(StepStatus::Refuted, Some("refutation"), json!({}))
        });
        assert_eq!(
            handed,
            ["late"],
            "early is not handed on before late settles"
        );
        assert_eq!(dispositions[0].name, "early");
        assert_eq!(dispositions[0].status, StepStatus::Blocked);
        assert_eq!(dispositions[0].blocked_by.as_deref(), Some("late"));
        assert_eq!(dispositions[1].status, StepStatus::Refuted);
        let (dispositions, handed) = gate(&steps, |_| {
            settled(StepStatus::Proved, Some("proof"), json!({}))
        });
        assert_eq!(handed, ["late", "early"]);
        assert_eq!(dispositions[0].status, StepStatus::Proved);
    }

    #[test]
    #[trace("FSpec-TC-2045", "FSpec-FR-824-AC-4")]
    fn a_blocked_prerequisite_blocks_its_dependents_by_the_first_unsatisfied_one() {
        let steps = [
            planned("a", &[]),
            planned("b", &["a"]),
            planned("c", &["b", "a"]),
        ];
        let (dispositions, handed) = gate(&steps, |_| {
            settled(StepStatus::Inconclusive, None, json!({}))
        });
        assert_eq!(handed, ["a"]);
        assert_eq!(dispositions[1].blocked_by.as_deref(), Some("a"));
        assert_eq!(dispositions[2].blocked_by.as_deref(), Some("a"));
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
