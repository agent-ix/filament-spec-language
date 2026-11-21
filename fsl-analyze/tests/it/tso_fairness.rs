// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-228 and FR-227 (TC-673, TC-672): memory fairness under `tso` and `ra`,
//! its effect on infinite traces, and replay of unfair lassos.
//!
//! `Spin` is the store-and-poll program of QSL-994 over the `litmus` fixture, named `SB`
//! because the shared case helpers name that protocol: branch `w` has one
//! `relaxed` store of 1 to `x`, and branch `r` a `repeat` with no maximum
//! whose body is one `relaxed` load of `x` into `k.r1`. The fixture's
//! operations return no value, so the loop guard is `true` in place of the
//! load's binder reading 0; the lasso of the AC is the same.

use filament_semantic_value::checking::CheckMode;
use std::collections::BTreeMap;

use filament_exact::{Cancel, Meter};
use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use fsl_analyze::model_check::{
    check_protocol, check_protocol_constrained, ModelCheckRefusal, ProtocolCheckRequest,
};
use fsl_eval::fairness::{check_loop, EdgeLabel, Identity, LassoFairness, LoopStep};
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::protocol::{
    protocol_constraints, FairnessOrigin, MemoryModel, MemorySelection, ProtocolFairness,
    ProtocolState, ProtocolSystem, ProtocolTarget, ProtocolTransition, Ra, Sc, StepKind, Tso,
};
use fsl_eval::simulation::{SubjectSystem, TransitionSystem};
use fsl_eval::temporal::AtomReading;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{Code, MemoryModelName, SourceIdentity};
use fsl_package::CheckedPackage;
use fsl_replay::{
    check_protocol_closure, check_protocol_components, replay_protocol_trace,
    settle_protocol_check, CertificateRule, ClosureRefusal, ComponentCertificate,
    CounterexampleKind, Lasso, MemoryBound, MemoryBoundMember, MemoryCause, ModelCheckItem,
    ModelCheckLimits, ModelCheckOutcome, ModelStep, ModelTraceRefusal, ModelTraceSubject,
    ProofCertificate, ProtocolCertificateRequest, ProtocolTraceSubject, ReductionCause,
    ReplayLimits, ReplayRefusal, TemporalCertificateCheck, TemporalCounterexample,
};
use fsl_semantics::check::FormulaNodeId;
use ix_trace_rs::trace;

use crate::litmus::{
    attempt, contracts, core, document, field, package, protocol, protocol_with, sb_run, unit_text,
    Documents,
};
use crate::model_settle::{envelope_at, request};
use crate::weak_memory::select;

/// `eventually holds(k.r1 = 1)`.
const SETTLES: &str = "temporal Settles using t0 over (k: M::Core) clock \"model-steps\" \
                       on origin { eventually holds(k.r1 = 1) }\n";

/// `eventually holds(k.r1 <= 1)` over store buffering.
const AT_MOST_ONE: &str = "temporal AtMostOne using t0 over (k: M::Core) clock \"model-steps\" \
                           on origin { eventually holds(k.r1 <= 1) }\n";

/// The claim's one atom, read from the bound core's `r1`.
struct R1(fn(i64) -> bool);

impl ModelLetters for R1 {
    crate::units::unguarded!();

    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Ok(if (self.0)(field(state, "cores", &instance.key, "r1")) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

static IS_ONE: R1 = R1(|value| value == 1);
static AT_MOST: R1 = R1(|value| value <= 1);

/// `Spin`'s `run`.
fn spin_run() -> String {
    format!(
        "parallel Both {{ branch w sequence W {{ {} }} \
         branch r repeat Spin by c visible () while {{ true }} sequence R {{ {} }} }} \
         join all [w, r];",
        attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        attempt("Lx", "c", "Core::read1", "relaxed", "ReadsX1, Read1"),
    )
}

/// One unit with its claim.
struct Unit {
    body: String,
    documents: Documents,
    checked: CheckedPackage,
    letters: &'static dyn ModelLetters,
}

impl Unit {
    fn new(run: &str, claim: &str, letters: &'static dyn ModelLetters) -> Self {
        let body = contracts() + &protocol("SB", run) + claim;
        Self {
            checked: package(&body),
            documents: Documents::new(),
            body,
            letters,
        }
    }

    fn subject(
        &self,
        selection: &[MemorySelection],
    ) -> fsl_eval::model_subject::ProtocolSubject<'_> {
        self.documents.subject(&self.checked, "SB", selection)
    }

    fn clause(&self) -> &fsl_semantics::check::CheckedTemporalClause {
        self.checked.graph().temporal_clauses()[0].clause()
    }

    fn item(&self) -> ModelCheckItem<'_> {
        ModelCheckItem::Clause {
            clause: self.clause(),
            letters: self.letters,
        }
    }

    fn tso(&self) -> Vec<MemorySelection> {
        vec![select("SB::Both", MemoryModelName::Tso)]
    }

    fn ra(&self) -> Vec<MemorySelection> {
        vec![select("SB::Both", MemoryModelName::Ra)]
    }

    fn check(
        &self,
        selection: &[MemorySelection],
        max_store_buffer: Option<u64>,
    ) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal> {
        self.check_with_ignored_memory(selection, max_store_buffer, false)
    }

    /// The run under the state constraint `constraint` and the channel bound
    /// `max_messages` (FR-158, FR-225).
    fn check_constrained(
        &self,
        selection: &[MemorySelection],
        constraint: Option<&str>,
        max_store_buffer: Option<u64>,
        max_messages: Option<u64>,
    ) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal> {
        check_protocol_constrained(
            ProtocolCheckRequest {
                temporal_clock_selections: &[],
                subject: self.subject(selection),
                item: self.item(),
                max_depth: None,
                max_store_buffer,
                max_messages,
                max_live_instances: None,
                limits: ModelCheckLimits::default(),
                ignore_memory_constraints: false,
            },
            constraint,
            &Cancel::new(),
        )
    }

    fn check_with_ignored_memory(
        &self,
        selection: &[MemorySelection],
        max_store_buffer: Option<u64>,
        ignore_memory_constraints: bool,
    ) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal> {
        check_protocol(
            ProtocolCheckRequest {
                temporal_clock_selections: &[],
                subject: self.subject(selection),
                item: self.item(),
                max_depth: None,
                max_store_buffer,
                max_messages: None,
                max_live_instances: None,
                limits: ModelCheckLimits::default(),
                ignore_memory_constraints,
            },
            &Cancel::new(),
        )
    }

