// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-1 item that decides an authored temporal clause of a closed timed
//! subject on FR-243's digital-clock route, for an item naming
//! `options.method` `digital-clocks` (FR-632): explicit-state search over
//! the subject's integer-clock reading ([`crate::model_check::digital`]),
//! settled by FR-625's map with basis `exhaustive`, its counterexample
//! replayed by FR-237's timed replay. A subject or claim that is not closed
//! is declined `invalid-input` before any search, naming the strict
//! constraint or interval.

use filament_semantic_value::catalog::CatalogCode;
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ModelSystem, TimedSystem, DEFAULT_MAX_CLOCKS};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_replay::{
    settle_model_check, CertificateRequest, DeclineCode, ModelCheckItem, ProofRefusalCause,
    ReplayTargetMembers, RequestTargetWire, TimedReplay,
};
use fsl_route::Mode;
use fsl_semantics::check::Capability;
use serde_json::{json, Map};

use super::clause_item::{clause_name, replay_over, ReplayInputs, ReplayShape, CLAUSE_TRANSCRIPT};
use super::items::{admission, BROKEN, CLAUSE_FAMILY, NOT_YET};
use super::timed_item::{certificate_simulation, subject_refusal, timed_model};
use super::{
    AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SourceProvision, SubjectBinding,
    TemporalItem,
};
use crate::model_check::digital::{
    check_digital, DigitalLimits, DigitalRefusal, DigitalRequest, Strict,
};
use crate::model_check::manifest as en1;

/// FR-243's digital-clock item for the authored clause at `key` over the
/// timed subject of `package`, or `None` when the package holds no such
/// clause or no timed model.
pub(super) fn digital_clause_item<'a>(
    analysis: &super::RunInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    key: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let package = analysis.package;
    let node = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.identity() == *key.node() && node.claim() == key.origin())?;
    let model = timed_model(package)?;
    clause_name(node).ok()?;
    let (binding, form, input) = (binding.clone(), form.clone(), analysis.model.clone());
    Some(
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            CLAUSE_FAMILY,
            Mode::Bounded,
            en1::BACKEND_ID,
            move |cancel| {
                if form.abstract_subject.is_some()
                    || form.stutter_closure.is_some()
                    || form.fairness != node.clause().fairness()
                {
                    return EngineReport::Refused(Refusal::Unsupported(NOT_YET));
                }
                let simulation = || certificate_simulation(&input, &binding, limits, &form);
                let system = match ModelSystem::new(simulation()) {
                    Ok(system) => system,
                    Err(failure) => return EngineReport::Refused(admission(&failure)),
                };
                let max_clocks = form.max_clocks.unwrap_or(DEFAULT_MAX_CLOCKS);
                let timed = match TimedSystem::with_max_clocks(&system, &model, max_clocks) {
                    Ok(timed) => timed,
                    Err(refusal) => return EngineReport::Refused(subject_refusal(&refusal)),
                };
                let Ok(letters) = system.clause_letters(node) else {
                    return EngineReport::Refused(Refusal::Failed(BROKEN));
                };
                let Ok(name) = clause_name(node) else {
                    return EngineReport::Refused(Refusal::Failed(BROKEN));
                };
                let request = DigitalRequest {
                    subject: &timed,
                    clause: node.clause(),
                    letters: &letters,
                    limits: DigitalLimits {
                        max_states: u64::try_from(form.limits.limits.max_states)
                            .unwrap_or(u64::MAX),
                        max_transitions: u64::try_from(form.limits.limits.max_transitions)
                            .unwrap_or(u64::MAX),
                        max_automaton_states: form.limits.max_automaton_states,
                    },
                };
                let outcome = match check_digital(&request, cancel) {
                    Ok(outcome) => outcome,
                    Err(refusal) => return EngineReport::Refused(digital_refusal(&refusal)),
                };
                let inputs = ReplayInputs {
                    backend: en1::BACKEND_ID,
                    package,
                    binding: &binding,
                    limits: &limits,
                    provision,
                    max_candidates: form.max_candidates,
                    timed: Some(TimedReplay {
                        model: model.clone(),
                        exploration: fsl_eval::simulation::Limits::default(),
                        max_clocks,
                        digital: true,
                    }),
                };
                // The closure checker reads no digital-clock product: the
                // proof settles `uncertified` with no certificate (FR-625).
                let certificate = CertificateRequest {
                    temporal_clock_selections: &[],
                    subject: ModelSubject {
                        simulation: simulation(),
                        proof_bounds: form.proof_bounds.clone(),
                    },
                    item: ModelCheckItem::Clause {
                        clause: node.clause(),
                        letters: &letters,
                    },
                    smt_encoding: form.smt_encoding.unwrap_or_default(),
                };
                let record = settle_model_check(
                    &outcome.outcome,
                    &form.limits,
                    &certificate,
                    cancel,
                    |payload| {
                        replay_over(
                            &inputs,
                            ReplayShape {
                                target: RequestTargetWire::state_clause(name.clone()),
                                members: ReplayTargetMembers::state_clause(name.clone()),
                                node: node.identity(),
                                occurrence: OccurrenceKey::new(
                                    node.identity(),
                                    node.claim().clone(),
                                ),
                                transcript: CLAUSE_TRANSCRIPT,
                            },
                            payload,
                            &letters,
                        )
                    },
                );
                let over = record
                    .counterexample
                    .as_ref()
                    .and_then(|found| found.over_binding.as_ref())
                    .and_then(|instance| letters.instance_reference(instance));
                EngineReport::Clause(Box::new(record), over, outcome.non_zeno)
            },
        )
        .claimed(key.clone()),
    )
}

/// FR-243 "Refusals": a strict constraint or interval declines the item
/// `invalid-input` naming it, before any search; a clause with no claim
/// automaton the route reads is unsupported, as EN-6 reads it.
fn digital_refusal(refusal: &DigitalRefusal) -> Refusal {
    match refusal {
        DigitalRefusal::Strict(strict) => {
            let mut detail = Map::new();
            match strict {
                Strict::Constraint(span) => {
                    detail.insert(
                        "strict_constraint".to_owned(),
                        json!({"start": span.start, "end": span.end}),
                    );
                }
                Strict::Interval(interval) => {
                    detail.insert("strict_interval".to_owned(), json!(interval));
                }
            }
            Refusal::DeclinedWithDetail(
                ProofRefusalCause::InvalidInput,
                DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                CatalogCode::new("invalid_runtime_input", "strict-clock-constraint"),
                detail,
            )
        }
        DigitalRefusal::UnsupportedForm => Refusal::Unsupported(NOT_YET),
        DigitalRefusal::Atom(_) | DigitalRefusal::Fault(_) => Refusal::Failed(BROKEN),
    }
}
