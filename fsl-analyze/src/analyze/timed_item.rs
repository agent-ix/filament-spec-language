// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-6 item over an authored temporal clause of a timed subject
//! (FR-632, FR-239, FR-243): built from bytes as [`super::ClauseSubject`]
//! builds EN-1's, so the checked package, the request's subject binding and
//! the item's budgets are everything it needs.
//!
//! The timed subject and the letters of the clause borrow the model system,
//! so they are built inside the item's work, where they live exactly as long
//! as the zone search, its certificate check and the replay of its
//! counterexample (FR-237, through `replay_model_trace`'s timed reading).

use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, TimedSubjectRefusal, TimedSystem,
    DEFAULT_MAX_CLOCKS,
};
use fsl_eval::simulation::NotSimulated;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_replay::{
    CertificateCheckLimits, CertifiedItem, ReplayTargetMembers, RequestTargetWire, TimedReplay,
    ZoneCertificateRequest,
};
use fsl_route::deadlock::{ItemObligation, ObligationKind};
use fsl_route::reductions::ObligationReductions;
use fsl_route::request::ItemKind;
use fsl_route::Mode;
use fsl_semantics::check::{Capability, TemporalClauseNode};
use fsl_semantics::family::ClassifyFailure;

use super::clause_item::{clause_name, replay_over, ReplayInputs, ReplayShape, CLAUSE_TRANSCRIPT};
use super::derived_item::{derived_target, NoLetters, DEADLOCK_TRANSCRIPT};
use super::items::{
    admission, joined_replay, settle_zone_item, TimedItem, BROKEN, CLAUSE_FAMILY, INVALID_VALUE,
    NOT_YET,
};
use super::{
    AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SourceProvision, SubjectBinding,
    TemporalItem,
};
use crate::zone_check::{manifest as en6, ZoneCheckItem, ZoneCheckLimits, ZoneCheckRequest};

/// The alias of the model whose time source the package declares, which the
/// timed subject is read under (FR-230); `None` for an untimed package.
fn timed_model(package: &CheckedPackage) -> Option<String> {
    package
        .graph()
        .time_sources()
        .first()
        .map(|source| source.model.clone())
}

/// The EN-6 item that decides the authored clause at `key` over the timed
/// subject of `package`, or `None` when the package holds no such clause or
/// no timed model.
pub(super) fn zone_clause_item<'a>(
    package: &'a CheckedPackage,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    key: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let node = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.identity() == *key.node() && node.claim() == key.origin())?;
    let model = timed_model(package)?;
    clause_name(node).ok()?;
    let (binding, form) = (binding.clone(), form.clone());
    Some(
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            CLAUSE_FAMILY,
            Mode::Bounded,
            en6::BACKEND_ID,
            move |cancel| {
                run(
                    &Run {
                        package,
                        binding: &binding,
                        limits,
                        provision,
                        node,
                        model: &model,
                        form: &form,
                    },
                    cancel,
                )
            },
        )
        .claimed(key.clone()),
    )
}

/// What one EN-6 run of an authored clause reads.
#[derive(Clone, Copy)]
struct Run<'r, 'a> {
    package: &'a CheckedPackage,
    binding: &'r SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'r SourceProvision,
    node: &'a TemporalClauseNode,
    model: &'r str,
    form: &'r TemporalItem,
}

fn run(read: &Run<'_, '_>, cancel: &filament_exact::Cancel) -> EngineReport {
    let Run {
        package,
        binding,
        limits,
        provision,
        node,
        model,
        form,
    } = *read;
    let simulation = ModelSimulationRequest {
        package,
        packages: binding.packages,
        normalization: limits.normalization,
        snapshots: binding.snapshots,
        initial: binding.initial.clone(),
        universes: binding.universes.clone(),
        observation: limits.observation,
        expansion: ExpansionLimits {
            max_candidates: form.max_candidates,
        },
        meter: limits.accounting,
        emit: limits.emit,
    };
    let system = match ModelSystem::new(simulation) {
        Ok(system) => system,
        Err(failure) => return EngineReport::Refused(admission(&failure)),
    };
    let max_clocks = form.max_clocks.unwrap_or(DEFAULT_MAX_CLOCKS);
    let timed = match TimedSystem::with_max_clocks(&system, model, max_clocks) {
        Ok(timed) => timed,
        Err(refusal) => return EngineReport::Refused(subject_refusal(&refusal)),
    };
    let Ok(letters) = system.clause_letters(node) else {
        return EngineReport::Refused(Refusal::Failed(BROKEN));
    };
    let Ok(name) = clause_name(node) else {
        return EngineReport::Refused(Refusal::Failed(BROKEN));
    };
    let inputs = ReplayInputs {
        backend: en6::BACKEND_ID,
        package,
        binding,
        limits: &limits,
        provision,
        max_candidates: form.max_candidates,
        timed: Some(TimedReplay {
            model: model.to_owned(),
            exploration: fsl_eval::simulation::Limits::default(),
            max_clocks,
        }),
    };
    let reductions = form.reductions.clone();
    let item = TimedItem {
        request: ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
            obligation: ItemObligation {
                subject: (),
                kind: ObligationKind::Authored(fsl_replay::node_occurrence(node)),
                reductions: ObligationReductions::default(),
            },
            limits: zone_limits(form),
            reductions: &reductions,
        },
        certificate: ZoneCertificateRequest {
            simulation: certificate_simulation(package, binding, limits, form),
            model,
            max_clocks,
            item: CertifiedItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
        },
        certificate_limits: CertificateCheckLimits::default(),
        replay: Box::new(|payload| {
            replay_over(
                &inputs,
                ReplayShape {
                    target: RequestTargetWire::state_clause(name.clone()),
                    members: ReplayTargetMembers::state_clause(name.clone()),
                    node: node.identity(),
                    occurrence: OccurrenceKey::new(node.identity(), node.claim().clone()),
                    transcript: CLAUSE_TRANSCRIPT,
                },
                payload,
                &letters,
            )
        }),
    };
    settle_zone_item(item, cancel)
}

