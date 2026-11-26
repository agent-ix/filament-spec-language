// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-820's transfer-item settlement after its checked declaration and two
//! premises have been admitted.

use filament_exact::NodeKey;
use filament_semantic_value::fairness::FairnessConstraint;
use fsl_foundation::node_wire::node_key_to_json;
use serde_json::{json, Value};

use super::settle::under_policy;
use super::{
    settle::{fairness_value, fold_certifications, label_str},
    Budget, PlannerCause, PlannerValue,
};
use crate::outcome::ItemLabel;
use crate::proof_result::{
    CertificationPolicy,
    Certification, InconclusiveCause, PremiseOutcome, TerminalValue, TransferReason,
};

/// The work one transfer spends: one unit for each of its two premise slots
/// (FSpec FR-492 `limits.max_work` counts one unit per slot read).
const TRANSFER_WORK: u64 = 2;

/// Checked package facts against which a transfer settlement request is
/// admitted.
#[derive(Clone, Debug)]
pub struct TransferFacts {
    /// The checked package identity.
    pub package: String,
    /// The refinement declaration node.
    pub refinement: NodeKey,
    /// The abstract clause node.
    pub clause: NodeKey,
    /// The concrete subject in FR-331 wire form.
    pub subject: Value,
    /// Slot 0's expected FR-331 identity.
    pub refinement_identity: Value,
    /// Slot 1's expected FR-331 identity.
    pub abstract_claim_identity: Value,
    /// The complete fact explaining the step-for-step decision.
    pub step_for_step: Value,
    /// The first position-counting operator, when the clause has one.
    pub fragment_operator: Option<String>,
    /// Whether the refinement's abstract fairness implies the clause fairness.
    pub fairness_implied: bool,
    /// The first unimplied clause fairness constraint.
    pub unimplied_constraint: Option<FairnessConstraint>,
    /// Whether this clause needs the liveness premises.
    pub liveness: bool,
}

/// The result value and certification of one already admitted transfer slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferPremiseValue {
    /// A proved result, retaining its in-process or byte-reader certification.
    Proved(Certification),
    /// Any result that is not proved, by its result label.
    Other(ItemLabel),
}

impl TransferPremiseValue {
    /// The slot value a settled premise `value` gives: `proved` keeps its
    /// certification, every other label is the premise's unproved result.
    pub fn of(value: &TerminalValue) -> Self {
        match (ItemLabel::of(value), value) {
            (ItemLabel::Proved, TerminalValue::Proved { certification, .. }) => {
                Self::Proved(*certification)
            }
            (label, _) => Self::Other(label),
        }
    }
}

/// The two admitted premise slots and checked transfer facts.
#[derive(Clone, Debug)]
pub struct TransferSettlement {
    /// The refinement declaration node.
    pub(crate) refinement: NodeKey,
    /// The abstract clause node.
    pub(crate) clause: NodeKey,
    /// The concrete subject in FR-331 wire form.
    pub(crate) subject: Value,
    /// The complete fact explaining the step-for-step decision.
    pub(crate) step_for_step: Value,
    /// The first position-counting operator, when the clause has one.
    pub(crate) fragment_operator: Option<String>,
    /// Whether the refinement's abstract fairness implies the clause fairness.
    pub(crate) fairness_implied: bool,
    /// The first unimplied clause fairness constraint.
    pub(crate) unimplied_constraint: Option<FairnessConstraint>,
    /// Whether this clause needs the liveness premises.
    pub(crate) liveness: bool,
    /// Whether the abstract premise was checked over the stutter closure.
    pub(crate) stutter_closure: bool,
    /// The two FR-492 slot identities and result labels, in slot order.
    pub(crate) premises: [(Value, TransferPremiseValue); 2],
}

impl TransferSettlement {
    /// Build an in-process transfer after the caller has resolved the
    /// checked declaration, admitted both subjects, and settled the two
    /// premise items. Slot values retain their in-process certification.
    pub fn from_settled_facts(
        facts: &TransferFacts,
        stutter_closure: bool,
        premises: [(Value, TransferPremiseValue); 2],
    ) -> Self {
        Self {
            refinement: facts.refinement.clone(),
            clause: facts.clause.clone(),
            subject: facts.subject.clone(),
            step_for_step: facts.step_for_step.clone(),
            fragment_operator: facts.fragment_operator.clone(),
            fairness_implied: facts.fairness_implied,
            unimplied_constraint: facts.unimplied_constraint.clone(),
            liveness: facts.liveness,
            stutter_closure,
            premises,
        }
    }
}

