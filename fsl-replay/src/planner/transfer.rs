// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-820's transfer-item settlement after its checked declaration and two
//! premises have been admitted.

use filament_exact::NodeKey;
use filament_semantic_value::fairness::FairnessConstraint;
use fsl_foundation::node_wire::{node_key_from_json, node_key_to_json};
use serde_json::{json, Value};

use super::{
    json::{array, check_members, object, parse_strict, require, string, Cause, Fault, Parsed},
    request::{read_premise_parts, PremiseResult, ReadLimits, RequestRefusal},
    settle::fairness_value,
    Budget, PlannerCause, PlannerValue,
};
use crate::proof_result::{
    Certification, InconclusiveCause, PremiseOutcome, TerminalValue, TransferReason,
};

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
    /// Any result that is not proved, rendered with its FR-331 spelling.
    Other(&'static str),
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

/// Read and admit one transfer request against its checked declaration facts.
pub fn read_transfer_settlement_request(
    facts: &TransferFacts,
    bytes: &[u8],
    limits: &ReadLimits,
) -> Result<(TransferSettlement, Budget), RequestRefusal> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.input_bytes {
        return Err(Fault::new(Cause::InputBytesExceeded, "").into());
    }
    let document = match parse_strict(bytes) {
        Parsed::Document(document) => document,
        Parsed::Malformed => return Err(Fault::new(Cause::MalformedJson, "").into()),
        Parsed::Duplicate(pointer) => {
            return Err(Fault::new(Cause::DuplicateMember, pointer).into());
        }
    };
    let top = object(&document, "")?;
    check_members(
        top,
        "",
        &["format", "package", "item", "slots"],
        &["limits"],
    )?;
    if string(require(top, "format", "")?, "/format")? != super::FORMAT {
        return Err(Fault::invalid("/format".to_owned()).into());
    }
    if string(require(top, "package", "")?, "/package")? != facts.package {
        return Err(Fault::invalid("/package".to_owned()).into());
    }
    let item = object(require(top, "item", "")?, "/item")?;
    check_members(item, "/item", &["role", "node", "clause", "subject"], &[])?;
    if string(require(item, "role", "/item")?, "/item/role")? != "transfer" {
        return Err(Fault::invalid("/item/role".to_owned()).into());
    }
    let node = node_key_from_json(require(item, "node", "/item")?)
        .map_err(|_| Fault::invalid("/item/node".to_owned()))?;
    if node != facts.refinement {
        return Err(Fault::invalid("/item/node".to_owned()).into());
    }
    let clause = node_key_from_json(require(item, "clause", "/item")?)
        .map_err(|_| Fault::invalid("/item/clause".to_owned()))?;
    if clause != facts.clause {
        return Err(Fault::invalid("/item/clause".to_owned()).into());
    }
    if require(item, "subject", "/item")? != &facts.subject {
        return Err(Fault::invalid("/item/subject".to_owned()).into());
    }
    let budget = read_budget(top)?;
    let slots = array(require(top, "slots", "")?, "/slots")?;
    if slots.len() != 2 {
        return Err(Fault::invalid("/slots".to_owned()).into());
    }
    let identities = [&facts.refinement_identity, &facts.abstract_claim_identity];
    let roles = ["refinement", "abstract-claim"];
    let mut premises = Vec::with_capacity(2);
    for index in 0..2 {
        let pointer = format!("/slots/{index}");
        let slot = object(&slots[index], &pointer)?;
        check_members(slot, &pointer, &["role", "identity", "result"], &[])?;
        if string(require(slot, "role", &pointer)?, &format!("{pointer}/role"))? != roles[index] {
            return Err(Fault::invalid(format!("{pointer}/role")).into());
        }
        let identity = require(slot, "identity", &pointer)?;
        object(identity, &format!("{pointer}/identity"))?;
        if identity != identities[index] {
            return Err(Fault::invalid(format!("{pointer}/identity")).into());
        }
        let result_pointer = format!("{pointer}/result");
        let result = require(slot, "result", &pointer)?;
        object(result, &result_pointer)?;
        let (premise_item, entry) = read_premise_parts(result, &result_pointer)?;
        let read_identity = premise_item
            .key
            .to_wire()
            .map_err(|_| Fault::invalid(format!("{result_pointer}/request")))?;
        if read_identity != *identity {
            return Err(Fault::invalid(format!("{result_pointer}/request")).into());
        }
        let result = PremiseResult::from_entry(premise_item, entry);
        premises.push((identity.clone(), premise_value(&result)?));
    }
    let [first, second] = premises
        .try_into()
        .map_err(|_| Fault::invalid("/slots".to_owned()))?;
    Ok((
        TransferSettlement {
            refinement: facts.refinement.clone(),
            clause: facts.clause.clone(),
            subject: facts.subject.clone(),
            step_for_step: facts.step_for_step.clone(),
            fragment_operator: facts.fragment_operator.clone(),
            fairness_implied: facts.fairness_implied,
            unimplied_constraint: facts.unimplied_constraint.clone(),
            liveness: facts.liveness,
            stutter_closure: second
                .0
                .pointer("/temporal/stutter_closure")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            premises: [first, second],
        },
        budget,
    ))
}