    fn certificate_request<'u>(
        &'u self,
        selection: &[MemorySelection],
        item: ModelCheckItem<'u>,
    ) -> ProtocolCertificateRequest<'u> {
        ProtocolCertificateRequest {
            temporal_clock_selections: &[],
            subject: self.subject(selection),
            item,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        }
    }

    /// The component certificate of the unit's liveness proof under `tso`.
    fn certified(&self) -> ComponentCertificate {
        match self.check(&self.tso(), None).expect("the subject checks") {
            ModelCheckOutcome::Holds {
                certificate: ProofCertificate::Component(certificate),
                ..
            } => certificate,
            other => panic!("{other:?}"),
        }
    }

    fn components(
        &self,
        certificate: &ComponentCertificate,
    ) -> Result<TemporalCertificateCheck, ClosureRefusal> {
        check_protocol_components(
            &self.certificate_request(&self.tso(), self.item()),
            certificate,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        )
    }

    fn replay(
        &self,
        selection: &[MemorySelection],
        payload: TemporalCounterexample<ProtocolTransition>,
    ) -> Result<fsl_replay::ModelTraceReplayResult, ReplayRefusal> {
        let unit = unit_text(&self.body).into_bytes();
        let node = &self.checked.graph().temporal_clauses()[0];
        let clause_node = node.identity().clone();
        let occurrence = OccurrenceKey::new(clause_node.clone(), node.claim().clone());
        let subject = self.subject(selection);
        replay_protocol_trace(
            request(
                &self.checked,
                &[],
                &document(),
                self.documents.snapshots.iter(),
                &unit,
            ),
            &ProtocolTraceSubject {
                model: ModelTraceSubject {
                    timed: None,
                    initial: vec![self.documents.initial.clone()],
                    universes: self.documents.universes(),
                    expansion: subject.model.simulation.expansion,
                    stutter_closure: false,
                },
                protocol: "SB".to_owned(),
                over: core(),
                roles: subject.roles.clone(),
                capacities: Vec::new(),
                memory: subject.memory.clone(),
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
            },
            &envelope_at(payload, clause_node, occurrence),
            self.letters,
            ReplayLimits::default(),
        )
    }
}

/// The `fair weak each` flush member over the branch `branch` of `Both`.
fn flush(branch: &str) -> ProtocolFairness {
    ProtocolFairness::memory_step(MemoryModelName::Tso, "flush", "Both", branch)
}

/// The system of `unit` under `memory`.
macro_rules! system {
    ($unit:expr, $selection:expr, $memory:expr) => {{
        let subject = $unit.subject($selection);
        let model = ModelSystem::new(subject.model.simulation.clone()).expect("admits");
        ProtocolSystem::new(model, &subject, $memory).expect("builds")
    }};
}

/// ADR-025 MF-1 (TC-673): `eventually holds(k.r1 <= 1)` over
/// `SB` resolved to `tso` carries `fair weak each flush(left)` and
/// `fair weak each flush(right)` with origin `memory` beside the three
/// scheduler constraints; resolved to `sc` it carries no `memory` member.
/// The separate RA test below checks the visibility member.
#[trace("TC-673")]
#[trace("FR-228-AC-1")]
#[test]
fn a_tso_clause_carries_flush_fairness_beside_the_scheduler_constraints() {
    let unit = Unit::new(&sb_run("", "", ""), AT_MOST_ONE, &AT_MOST);
    let tso = unit.tso();
    let set = system!(unit, &tso, Tso::new(4))
        .fairness_set(unit.clause().fairness())
        .expect("the set builds");
    let memory: Vec<&ProtocolFairness> = set
        .iter()
        .filter(|member| member.origin() == FairnessOrigin::Memory)
        .collect();
    assert_eq!(memory, [&flush("left"), &flush("right")], "{set:?}");
    for member in &memory {
        assert_eq!(
            (member.kind(), member.granularity()),
            (FairnessKind::Weak, FairnessGranularity::Each)
        );
    }
    let schedulers: Vec<&ProtocolTarget> = set
        .iter()
        .filter(|member| member.origin() == FairnessOrigin::Scheduler)
        .map(ProtocolFairness::target)
        .collect();
    assert_eq!(schedulers.len(), 3, "{set:?}");
    assert_eq!(set.len(), 5, "{set:?}");
    let keys: Vec<Vec<u8>> = set
        .iter()
        .map(|member| {
            quire_canonical::to_vec(member, quire_canonical::Limits::new(4096)).expect("encodes")
        })
        .collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{keys:?}");

    let sc = system!(unit, &[], Sc)
        .fairness_set(unit.clause().fairness())
        .expect("the set builds");
    assert!(
        sc.iter()
            .all(|member| member.origin() != FairnessOrigin::Memory),
        "{sc:?}"
    );
}

/// A reachable state and its steps, each with the index of its post-state.
type Reach<M> = (ProtocolState<M>, Vec<(ProtocolTransition, usize)>);

/// The reachable states of `system` with each state's steps, breadth-first.
fn reached<M: MemoryModel>(system: &ProtocolSystem<'_, M>) -> Vec<Reach<M>> {
    let mut known: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    let mut states: Vec<Reach<M>> = Vec::new();
    for state in system.initial() {
        let key = system.state_key(&state).expect("a key").0;
        known.insert(key, states.len());
        states.push((state, Vec::new()));
    }
    let mut at = 0;
    while at < states.len() {
        let successors = system
            .successors(&states[at].0)
            .expect("expands")
            .successors;
        for (transition, next) in successors {
            let key = system.state_key(&next).expect("a key").0;
            let index = *known.entry(key).or_insert_with(|| {
                states.push((next, Vec::new()));
                states.len() - 1
            });
            states[at].1.push((transition, index));
        }
        at += 1;
    }
    states
}

