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
    ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem, TimedSubjectRefusal,
    TimedSystem, DEFAULT_MAX_CLOCKS,
};
use fsl_eval::simulation::NotSimulated;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{RequestIndex, Setting, SettingLimits};
use fsl_package::CheckedPackage;
use fsl_replay::{
    CertificateCheckLimits, CertifiedItem, ModelCheckLimit, ReplayTargetMembers, RequestTargetWire,
    TimedReplay, ZoneCertificateRequest,
};
use fsl_route::deadlock::{ItemObligation, ObligationKind};
use fsl_route::reductions::ObligationReductions;
use fsl_route::request::ItemKind;
use fsl_route::Mode;
use fsl_semantics::check::{Capability, TemporalClauseNode};
use fsl_semantics::family::ClassifyFailure;
use ix_trace_rs::implements;

use super::clause_item::{clause_name, replay_over, ReplayInputs, ReplayShape, CLAUSE_TRANSCRIPT};
use super::derived_item::{derived_target, NoLetters, DEADLOCK_TRANSCRIPT};
use super::items::{
    admission, joined_replay, settle_zone_item, TimedItem, BROKEN, CLAUSE_FAMILY, INVALID_VALUE,
    NOT_YET,
};
use super::{
    AnalyzeItem, AnalyzeLimits, EngineBounds, EngineReport, Refusal, SourceProvision,
    SubjectBinding, TemporalItem, ENGINE_SETTINGS,
};
use crate::zone_check::{manifest as en6, ZoneCheckItem, ZoneCheckLimits, ZoneCheckRequest};

/// The alias of the model whose time source the package declares, which the
/// timed subject is read under (FR-230); `None` for an untimed package.
pub(super) fn timed_model(package: &CheckedPackage) -> Option<String> {
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
    analysis: &super::RunInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    key: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let (package, input) = (analysis.package, &analysis.model);
    let node = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.identity() == *key.node() && node.claim() == key.origin())?;
    let model = timed_model(package)?;
    clause_name(node).ok()?;
    let (binding, form, input) = (binding.clone(), form.clone(), input.clone());
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
                        input: &input,
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
    input: &'r ModelCheckInput<'a>,
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
        input,
        binding,
        limits,
        provision,
        node,
        model,
        form,
    } = *read;
    let simulation = ModelSimulationRequest {
        input: input.clone(),
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
            digital: false,
        }),
        stutter_closure: false,
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
            limits: zone_limits(&zone_bounds(form, limits.engines)),
            reductions: &reductions,
        },
        certificate: ZoneCertificateRequest {
            simulation: certificate_simulation(input, binding, limits, form),
            model,
            max_clocks,
            item: CertifiedItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
        },
        certificate_limits: certificate_limits(&limits.engines),
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
pub(super) fn subject_refusal(refusal: &TimedSubjectRefusal) -> Refusal {
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

/// The caller's settings as a budget reads them: only the rows it set.
impl SettingLimits for EngineBounds {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        ENGINE_SETTINGS
            .iter()
            .filter_map(|setting| self.get(*setting).map(|bound| (*setting, bound)))
            .collect()
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        self.set(setting, bound)
    }
}

/// EN-6's bounds for one item: the caller's `zone_check.*` settings, with
/// each member the item states in place of its setting (FR-239 "Budget
/// sources", FSpec FR-368 "Negotiation"). `model_check.*` bounds never
/// reach them.
#[implements("FR-239-AC-7")]
pub(super) fn zone_bounds(form: &TemporalItem, engines: EngineBounds) -> EngineBounds {
    let stated = |limit, value: usize| {
        form.requested_limits
            .contains(&limit)
            .then(|| u64::try_from(value).unwrap_or(u64::MAX))
    };
    with_stated(
        engines,
        stated(ModelCheckLimit::MaxStates, form.limits.limits.max_states),
        stated(
            ModelCheckLimit::MaxTransitions,
            form.limits.limits.max_transitions,
        ),
    )
}

/// `engines` with an item's stated `max_states` and `max_transitions` as its
/// symbolic-state and symbolic-edge bounds.
pub(super) fn with_stated(
    mut engines: EngineBounds,
    max_states: Option<u64>,
    max_transitions: Option<u64>,
) -> EngineBounds {
    for (setting, stated) in [
        (Setting::ZoneCheckMaxSymbolicStates, max_states),
        (Setting::ZoneCheckMaxSymbolicTransitions, max_transitions),
    ] {
        if let Some(bound) = stated {
            engines.set(setting, bound);
        }
    }
    engines
}