/// The refusal of a timed subject the request's binding cannot build: an
/// unbounded root needs a bound (as zone search maps it), a clock owner with
/// no universe is a defect of the binding, the forms EN-6 does not read yet are
/// unsupported, and a walk of the model's root types that reaches its stage
/// limit is a resource stop (`incomplete`). Only the refusals the package and the
/// checker rule out (a missing timed model, a broken extent check) stay an
/// engine fault.
fn subject_refusal(refusal: &TimedSubjectRefusal) -> Refusal {
    match refusal {
        TimedSubjectRefusal::NotSimulated(NotSimulated::RequiresBound(_)) => Refusal::RequiresBound,
        TimedSubjectRefusal::Unbounded { .. } => Refusal::Failed(INVALID_VALUE),
        TimedSubjectRefusal::ParameterizedGuard { .. }
        | TimedSubjectRefusal::NestedUnboundedCollection { .. } => Refusal::Unsupported(NOT_YET),
        TimedSubjectRefusal::NotSimulated(NotSimulated::Extent(ClassifyFailure::Limit(_))) => {
            Refusal::Incomplete(fsl_replay::IncompleteCause::ResourceExhausted)
        }
        TimedSubjectRefusal::NoTimedModel { .. } | TimedSubjectRefusal::NotSimulated(_) => {
            Refusal::Failed(BROKEN)
        }
    }
}

/// EN-6's budgets under the item's `max_states` and `max_transitions`, which
/// bound the symbolic states and edges (FSpec FR-368 "Negotiation").
fn zone_limits(form: &TemporalItem) -> ZoneCheckLimits {
    let mut limits = ZoneCheckLimits::default();
    if let Ok(states) = u64::try_from(form.limits.limits.max_states) {
        limits.max_symbolic_states = states;
    }
    if let Ok(edges) = u64::try_from(form.limits.limits.max_transitions) {
        limits.max_symbolic_transitions = edges;
    }
    limits
}

fn certificate_simulation<'a>(
    package: &'a CheckedPackage,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    form: &TemporalItem,
) -> ModelSimulationRequest<'a> {
    ModelSimulationRequest {
        package,
        packages: binding.packages,
        normalization: limits.normalization,
        snapshots: binding.snapshots,
        initial: binding.initial.clone(),
        universes: binding.universes.clone(),
        observation: limits.observation,
        expansion: ExpansionLimits {
            max_candidates: form.max_candidates,
        },
        meter: limits.accounting,
        emit: limits.emit,
    }
}