/// The path to an RA state with a stale load self-loop, plus that loop's
/// destination index. Its fresh load is enabled at the same state.
fn stale_load_lasso(
    system: &ProtocolSystem<'_, Ra>,
) -> (
    Vec<(ProtocolTransition, usize)>,
    usize,
    Vec<ProtocolTransition>,
) {
    let states = reached(system);
    let (looped, load) = states
        .iter()
        .enumerate()
        .find_map(|(at, (_, steps))| {
            steps
                .iter()
                .find(|(transition, next)| {
                    *next == at
                        && transition.node().is_some_and(|node| node.ends_with("Lx"))
                        && transition
                            .memory_choice()
                            .and_then(|choice| choice.read_depth)
                            .is_some_and(|depth| depth > 0)
                })
                .map(|(transition, _)| (at, transition.clone()))
        })
        .expect("a stale load self-loop");
    let read_depth = |transition: &ProtocolTransition| {
        transition
            .memory_choice()
            .and_then(|choice| choice.read_depth)
    };
    assert!(
        states[looped].1.iter().any(|(transition, _)| {
            transition.node() == load.node() && read_depth(transition) == Some(0)
        }),
        "the mo-last load is also enabled at the loop state"
    );

    let mut parent: BTreeMap<usize, (usize, ProtocolTransition)> = BTreeMap::new();
    let mut queue = std::collections::VecDeque::from([0usize]);
    while let Some(at) = queue.pop_front() {
        for (transition, next) in &states[at].1 {
            if *next != 0 && !parent.contains_key(next) {
                parent.insert(*next, (at, transition.clone()));
                queue.push_back(*next);
            }
        }
    }
    let mut stem = Vec::new();
    let mut at = looped;
    while at != 0 {
        let (before, transition) = parent[&at].clone();
        stem.push((transition, at));
        at = before;
    }
    stem.reverse();
    stem.push((load, looped));
    let enabled = states[looped]
        .1
        .iter()
        .map(|(transition, _)| transition.clone())
        .collect();
    (stem, looped, enabled)
}

/// The unflushed lasso over `Spin`: its stem and the index of the state
/// whose `Lx` self-loop is its loop. The path is the breadth-first one to the
/// first state at which `r`'s load leaves the state as it was while `w`'s
/// buffer holds the store.
fn unflushed_lasso(
    system: &ProtocolSystem<'_, Tso>,
) -> (
    Vec<(ProtocolTransition, usize)>,
    usize,
    Vec<ProtocolTransition>,
) {
    let states = reached(system);
    let (looped, load) = states
        .iter()
        .enumerate()
        .find_map(|(at, (_, steps))| {
            let buffered = steps
                .iter()
                .any(|(transition, _)| transition.memory_step().is_some());
            steps
                .iter()
                .find(|(transition, next)| {
                    buffered
                        && *next == at
                        && transition.node().is_some_and(|node| node.ends_with("Lx"))
                })
                .map(|(transition, _)| (at, transition.clone()))
        })
        .expect("a state whose load returns to it");
    let mut parent: BTreeMap<usize, (usize, ProtocolTransition)> = BTreeMap::new();
    let mut queue = std::collections::VecDeque::from([0usize]);
    while let Some(at) = queue.pop_front() {
        for (transition, next) in &states[at].1 {
            if *next != 0 && !parent.contains_key(next) {
                parent.insert(*next, (at, transition.clone()));
                queue.push_back(*next);
            }
        }
    }
    let mut stem = Vec::new();
    let mut at = looped;
    while at != 0 {
        let (before, transition) = parent[&at].clone();
        stem.push((transition, at));
        at = before;
    }
    stem.reverse();
    stem.push((load, looped));
    let enabled = states[looped]
        .1
        .iter()
        .map(|(transition, _)| transition.clone())
        .collect();
    (stem, looped, enabled)
}

/// The first member of `fairness` that the loop taking `taken` at a state
/// enabling `enabled` is unfair under; `None` when the loop is fair.
fn unfair_member<'f>(
    fairness: &'f [ProtocolFairness],
    enabled: &[ProtocolTransition],
    taken: &ProtocolTransition,
) -> Option<&'f ProtocolFairness> {
    let identity = |transition: &ProtocolTransition| -> Result<Identity, ()> {
        enabled
            .iter()
            .position(|candidate| candidate == transition)
            .map(|index| Identity::new(u64::try_from(index).unwrap_or(u64::MAX)))
            .ok_or(())
    };
    let (members, constraints): (Vec<usize>, Vec<_>) =
        protocol_constraints(fairness, enabled, identity)
            .expect("every transition is numbered")
            .into_iter()
            .unzip();
    let step = LoopStep::new(
        enabled
            .iter()
            .map(|transition| identity(transition).expect("every enabled transition is numbered")),
        EdgeLabel::Transition(identity(taken).expect("the taken transition is numbered")),
    );
    match check_loop(&[step], &constraints).expect("a loop has a step") {
        LassoFairness::Fair => None,
        LassoFairness::Unfair { constraint } => Some(&fairness[members[constraint]]),
    }
}

/// ADR-025 MF-1 (TC-673), the filter half: the lasso whose stem is `fork`,
/// `w`'s store and whose loop is `r`'s load with `w`'s buffer `[x := 1]`
/// throughout meets every scheduler constraint, since `w` has no enabled
/// step and `r` steps on every edge, and is rejected by the fairness set only
/// under `flush(w)`'s constraint. Not covered: the AC's `proved` verdict,
/// which is FR-339-AC-7's, and its `refuted` verdict with the memory
/// constraints ignored, which the engine has no switch for.
#[trace("TC-673")]
#[test]
fn the_unflushed_lasso_is_unfair_only_under_the_flush_constraint() {
    let unit = Unit::new(&spin_run(), SETTLES, &IS_ONE);
    let tso = unit.tso();
    let system = system!(unit, &tso, Tso::new(4));
    let (stem, looped, enabled) = unflushed_lasso(&system);
    let (taken, _) = stem.last().expect("a loop step").clone();
    assert_eq!(
        stem.iter().map(|(t, _)| t.kind()).collect::<Vec<_>>()[..2],
        [StepKind::Fork, StepKind::Attempt]
    );
    assert!(looped > 0);
    let w = fsl_eval::simulation::protocol::ThreadPath::root(0).child("Both", "w", 0);
    assert!(
        enabled
            .iter()
            .any(|t| t.memory_step().is_some_and(|step| step.thread == w)),
        "w's flush is enabled throughout the loop: {enabled:?}"
    );
    assert!(
        enabled.iter().all(|t| t.thread() != Some(&w)),
        "w has no enabled step of its own"
    );

    let full = system
        .fairness_set(unit.clause().fairness())
        .expect("the set builds");
    assert_eq!(
        unfair_member(&full, &enabled, &taken),
        Some(&flush("w")),
        "{full:?}"
    );
    let ignored: Vec<ProtocolFairness> = full
        .into_iter()
        .filter(|member| member.origin() != FairnessOrigin::Memory)
        .collect();
    assert_eq!(unfair_member(&ignored, &enabled, &taken), None);
}