/// Settle an admitted transfer item using the FR-820 decision order,
/// admitting as `proved` what `policy` admits.
pub fn settle_transfer(
    request: &TransferSettlement,
    budget: Budget,
    policy: CertificationPolicy,
) -> Result<Value, super::KeyWireError> {
    let transfer = transfer_record(request)?;
    let step_for_step_holds = request
        .step_for_step
        .get("holds")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let fragment = if step_for_step_holds {
        None
    } else {
        request.fragment_operator.clone()
    };
    let (value, cause, certification) = if budget.max_work < TRANSFER_WORK {
        (
            PlannerValue::Incomplete,
            Some(PlannerCause::WorkLimit {
                value: budget.max_work,
            }),
            None,
        )
    } else if let Some(operator) = fragment {
        (
            PlannerValue::Inconclusive,
            Some(PlannerCause::Inconclusive(
                InconclusiveCause::TransferNotPreserving {
                    reason: TransferReason::Fragment { operator },
                },
            )),
            None,
        )
    } else if !request.fairness_implied {
        let constraint = request
            .unimplied_constraint
            .clone()
            .ok_or(super::KeyWireError)?;
        (
            PlannerValue::Inconclusive,
            Some(PlannerCause::Inconclusive(
                InconclusiveCause::TransferNotPreserving {
                    reason: TransferReason::Fairness { constraint },
                },
            )),
            None,
        )
    } else if request.liveness && !request.stutter_closure {
        (
            PlannerValue::Inconclusive,
            Some(PlannerCause::Inconclusive(
                InconclusiveCause::TransferNotPreserving {
                    reason: TransferReason::StutterClosure,
                },
            )),
            None,
        )
    } else if let Some((slot, result)) =
        request
            .premises
            .iter()
            .enumerate()
            .find_map(|(slot, (_, value))| match value {
                TransferPremiseValue::Proved(_) => None,
                TransferPremiseValue::Other(result) => Some((slot, *result)),
            })
    {
        (
            PlannerValue::Inconclusive,
            Some(PlannerCause::Inconclusive(
                InconclusiveCause::PremiseUnproved {
                    premise: u64::try_from(slot).unwrap_or(u64::MAX),
                    result: PremiseOutcome::Label(result),
                },
            )),
            None,
        )
    } else {
        let certification = fold_certifications(request.premises.iter().filter_map(
            |(_, premise)| match premise {
                TransferPremiseValue::Proved(certification) => Some(*certification),
                TransferPremiseValue::Other(_) => None,
            },
        ));
        (PlannerValue::Proved, None, Some(certification))
    };

    let (value, cause, certification) = under_policy(policy, value, cause, certification);
    let mut result = json!({
        "format": "filament.planner-result/v1",
        "item": {"role": "transfer", "node": node_key_to_json(&request.refinement)?},
        "value": value.as_str(),
        "basis": value.basis().as_str(),
        "slots": request.premises.iter().enumerate().map(|(slot, (_, value))| {
            match value {
                TransferPremiseValue::Proved(certification) => json!({
                    "slot": slot, "value": "proved", "certification": certification.as_str()
                }),
                TransferPremiseValue::Other(label) => {
                    json!({"slot": slot, "value": label_str(*label)})
                }
            }
        }).collect::<Vec<_>>(),
        "transfer": transfer,
    });
    if let Some(cause) = cause {
        let (member, cause) = cause.to_wire()?;
        result[member] = cause;
    }
    if let Some(certification) = certification {
        result["certification"] = json!(certification.as_str());
    }
    Ok(result)
}

fn transfer_record(request: &TransferSettlement) -> Result<Value, super::KeyWireError> {
    let fragment = match &request.fragment_operator {
        Some(operator) => json!({"class": "position-counting", "operator": operator}),
        None => json!({"class": "stutter-invariant"}),
    };
    let fairness = match (&request.fairness_implied, &request.unimplied_constraint) {
        (true, _) => json!({"implied": true}),
        (false, Some(constraint)) => json!({
            "implied": false,
            "constraint": fairness_value(constraint)?
        }),
        (false, None) => return Err(super::KeyWireError),
    };
    Ok(json!({
        "refinement": node_key_to_json(&request.refinement)?,
        "clause": node_key_to_json(&request.clause)?,
        "subject": request.subject,
        "premises": request.premises.iter().enumerate().map(|(slot, (identity, _))| {
            json!({"slot": slot, "identity": identity})
        }).collect::<Vec<_>>(),
        "step_for_step": request.step_for_step,
        "fragment": fragment,
        "fairness": fairness,
        "premise_form": if request.liveness {
            if request.stutter_closure { "stutter-closure" } else { "maximal" }
        } else { "safety" },
    }))
}