fn read_budget(top: &serde_json::Map<String, Value>) -> Result<Budget, Fault> {
    let Some(value) = top.get("limits") else {
        return Ok(Budget::default());
    };
    let limits = object(value, "/limits")?;
    check_members(limits, "/limits", &["max_work"], &[])?;
    let max_work = super::json::count(require(limits, "max_work", "/limits")?, "/limits/max_work")?;
    Ok(Budget { max_work })
}

fn premise_value(result: &PremiseResult) -> Result<TransferPremiseValue, Fault> {
    Ok(match result.record.value() {
        TerminalValue::Proved { certification, .. } => TransferPremiseValue::Proved(*certification),
        TerminalValue::Refuted => TransferPremiseValue::Other("refuted"),
        TerminalValue::Inconclusive(_) => TransferPremiseValue::Other("inconclusive"),
        TerminalValue::Unsupported(_) => TransferPremiseValue::Other("unsupported"),
        TerminalValue::Incomplete(_) => TransferPremiseValue::Other("incomplete"),
        TerminalValue::Failed | TerminalValue::ToolFailed(_) => {
            TransferPremiseValue::Other("failed")
        }
        TerminalValue::Tested => TransferPremiseValue::Other("tested"),
        TerminalValue::Declined { .. } => TransferPremiseValue::Other("declined"),
        TerminalValue::Measured(_) => TransferPremiseValue::Other("measured"),
    })
}

/// Settle an admitted transfer item using the FR-820 decision order.
pub fn settle_transfer(
    request: &TransferSettlement,
    budget: Budget,
) -> Result<Value, super::KeyWireError> {
    let transfer = transfer_record(request)?;
    let step_for_step_holds = request
        .step_for_step
        .get("holds")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let (value, cause, certification) = if budget.max_work < 2 {
        (
            PlannerValue::Incomplete,
            Some(PlannerCause::WorkLimit {
                value: budget.max_work,
            }),
            None,
        )
    } else if !step_for_step_holds && request.fragment_operator.is_some() {
        let Some(operator) = request.fragment_operator.clone() else {
            return Err(super::KeyWireError);
        };
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
                    result: PremiseOutcome::Label(match result {
                        "refuted" => crate::outcome::ItemLabel::Refuted,
                        "inconclusive" => crate::outcome::ItemLabel::Inconclusive,
                        "unsupported" => crate::outcome::ItemLabel::Unsupported,
                        "incomplete" => crate::outcome::ItemLabel::Incomplete,
                        "failed" => crate::outcome::ItemLabel::Failed,
                        "tested" => crate::outcome::ItemLabel::Tested,
                        "measured" => crate::outcome::ItemLabel::Measured,
                        "declined" => crate::outcome::ItemLabel::Declined,
                        _ => return Err(super::KeyWireError),
                    }),
                },
            )),
            None,
        )
    } else {
        let certifications = request
            .premises
            .iter()
            .filter_map(|(_, premise)| match premise {
                TransferPremiseValue::Proved(certification) => Some(*certification),
                TransferPremiseValue::Other(_) => None,
            });
        let certification = certifications.fold(Certification::Certified, |current, next| {
            match (current, next) {
                (Certification::Trusted, _) | (_, Certification::Trusted) => Certification::Trusted,
                (Certification::Uncertified, _) | (_, Certification::Uncertified) => {
                    Certification::Uncertified
                }
                _ => Certification::Certified,
            }
        });
        (PlannerValue::Proved, None, Some(certification))
    };

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
                TransferPremiseValue::Other(value) => json!({"slot": slot, "value": value}),
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