/// ADR-025 MF-1 (TC-673), the engine half: with flush fairness in the set,
/// EN-1 over `Spin` finds no fair counterexample to `eventually
/// holds(k.r1 = 1)`: it explores the product and holds with a component
/// certificate. Not covered: the certificate's check, which is the core's.
#[trace("TC-673")]
#[test]
fn eventually_the_load_reads_the_flushed_store() {
    let unit = Unit::new(&spin_run(), SETTLES, &IS_ONE);
    let outcome = unit.check(&unit.tso(), None).expect("the subject checks");
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Component(_),
        stats,
    } = outcome
    else {
        panic!("{outcome:?}");
    };
    assert!(stats.product_states > 0, "{stats:?}");
}

/// FR-228-AC-10 / FR-127-AC-17: ignoring memory fairness may reveal the
/// store-buffer lasso as a separate diagnostic, never a replayable refutation
/// or proof. The default request still takes the component-certificate path.
#[trace("TC-673")]
#[test]
fn ignoring_memory_fairness_returns_a_non_replayable_diagnostic() {
    let unit = Unit::new(&spin_run(), SETTLES, &IS_ONE);
    assert!(matches!(
        unit.check(&unit.tso(), None)
            .expect("the default request checks"),
        ModelCheckOutcome::Holds {
            certificate: ProofCertificate::Component(_),
            ..
        }
    ));

    let outcome = unit
        .check_with_ignored_memory(&unit.tso(), None, true)
        .expect("the diagnostic request checks");
    assert!(
        matches!(
            outcome,
            ModelCheckOutcome::RefutedIgnoringConstraints { .. }
        ),
        "the unflushed lasso is diagnostic only: {outcome:?}"
    );

    let record = settle_protocol_check(
        &outcome,
        &ModelCheckLimits::default(),
        &unit.certificate_request(&unit.tso(), unit.item()),
        &Cancel::new(),
        |_| panic!("a diagnostic lasso must not enter standard replay"),
    );
    assert!(matches!(
        record.value,
        fsl_replay::TerminalValue::Inconclusive(
            fsl_replay::InconclusiveCause::RefutedIgnoringConstraints
        )
    ));
    assert!(record.counterexample.is_none());
}

/// FR-339's checker over a protocol subject: `Spin`'s component certificate
/// is accepted by `check_protocol_components`; the same components reversed
/// are `BackwardEdge` and none at all are `NotPartition`; the closure alone of a liveness proof is refused
/// `LivenessNeedsComponents`, and components offered for a deadlock-freedom
/// item are refused `NotLiveness`.
#[test]
fn a_protocol_component_certificate_is_checked_by_the_protocol_checker() {
    let unit = Unit::new(&spin_run(), SETTLES, &IS_ONE);
    let certificate = unit.certified();
    assert!(certificate.components.len() > 1, "{certificate:?}");
    assert_eq!(
        unit.components(&certificate)
            .expect("the request is checked"),
        TemporalCertificateCheck::Accepted
    );

    let mut reversed = certificate.clone();
    reversed.components.reverse();
    let Ok(TemporalCertificateCheck::Rejected(rejection)) = unit.components(&reversed) else {
        panic!("a reversed order lists a successor before its predecessor");
    };
    assert_eq!(rejection.rule, CertificateRule::BackwardEdge);

    let mut emptied = certificate.clone();
    emptied.components.clear();
    let Ok(TemporalCertificateCheck::Rejected(rejection)) = unit.components(&emptied) else {
        panic!("no components partition nothing");
    };
    assert_eq!(rejection.rule, CertificateRule::NotPartition);

    assert!(matches!(
        check_protocol_closure(
            &unit.certificate_request(&unit.tso(), unit.item()),
            &certificate.closure,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        ),
        Err(ClosureRefusal::LivenessNeedsComponents)
    ));
    assert!(matches!(
        check_protocol_components(
            &unit.certificate_request(&unit.tso(), ModelCheckItem::DeadlockFreedom),
            &certificate,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        ),
        Err(ClosureRefusal::NotLiveness)
    ));
}

/// ADR-025 MF-1 (TC-673): a liveness clause over a `tso` `parallel` that is
/// false even with flush fairness is refuted, and the counterexample carries
/// the memory constraints in its fairness set. `k.r1` never reads 2.
#[trace("TC-673")]
#[test]
fn a_false_liveness_claim_over_tso_is_refuted_with_the_memory_constraints() {
    static IS_TWO: R1 = R1(|value| value == 2);
    let claim = SETTLES.replace("= 1", "= 2");
    let unit = Unit::new(&spin_run(), &claim, &IS_TWO);
    let outcome = unit.check(&unit.tso(), None).expect("the subject checks");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    assert!(counterexample.lasso.is_some());
    assert!(
        counterexample.fairness.contains(&flush("w")),
        "{:?}",
        counterexample.fairness
    );
    unit.replay(&unit.tso(), counterexample.as_ref().clone())
        .expect("the fair lasso replays");

    let ignored_selection = unit
        .check_with_ignored_memory(&unit.tso(), None, true)
        .expect("the diagnostic selection checks");
    let ModelCheckOutcome::Violated {
        counterexample: ignored_counterexample,
        ..
    } = ignored_selection
    else {
        panic!(
            "a lasso satisfying original fairness stays on ordinary replay: {ignored_selection:?}"
        );
    };
    assert!(ignored_counterexample.fairness.contains(&flush("w")));
    unit.replay(&unit.tso(), ignored_counterexample.as_ref().clone())
        .expect("a full-fairness lasso uses ordinary replay");
}