/// The EN-6 item that decides the derived `deadlock-freedom` or
/// `time-lock-freedom` item anchored at `claim` over the timed subject of
/// `package` (FR-232, FR-626, FSpec FR-368), or `None` for a package with no
/// timed model or an item kind EN-6 does not derive. A time-lock refutation
/// is a symbolic run concretised with exact rational delays (FR-241), and a
/// Zeno behaviour is reported by it (FR-240).
pub(super) fn zone_derived_item<'a>(
    package: &'a CheckedPackage,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    claim: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let model = timed_model(package)?;
    let kind = form.item_kind;
    if !matches!(kind, ItemKind::DeadlockFreedom | ItemKind::TimeLockFreedom) {
        return None;
    }
    let target = derived_target()?;
    let claimed = claim.clone();
    let (binding, form, claim) = (binding.clone(), form.clone(), claim.clone());
    let family = crate::claim_family::claim_family(package, kind, &claim);
    Some(
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            family,
            Mode::Bounded,
            en6::BACKEND_ID,
            move |cancel| {
                let simulation = certificate_simulation(package, &binding, limits, &form);
                let system = match ModelSystem::new(simulation) {
                    Ok(system) => system,
                    Err(failure) => return EngineReport::Refused(admission(&failure)),
                };
                let max_clocks = form.max_clocks.unwrap_or(DEFAULT_MAX_CLOCKS);
                let timed = match TimedSystem::with_max_clocks(&system, &model, max_clocks) {
                    Ok(timed) => timed,
                    Err(refusal) => return EngineReport::Refused(subject_refusal(&refusal)),
                };
                let inputs = ReplayInputs {
                    backend: en6::BACKEND_ID,
                    package,
                    binding: &binding,
                    limits: &limits,
                    provision,
                    max_candidates: form.max_candidates,
                    timed: Some(TimedReplay {
                        model: model.clone(),
                        exploration: fsl_eval::simulation::Limits::default(),
                        max_clocks,
                    }),
                };
                let reductions = form.reductions.clone();
                let (zone_item, certified, obligation) = match kind {
                    ItemKind::TimeLockFreedom => (
                        ZoneCheckItem::TimeLockFreedom,
                        CertifiedItem::TimeLockFreedom,
                        ObligationKind::TimeLockFreedom,
                    ),
                    _ => (
                        ZoneCheckItem::DeadlockFreedom,
                        CertifiedItem::DeadlockFreedom,
                        ObligationKind::DeadlockFreedom,
                    ),
                };
                let item = TimedItem {
                    request: ZoneCheckRequest {
                        subject: &timed,
                        item: zone_item,
                        obligation: ItemObligation {
                            subject: (),
                            kind: obligation,
                            reductions: ObligationReductions::default(),
                        },
                        limits: zone_limits(&form),
                        reductions: &reductions,
                    },
                    certificate: ZoneCertificateRequest {
                        simulation: certificate_simulation(package, &binding, limits, &form),
                        model: &model,
                        max_clocks,
                        item: certified,
                    },
                    certificate_limits: CertificateCheckLimits::default(),
                    replay: joined_replay(
                        Box::new(|payload| {
                            replay_over(
                                &inputs,
                                ReplayShape {
                                    target: RequestTargetWire::function(
                                        target.clone(),
                                        claim.clone(),
                                    ),
                                    members: ReplayTargetMembers::function(target.clone()),
                                    node: claim.node().clone(),
                                    occurrence: claim.clone(),
                                    transcript: DEADLOCK_TRANSCRIPT,
                                },
                                payload,
                                &NoLetters,
                            )
                        }),
                        package.graph().terminal_keys(),
                    ),
                };
                settle_zone_item(item, cancel)
            },
        )
        .claimed(claimed),
    )
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// A clock owner with no universe is a defect of the request's binding,
    /// never an engine fault; the model-intrinsic forms EN-6 does not read are
    /// unsupported; only a missing timed model, which the package rules out,
    /// stays a fault.
    #[trace("TC-694")]
    #[test]
    fn a_timed_subject_refusal_is_typed_by_what_causes_it() {
        let code = |refusal: Refusal| match refusal {
            Refusal::Failed(code) | Refusal::Unsupported(code) => Some(code),
            _ => None,
        };
        let node = filament_exact::NodeKey::new(
            "test",
            vec![filament_exact::PathSegment::Text("n".to_owned())],
            0,
        )
        .expect("a valid node key");
        let domains = fsl_semantics::family::ClaimExtent::from_domains(
            [(
                fsl_foundation::bound::DomainKey::Node { node, path: vec![] },
                fsl_foundation::bound::DomainKind::Integer,
            )]
            .into(),
        );
        let fsl_semantics::family::ClaimExtent::Unbounded(domains) = domains else {
            panic!("one domain is unbounded");
        };
        let requires = subject_refusal(&TimedSubjectRefusal::NotSimulated(
            NotSimulated::RequiresBound(fsl_eval::simulation::RequiresBound { domains }),
        ));
        assert!(matches!(requires, Refusal::RequiresBound));
        let unbounded = subject_refusal(&TimedSubjectRefusal::Unbounded {
            clock: "x".to_owned(),
        });
        assert_eq!(code(unbounded), Some(INVALID_VALUE));
        let nested = subject_refusal(&TimedSubjectRefusal::NestedUnboundedCollection {
            clock: "x".to_owned(),
        });
        assert!(matches!(nested, Refusal::Unsupported(found) if found == NOT_YET));
        let guard = subject_refusal(&TimedSubjectRefusal::ParameterizedGuard {
            clause: "Reply".to_owned(),
        });
        assert!(matches!(guard, Refusal::Unsupported(found) if found == NOT_YET));
        let absent = subject_refusal(&TimedSubjectRefusal::NoTimedModel {
            model: "M".to_owned(),
        });
        assert_eq!(code(absent), Some(BROKEN));
        let limit = subject_refusal(&TimedSubjectRefusal::NotSimulated(NotSimulated::Extent(
            ClassifyFailure::Limit(fsl_foundation::diagnostic::LimitExceeded::new(
                fsl_foundation::setting::Setting::S3Nodes,
                1,
                2,
            )),
        )));
        assert!(matches!(
            limit,
            Refusal::Incomplete(fsl_replay::IncompleteCause::ResourceExhausted)
        ));
    }
}