/// The zone search's limits under `bounds`, each unset one at its FR-255
/// default.
#[implements("FR-239-AC-7")]
pub(super) fn zone_limits(bounds: &EngineBounds) -> ZoneCheckLimits {
    let default = ZoneCheckLimits::default();
    let bound = |setting, default| bounds.get(setting).unwrap_or(default);
    ZoneCheckLimits {
        max_symbolic_states: bound(
            Setting::ZoneCheckMaxSymbolicStates,
            default.max_symbolic_states,
        ),
        max_symbolic_transitions: bound(
            Setting::ZoneCheckMaxSymbolicTransitions,
            default.max_symbolic_transitions,
        ),
        max_automaton_states: bound(
            Setting::ZoneCheckMaxAutomatonStates,
            default.max_automaton_states,
        ),
        max_zone_bytes: bound(Setting::ZoneCheckMaxZoneBytes, default.max_zone_bytes),
    }
}

/// The certificate checker's limits under `bounds`: its work is
/// `zone_check.max_check_work`.
pub(super) fn certificate_limits(bounds: &EngineBounds) -> CertificateCheckLimits {
    let default = CertificateCheckLimits::default();
    CertificateCheckLimits {
        max_check_work: bounds
            .get(Setting::ZoneCheckMaxCheckWork)
            .unwrap_or(default.max_check_work),
        ..default
    }
}

pub(super) fn certificate_simulation<'a>(
    input: &ModelCheckInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    form: &TemporalItem,
) -> ModelSimulationRequest<'a> {
    ModelSimulationRequest {
        input: input.clone(),
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
    analysis: &super::RunInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    claim: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let (package, input) = (analysis.package, &analysis.model);
    let model = timed_model(package)?;
    let kind = form.item_kind;
    if !matches!(kind, ItemKind::DeadlockFreedom | ItemKind::TimeLockFreedom) {
        return None;
    }
    let target = derived_target()?;
    let claimed = claim.clone();
    let (binding, form, claim, input) =
        (binding.clone(), form.clone(), claim.clone(), input.clone());
    let family = crate::claim_family::claim_family(package, kind, &claim);
    Some(
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            family,
            Mode::Bounded,
            en6::BACKEND_ID,
            move |cancel| {
                let simulation = certificate_simulation(&input, &binding, limits, &form);
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
                        digital: false,
                    }),
                    stutter_closure: false,
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
                        limits: zone_limits(&zone_bounds(&form, limits.engines)),
                        reductions: &reductions,
                    },
                    certificate: ZoneCertificateRequest {
                        simulation: certificate_simulation(&input, &binding, limits, &form),
                        model: &model,
                        max_clocks,
                        item: certified,
                    },
                    certificate_limits: certificate_limits(&limits.engines),
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

    /// FR-239-AC-7: an item that states no limit, under no caller setting,
    /// runs EN-6 with the FR-255 `zone_check.*` defaults, and EN-1's bounds
    /// do not reach it. A caller setting and a stated member each replace
    /// only their own bound.
    #[trace("FR-239-AC-7")]
    #[test]
    fn an_unstated_item_runs_under_the_zone_check_defaults() {
        let mut form = TemporalItem::default();
        // EN-1's published 10^7 states and 10^8 transitions, which the item
        // did not state.
        form.limits.limits.max_states = 10_000_000;
        form.limits.limits.max_transitions = 100_000_000;
        let published = ZoneCheckLimits {
            max_symbolic_states: 1_048_576,
            max_symbolic_transitions: 16_777_216,
            max_automaton_states: 1_048_576,
            max_zone_bytes: 1_073_741_824,
        };
        let silent = zone_bounds(&form, EngineBounds::default());
        assert_eq!(zone_limits(&silent), published);
        assert_eq!(certificate_limits(&silent).max_check_work, 100_000_000);

        let mut caller = EngineBounds::default();
        assert!(caller.set(Setting::ZoneCheckMaxSymbolicStates, 2));
        assert!(caller.set(Setting::ZoneCheckMaxCheckWork, 3));
        form.limits.limits.max_transitions = 5;
        form.requested_limits = vec![ModelCheckLimit::MaxTransitions];
        let bounds = zone_bounds(&form, caller);
        assert_eq!(
            zone_limits(&bounds),
            ZoneCheckLimits {
                max_symbolic_states: 2,
                max_symbolic_transitions: 5,
                ..published
            }
        );
        assert_eq!(certificate_limits(&bounds).max_check_work, 3);
    }

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
            NotSimulated::RequiresBound(fsl_eval::simulation::RequiresBound {
                domains: Some(domains),
                capacities: Vec::new(),
                clock_references: Vec::new(),
            }),
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