/// ADR-025 MK-2 (TC-672): the unflushed lasso over `Spin`, in which `w`'s buffer
/// stays `[x := 1]` through `r`'s repeated load of `x`, refuses
/// `invalid_runtime_input`/`invalid-value` as unfair under `flush(w)`'s
/// constraint. The RA replay case is covered separately below.
#[trace("TC-672")]
#[test]
fn replay_refuses_the_lasso_that_never_flushes_w() {
    let unit = Unit::new(&spin_run(), SETTLES, &IS_ONE);
    let tso = unit.tso();
    let system = system!(unit, &tso, Tso::new(4));
    let (stem, looped, _) = unflushed_lasso(&system);
    let states = reached(&system);
    let prefix: Vec<ModelStep<ProtocolTransition>> = stem
        .iter()
        .map(|(transition, at)| ModelStep {
            transition: system
                .reported(transition.clone(), &states[*at].0)
                .expect("the step reports"),
            post_state: system.state_key(&states[*at].0).expect("a key").0.into(),
            delay: None,
        })
        .collect();
    assert_eq!(stem.last().map(|(_, at)| *at), Some(looped));
    let payload = TemporalCounterexample {
        initial_state: 0,
        lasso: Some(Lasso {
            entry: prefix.len() - 1,
            terminal_stutter: false,
            terminal_stutter_delay: None,
        }),
        prefix,
        over_binding: Some(core()),
        fairness: system
            .fairness_set(unit.clause().fairness())
            .expect("the set builds"),
        kind: CounterexampleKind::Formula,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: Vec::new(),
    };
    let Err(ReplayRefusal::ModelTrace(refusal)) = unit.replay(&tso, payload) else {
        panic!("an unfair lasso refuses");
    };
    let ModelTraceRefusal::UnfairProtocolLasso { constraint } = refusal.as_ref() else {
        panic!("{refusal:?}");
    };
    assert_eq!(**constraint, flush("w"));
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "invalid-value")
    );
}

/// FR-228-AC-1 and FR-227-AC-4 (TC-673, TC-672): an RA clause carries one
/// strong visibility member. Its stale-load self-loop is refused on replay
/// because the same thread can load the mo-last message at that state.
#[trace("TC-673")]
#[trace("TC-672")]
#[trace("FR-228-AC-1", "FR-227-AC-4")]
#[test]
fn replay_refuses_an_ra_lasso_that_never_loads_the_mo_last_message() {
    let unit = Unit::new(&spin_run(), SETTLES, &IS_ONE);
    let ra = unit.ra();
    let system = system!(unit, &ra, Ra::new(4));
    let (stem, looped, enabled) = stale_load_lasso(&system);
    let (taken, _) = stem.last().expect("a loop step").clone();

    let full = system
        .fairness_set(unit.clause().fairness())
        .expect("the set builds");
    let visibility: Vec<&ProtocolFairness> = full
        .iter()
        .filter(|member| member.origin() == FairnessOrigin::Memory)
        .collect();
    assert_eq!(
        visibility,
        [&ProtocolFairness::visibility(MemoryModelName::Ra)]
    );
    assert_eq!(
        unfair_member(&full, &enabled, &taken),
        visibility.first().copied(),
        "the repeated stale read violates its visibility pair"
    );

    let states = reached(&system);
    let prefix: Vec<ModelStep<ProtocolTransition>> = stem
        .iter()
        .map(|(transition, at)| ModelStep {
            transition: system
                .reported(transition.clone(), &states[*at].0)
                .expect("the step reports"),
            post_state: system.state_key(&states[*at].0).expect("a key").0.into(),
            delay: None,
        })
        .collect();
    assert_eq!(stem.last().map(|(_, at)| *at), Some(looped));
    let payload = TemporalCounterexample {
        initial_state: 0,
        lasso: Some(Lasso {
            entry: prefix.len() - 1,
            terminal_stutter: false,
            terminal_stutter_delay: None,
        }),
        prefix,
        over_binding: Some(core()),
        fairness: full,
        kind: CounterexampleKind::Formula,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: Vec::new(),
    };
    let Err(ReplayRefusal::ModelTrace(refusal)) = unit.replay(&ra, payload) else {
        panic!("an unfair RA lasso refuses");
    };
    let ModelTraceRefusal::UnfairProtocolLasso { constraint } = refusal.as_ref() else {
        panic!("{refusal:?}");
    };
    assert_eq!(
        **constraint,
        ProtocolFairness::visibility(MemoryModelName::Ra)
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "invalid-value")
    );
}

/// The S3 refusal of the `Spin` unit whose clause carries `fairness`.
fn refusal_of(fairness: &str) -> fsl_semantics::check::CheckRefusal {
    let claim = format!(
        "temporal Settles using t0 over (k: M::Core) clock \"model-steps\" \
         on origin {{ {fairness} eventually holds(k.r1 = 1) }}\n"
    );
    let body = contracts() + &protocol("SB", &spin_run()) + &claim;
    let document = document();
    let text = unit_text(&body);
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let parsed = fsl_cst::parse(
        SourceIdentity::new("agent-ix", "litmus"),
        "litmus.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 parses the unit");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = fsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the model selection admits")
    .with_parsed_operation_clauses(|text| {
        fsl_forms::parse_clause_text(text, fsl_cst::Limits::default())
    })
    .expect("the operation clauses parse");
    let mut refusals = fsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "package".to_owned(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(
        CheckMode::Linked,
        filament_semantic_value::checking::CheckingLimits::default(),
        &fsl_eval::value::S6aRewardEvaluator,
    )
    .expect_err("the unit refuses");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    refusals.remove(0)
}

/// ADR-025 MF-1 (TC-673): a `fair weak` constraint naming
/// `flush` refuses `missing_declaration`/`missing-name`, since `flush(b)` is
/// no declared operation or thread target. Not covered: the TP-1 claim's
/// verdicts, whose `ra` half waits on QSL-933 and whose "filter ignores them"
/// half the engine has no switch for.
#[trace("TC-673")]
#[test]
fn a_fair_constraint_naming_flush_is_a_missing_declaration() {
    let refusal = refusal_of("fair weak flush;");
    assert_eq!(
        (refusal.cause.code(), refusal.cause.cause()),
        (Code::MissingDeclaration, Some("missing-name")),
        "{refusal:?}"
    );
}

/// ADR-025 MB-3 and MB-4 over a liveness clause: `w` stores twice, so under
/// `max_store_buffer` 1 its second store is cut at the state where the first
/// is buffered. The run settles `MemoryBoundReached` naming the bound and the
/// bound-limited states in place of a proof, and under the default bound,
/// which cuts nothing, it reaches the proof's component certificate.
#[trace("TC-673")]
#[test]
fn a_liveness_run_that_cut_a_store_settles_inconclusive() {
    let run = spin_run().replace(
        &attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        &format!(
            "{} {}",
            attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
            attempt("Sy", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        ),
    );
    let unit = Unit::new(&run, SETTLES, &IS_ONE);
    let outcome = unit
        .check(&unit.tso(), Some(1))
        .expect("the subject checks");
    let ModelCheckOutcome::Memory {
        cause: MemoryCause::BoundReached { bound, states },
        ..
    } = outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!(
        bound,
        MemoryBound {
            member: MemoryBoundMember::MaxStoreBuffer,
            value: 1
        }
    );
    assert!(states > 0);
    let outcome = unit.check(&unit.tso(), None).expect("no bound reached");
    assert!(
        matches!(
            outcome,
            ModelCheckOutcome::Holds {
                certificate: ProofCertificate::Component(_),
                ..
            }
        ),
        "{outcome:?}"
    );
}

/// `Spin` whose `w` stores twice, so `max_store_buffer` 1 cuts its second
/// store at the state where the first is buffered.
fn double_store_run() -> String {
    spin_run().replace(
        &attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        &format!(
            "{} {}",
            attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
            attempt("Sy", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        ),
    )
}

/// ADR-025 MB-4 over a liveness clause: the claim `eventually holds(k.r1 =
/// 2)` is false (no load reads 2), and its lasso exists under the default
/// bound, but under `max_store_buffer` 1 a store is cut, so the run settles
/// `MemoryBoundReached` in place of the counterexample.
#[trace("TC-673")]
#[test]
fn a_false_liveness_claim_with_a_cut_store_settles_inconclusive() {
    static IS_TWO: R1 = R1(|value| value == 2);
    let claim = SETTLES.replace("= 1", "= 2");
    let unit = Unit::new(&double_store_run(), &claim, &IS_TWO);
    let found = unit.check(&unit.tso(), None).expect("the subject checks");
    assert!(
        matches!(found, ModelCheckOutcome::Violated { .. }),
        "{found:?}"
    );
    let cut = unit
        .check(&unit.tso(), Some(1))
        .expect("the subject checks");
    let ModelCheckOutcome::Memory {
        cause: MemoryCause::BoundReached { bound, states },
        ..
    } = cut
    else {
        panic!("{cut:?}");
    };
    assert_eq!(bound.value, 1);
    assert!(states > 0);
}

/// The model-only pass over a clause that is dead at its initial state still
/// meets the cut store: `eventually holds(k.r1 <= 3)` holds at once, so the
/// product has one state and the cut store is met only by the model-only
/// walk of the dead root, which settles `MemoryBoundReached` all the same.
#[trace("TC-673")]
#[test]
fn a_cut_store_met_only_by_the_model_only_pass_settles_inconclusive() {
    static ANY: R1 = R1(|value| value <= 3);
    let claim = SETTLES.replace("k.r1 = 1", "k.r1 <= 3");
    let unit = Unit::new(&double_store_run(), &claim, &ANY);
    let cut = unit
        .check(&unit.tso(), Some(1))
        .expect("the subject checks");
    let ModelCheckOutcome::Memory {
        cause: MemoryCause::BoundReached { states, .. },
        ..
    } = cut
    else {
        panic!("{cut:?}");
    };
    assert!(states > 0);
    let uncut = unit.check(&unit.tso(), None);
    assert!(
        !matches!(uncut, Ok(ModelCheckOutcome::Memory { .. })),
        "{uncut:?}"
    );
}

/// The claim's atom, undefined once the bound core's `r1` holds 1.
struct UndefinedAtOne;

impl ModelLetters for UndefinedAtOne {
    crate::units::unguarded!();

    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Ok(if field(state, "cores", &instance.key, "r1") == 1 {
            AtomReading::Undefined(fsl_eval::temporal::UndefinedCause::Kernel(
                filament_exact::Undefined::DivisionByZero,
            ))
        } else {
            AtomReading::False
        })
    }
}

static UNDEFINED_AT_ONE: UndefinedAtOne = UndefinedAtOne;

/// FR-128's authored clause through `replay_protocol_trace` (FR-217): the
/// undefined-evaluation counterexample of a protocol clause whose atom has no
/// value once `r1` is 1 replays to `reproduced-with-evaluated-witness` with
/// the undefined value; with `where` moved, another locus or another cause it
/// settles `inconclusive`.
#[trace("TC-539")]
#[test]
fn a_protocol_clauses_undefined_evaluation_replays() {
    use fsl_replay::{DisagreementCause, ModelTraceSettlement, ModelTraceValue, UndefinedLocus};
    let unit = Unit::new(&spin_run(), SETTLES, &UNDEFINED_AT_ONE);
    let outcome = unit.check(&[], None).expect("the subject checks");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    let payload = counterexample.as_ref().clone();
    let CounterexampleKind::UndefinedEvaluation(claimed) = payload.kind.clone() else {
        panic!("expected an undefined evaluation: {payload:?}");
    };
    assert!(matches!(claimed.site.locus, UndefinedLocus::Atom(_)));
    let settle = |payload: TemporalCounterexample<ProtocolTransition>| {
        unit.replay(&[], payload).expect("replays").settlement
    };
    assert_eq!(
        settle(payload.clone()),
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Undefined(claimed.clone()),
            trace_position: payload.trace_position(),
        }
    );
    let altered = |edit: &dyn Fn(&mut fsl_replay::UndefinedEvaluation)| {
        let mut payload = payload.clone();
        let CounterexampleKind::UndefinedEvaluation(evaluation) = &mut payload.kind else {
            panic!("an undefined evaluation");
        };
        edit(evaluation);
        payload
    };
    for payload in [
        altered(&|evaluation| evaluation.site.position += 1),
        altered(&|evaluation| evaluation.site.locus = UndefinedLocus::Activation),
        altered(&|evaluation| {
            evaluation.cause =
                fsl_eval::temporal::UndefinedCause::Kernel(filament_exact::Undefined::IeeeNotFinite)
        }),
    ] {
        assert!(
            matches!(
                settle(payload),
                ModelTraceSettlement::Inconclusive(DisagreementCause::Verdicts { .. })
            ),
            "an altered undefined evaluation is a disagreement"
        );
    }
}

/// `eventually holds(k.r1 = 1)` is true of the program `Sx; Lx` (one thread
/// stores 1 to `x`, then loads it), a constraint `Quiet` (`k.r1 >= 2`, the registers start at 3)
/// is false at the state after `Lx`.
fn store_then_load() -> String {
    format!(
        "sequence Main {{ {} {} }}",
        attempt("Sx", "w", "Cell::set", "", "IsX, StoreOne"),
        attempt("Lx", "c", "Core::read1", "", "ReadsX1, Read1"),
    )
}

/// `Sx` and `Lx` on two threads.
fn store_beside_load() -> String {
    format!(
        "parallel Both {{ branch w sequence W {{ {} }} branch r sequence R {{ {} }} }} \
         join all [w, r];",
        attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        attempt("Lx", "c", "Core::read1", "relaxed", "ReadsX1, Read1"),
    )
}

const QUIET: &str = "constraint Quiet using v on M::Core at current { self.r1 >= 2 }\n";

fn boundary_states(outcome: &ModelCheckOutcome<ProtocolTransition>) -> Option<u64> {
    match outcome {
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::ConstraintReached { boundary_states },
            ..
        } => Some(*boundary_states),
        _ => None,
    }
}

/// FR-158-AC-2 over a protocol subject: `eventually holds(k.r1 = 1)` holds
/// of `Sx; Lx`, and under `Quiet` the only state with `r1 = 1` is a boundary
/// state, which is stored but never expanded, so the run settles
/// `ConstraintReached{1}` in place of `Holds`, under `sc`.
#[trace("TC-580", "FR-158-AC-2")]
#[test]
fn a_protocol_liveness_claim_cut_at_a_constraint_boundary_never_holds() {
    let unit = Unit::new(&store_then_load(), &(QUIET.to_owned() + SETTLES), &IS_ONE);
    let free = unit
        .check_constrained(&[], None, None, None)
        .expect("the subject checks");
    assert!(matches!(free, ModelCheckOutcome::Holds { .. }), "{free:?}");
    let cut = unit
        .check_constrained(&[], Some("Quiet"), None, None)
        .expect("the subject checks");
    assert_eq!(boundary_states(&cut), Some(1), "{cut:?}");
}

/// FR-158-AC-3 over a protocol subject: a constraint false at the initial
/// state makes it a boundary state with no stutter lasso, so the false claim
/// settles `ConstraintReached{1}` after storing one state.
#[trace("TC-581", "FR-158-AC-3")]
#[test]
fn an_initially_false_constraint_stores_one_boundary_state_with_no_lasso() {
    let body = "constraint Started using v on M::Core at current { self.r1 <= 2 }\n";
    let unit = Unit::new(&store_beside_load(), &(body.to_owned() + SETTLES), &IS_ONE);
    let free = unit
        .check_constrained(&[], None, None, None)
        .expect("the subject checks");
    assert!(
        matches!(free, ModelCheckOutcome::Violated { .. }),
        "{free:?}"
    );
    let cut = unit
        .check_constrained(&[], Some("Started"), None, None)
        .expect("the subject checks");
    assert_eq!(boundary_states(&cut), Some(1), "{cut:?}");
    let ModelCheckOutcome::Reduction { stats, .. } = cut else {
        unreachable!();
    };
    assert_eq!(stats.product_states, 1);
}

/// A violation that lies before the boundary still refutes: `Sx` beside
/// `Lx` has a run whose load reads 1 and ends at a terminal stutter loop with
/// `r1 = 1`, which is no boundary state of `NotZero`, though the other
/// interleaving (the load first, `r1 = 0`) reaches a boundary state.
#[trace("TC-580", "FR-158-AC-2")]
#[test]
fn a_protocol_liveness_violation_before_the_boundary_still_refutes() {
    static IS_ZERO: R1 = R1(|value| value == 0);
    let claim = SETTLES.replace("= 1", "= 0")
        + "constraint NotZero using v on M::Core at current { not (self.r1 = 0) }\n";
    let unit = Unit::new(&store_beside_load(), &claim, &IS_ZERO);
    let cut = unit
        .check_constrained(&[], Some("NotZero"), None, None)
        .expect("the subject checks");
    assert!(matches!(cut, ModelCheckOutcome::Violated { .. }), "{cut:?}");
}

/// FR-158-AC-3 over a protocol subject: a constraint undefined at a reached
/// state (`6 mod (1 - r1)` at `r1 = 1`) refutes with `UndefinedEvaluation`
/// at the constraint.
#[trace("TC-581", "FR-158-AC-3")]
#[test]
fn an_undefined_constraint_refutes_a_protocol_liveness_claim() {
    use fsl_replay::UndefinedLocus;
    let body = "function odd_value using v(n: Int[0, 3]): Boolean pure { 6 mod (1 - n) >= 0 }\n\
                constraint Odd using v on M::Core at current { odd_value(self.r1) }\n";
    let text = contracts() + &protocol("SB", &store_then_load()) + body + SETTLES;
    let unit = Unit {
        checked: crate::litmus::package_kernel(&text),
        documents: Documents::new(),
        body: text,
        letters: &IS_ONE,
    };
    let outcome = unit
        .check_constrained(&[], Some("Odd"), None, None)
        .expect("the subject checks");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("{counterexample:?}");
    };
    assert_eq!(
        evaluation.site.locus,
        UndefinedLocus::Constraint("Odd".to_owned())
    );
}

/// FR-158 "Resolution" over a protocol liveness claim: a constraint the
/// package does not declare refuses, naming it.
#[trace("TC-581", "FR-158-AC-3")]
#[test]
fn an_absent_constraint_refuses_a_protocol_liveness_claim() {
    let unit = Unit::new(&store_then_load(), &(QUIET.to_owned() + SETTLES), &IS_ONE);
    let refusal = unit
        .check_constrained(&[], Some("Missing"), None, None)
        .expect_err("an absent constraint refuses");
    assert!(
        matches!(refusal, ModelCheckRefusal::UnknownConstraint { ref name } if name == "Missing"),
        "{refusal:?}"
    );
}

/// FR-158 with FR-225-AC-6 and ADR-025 MB-4: `w` stores to `x` and loads it (so
/// `eventually holds(k.r1 = 1)` holds) while `s` sends twice on a channel. Under `max_messages` 1 the second send is cut and the
/// run settles `ChannelBoundReached`; under the constraint `Zero` the store
/// is a boundary state and `ConstraintReached` settles first, before the
/// channel bound.
#[trace("TC-670", "FR-225-AC-5")]
#[test]
fn constraint_reached_settles_before_a_channel_bound() {
    let run = format!(
        "parallel Both {{ branch w sequence W {{ {} {} }} \
         branch s sequence S {{ send A via ch as (a: Boolean) {{ a }}; \
         send B via ch as (b: Boolean) {{ b }}; }} }} join all [w, s];",
        attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        attempt("Lx", "c", "Core::read1", "relaxed", "ReadsX1, Read1"),
    );
    let channel = "channel ch from w to c carries Boolean ordering unordered delivery unknown \
                   capacity 3 overflow reject;";
    let claim =
        SETTLES.to_owned() + "constraint Zero using v on M::Cell at current { self.v <= 0 }\n";
    let body = contracts() + &protocol_with("SB", channel, &run) + &claim;
    let unit = Unit {
        checked: package(&body),
        documents: Documents::new(),
        body,
        letters: &IS_ONE,
    };
    let cut = unit
        .check_constrained(&[], None, None, Some(1))
        .expect("the subject checks");
    assert!(
        matches!(
            cut,
            ModelCheckOutcome::Memory {
                cause: MemoryCause::ChannelBoundReached { .. },
                ..
            }
        ),
        "{cut:?}"
    );
    let constrained = unit
        .check_constrained(&[], Some("Zero"), None, Some(1))
        .expect("the subject checks");
    assert!(
        boundary_states(&constrained).is_some_and(|n| n > 0),
        "{constrained:?}"
    );
}

/// `s` sends twice on a channel while `r` loads in a loop that never ends.
fn sender_beside_spinner() -> String {
    format!(
        "parallel Both {{ branch s sequence S {{ send A via ch as (a: Boolean) {{ a }}; \
         send B via ch as (b: Boolean) {{ b }}; }} \
         branch r repeat Spin by c visible () while {{ true }} sequence R {{ {} }} }} \
         join all [s, r];",
        attempt("Lx", "c", "Core::read1", "", "ReadsX1, Read1"),
    )
}

const CHANNEL: &str = "channel ch from w to c carries Boolean ordering unordered delivery \
                       exactly-once-premise capacity 3 overflow reject;";

fn channel_unit(run: &str, claim: &str, letters: &'static dyn ModelLetters) -> Unit {
    let body = contracts() + &protocol_with("SB", CHANNEL, run) + claim;
    Unit {
        checked: package(&body),
        documents: Documents::new(),
        body,
        letters,
    }
}

/// ADR-025 MF-4 with FR-225: a send the channel bound cut is enabled to
/// fairness, so the loop of `r` that starves `s` after its first send is
/// unfair and the run settles `ChannelBoundReached`, never `Violated`; with
/// room in the channel `s` finishes and the loop of `r` alone is a genuine
/// fair violation.
#[trace("TC-2986", "FR-225-AC-6")]
#[test]
fn a_loop_that_starves_a_send_cut_by_the_channel_bound_is_unfair() {
    static IS_TWO: R1 = R1(|value| value == 2);
    let claim = SETTLES.replace("= 1", "= 2");
    let unit = channel_unit(&sender_beside_spinner(), &claim, &IS_TWO);
    let cut = unit
        .check_constrained(&[], None, None, Some(1))
        .expect("the subject checks");
    assert!(
        matches!(
            cut,
            ModelCheckOutcome::Memory {
                cause: MemoryCause::ChannelBoundReached { .. },
                ..
            }
        ),
        "{cut:?}"
    );
    let found = unit
        .check_constrained(&[], None, None, None)
        .expect("the subject checks");
    assert!(
        matches!(found, ModelCheckOutcome::Violated { .. }),
        "{found:?}"
    );
}

/// A send cut met only by the model-only pass of a claim that holds at the
/// initial state settles `ChannelBoundReached`, never `Holds`.
#[trace("TC-2986", "FR-225-AC-6")]
#[test]
fn a_send_cut_met_only_by_the_model_only_pass_settles_channel_bound_reached() {
    static ANY: R1 = R1(|value| value <= 3);
    let claim = SETTLES.replace("k.r1 = 1", "k.r1 <= 3");
    let unit = channel_unit(&sender_beside_spinner(), &claim, &ANY);
    let cut = unit
        .check_constrained(&[], None, None, Some(1))
        .expect("the subject checks");
    assert!(
        matches!(
            cut,
            ModelCheckOutcome::Memory {
                cause: MemoryCause::ChannelBoundReached { .. },
                ..
            }
        ),
        "{cut:?}"
    );
}

/// FR-158 "Protocol subjects": the constraint reads memory values under
/// `tso`, so a store still in the buffer is not a boundary state; `Sx`
/// beside `Lx` under `Zero` (`v <= 0`) reaches its boundary only after the
/// flush, so the `tso` run stores more states than the `sc` run (both settle
/// `ConstraintReached`).
#[trace("TC-581", "FR-158-AC-5")]
#[test]
fn a_buffered_store_is_not_a_constraint_boundary_under_tso() {
    static AT_MOST_ONE_LOADED: R1 = R1(|value| value <= 1);
    let run = store_beside_load();
    let claim = SETTLES.replace("k.r1 = 1", "k.r1 <= 1")
        + "constraint Zero using v on M::Cell at current { self.v <= 0 }\n";
    let unit = Unit::new(&run, &claim, &AT_MOST_ONE_LOADED);
    let sc = unit
        .check_constrained(&[], Some("Zero"), None, None)
        .expect("the subject checks");
    let tso = unit
        .check_constrained(&unit.tso(), Some("Zero"), None, None)
        .expect("the subject checks");
    let (
        ModelCheckOutcome::Reduction { stats: on_sc, .. },
        ModelCheckOutcome::Reduction { stats: on_tso, .. },
    ) = (&sc, &tso)
    else {
        panic!("{sc:?} {tso:?}");
    };
    assert!(boundary_states(&sc).is_some() && boundary_states(&tso).is_some());
    assert!(on_tso.product_states > on_sc.product_states);
}

/// ADR-025 MF-4: a store the buffer bound cut is enabled to fairness, so the
/// loop of `r` that never lets `w` store is unfair and the run settles
/// `MemoryBoundReached` in place of `Violated`.
#[trace("TC-673")]
#[test]
fn a_loop_that_starves_a_store_cut_by_the_buffer_bound_is_unfair() {
    let unit = Unit::new(&spin_run(), SETTLES, &IS_ONE);
    let cut = unit
        .check_constrained(&unit.tso(), None, Some(0), None)
        .expect("the subject checks");
    assert!(
        matches!(
            cut,
            ModelCheckOutcome::Memory {
                cause: MemoryCause::BoundReached { .. },
                ..
            }
        ),
        "{cut:?}"
    );
}
