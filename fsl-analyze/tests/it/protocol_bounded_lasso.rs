// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-329 "Bounded-profile clause over a zero-counted loop" (AC-8, AC-9,
//! AC-10; TC-839) over protocol-subject lassos built through FR-217's replay
//! (`replay_protocol_trace_with_temporal_selections`).
//!
//! The zero-counted subject is a `repeat` around a `parallel` whose branches
//! are empty sequences, so the only loop steps are `fork` and `join`
//! (ADR-027 PB-3: neither counts). A protocol system's atoms read the model
//! state, which a fork/join loop does not change, so a replayed lasso cannot
//! make one atom true at one loop position and false at another; the AC-8
//! row for that shape evaluates the lasso's real shape (its steps and their
//! `counts` flags, from the replayed counterexample) with a position-keyed
//! atom.

use std::cell::Cell;
use std::collections::BTreeSet;

use filament_exact::Meter;
use fsl_eval::model_subject::ModelLetters;
use fsl_eval::simulation::model::ExpansionLimits;
use fsl_eval::simulation::protocol::{ProtocolTransition, StepKind};
use fsl_eval::simulation::{SubjectSystem, TransitionSystem};
use fsl_eval::temporal::{
    evaluate_at, AtomReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalRefusal, TemporalTrace,
};
use fsl_foundation::diagnostic::Code;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_replay::{
    replay_protocol_trace_with_temporal_selections, ClosureRule, ClosureStutter,
    CounterexampleKind, DisagreementCause, Lasso, ModelCheckOutcome, ModelStep, ModelTraceRefusal,
    ModelTraceReplayResult, ModelTraceSettlement, ModelTraceSubject, ModelTraceValue,
    ProtocolTraceSubject, ReplayLimits, ReplayRefusal, StepSite, TemporalCounterexample,
};
use fsl_semantics::check::FormulaNodeId;
use ix_trace_rs::trace;

use crate::counter::UNLIMITED;
use crate::fill::{attempt, contracts, document, instance, protocol, universe};
use crate::model_settle_support::{envelope_at, request};
use crate::protocol_counted_steps::{claim, closure_check_over, tp3, Case, CellV};

/// A `repeat` around a `parallel` of two empty branches: the loop is `fork`
/// then `join`.
const FORK_JOIN: &str = "repeat Loop by w visible () while { true } \
     parallel Pa { branch a sequence Ea { } branch b sequence Eb { } } join all [a, b];";

fn fork_join(formula: &str) -> Case {
    let unit = contracts("self.v <= 1") + &protocol("P", "", FORK_JOIN, "d");
    Case::new(&(unit + &claim(formula)))
}

fn is_one(value: i64) -> bool {
    value == 1
}

fn is_zero(value: i64) -> bool {
    value == 0
}

/// `payload` replayed as the first clause of `case`, a protocol `P` subject.
fn replay(
    case: &Case,
    letters: &dyn ModelLetters,
    payload: &TemporalCounterexample<ProtocolTransition>,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    replay_selecting(case, letters, payload, false)
}

/// [`replay`] with the item's `stutter_closure` as the request's subject
/// carries it.
fn replay_selecting(
    case: &Case,
    letters: &dyn ModelLetters,
    payload: &TemporalCounterexample<ProtocolTransition>,
    stutter_closure: bool,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let unit = crate::fill::unit_text(&case.body).into_bytes();
    let node = &case.checked.graph().temporal_clauses()[0];
    let clause_node = node.identity().clone();
    let occurrence = OccurrenceKey::new(clause_node.clone(), node.claim().clone());
    let protocol = if case.checked.graph().protocol("Echo").is_some() {
        "Echo"
    } else {
        "P"
    };
    let subject = case.subject(protocol);
    replay_protocol_trace_with_temporal_selections(
        request(
            &case.checked,
            &case.temporal_clock_selections,
            &document(),
            case.documents.snapshots.iter(),
            &unit,
        ),
        &ProtocolTraceSubject {
            model: ModelTraceSubject {
                timed: None,
                initial: case.initial.clone(),
                universes: universe(&["c"]),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
                stutter_closure,
            },
            protocol: protocol.to_owned(),
            over: instance("c"),
            roles: subject.roles,
            capacities: Vec::new(),
            memory: subject.memory,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        },
        &envelope_at(payload.clone(), clause_node, occurrence),
        letters,
        &case.temporal_clock_selections,
        ReplayLimits::default(),
    )
}

/// The counterexample EN-1 finds for `case`'s claim under `letters`.
fn refutation(
    case: &Case,
    letters: &dyn ModelLetters,
) -> TemporalCounterexample<ProtocolTransition> {
    let protocol = if case.checked.graph().protocol("Echo").is_some() {
        "Echo"
    } else {
        "P"
    };
    match case.check(protocol, letters) {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample.as_ref().clone(),
        other => panic!("expected a violation, got {:?}", other.stats()),
    }
}

/// The replay of `payload` reproduced a violation at `position`.
fn assert_reproduced_at(result: Result<ModelTraceReplayResult, ReplayRefusal>, position: u64) {
    match result.expect("the lasso replays").settlement {
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position,
        } => assert_eq!(trace_position, Some(position)),
        other => panic!("expected a reproduction at {position}, got {other:?}"),
    }
}

/// The replay evaluated the clause `true` on the lasso, so it disagrees with
/// the refutation it was handed.
fn assert_clause_holds(result: Result<ModelTraceReplayResult, ReplayRefusal>) {
    use filament_semantic_value::catalog::Category;
    match result.expect("the lasso replays").settlement {
        ModelTraceSettlement::Inconclusive(DisagreementCause::Verdicts { replayed, .. }) => {
            assert_eq!(
                replayed,
                fsl_replay::Verdict::from_category(Category::Success)
            );
        }
        other => panic!("expected a true evaluation, got {other:?}"),
    }
}

/// The zero-counted `fork`/`join` lasso EN-1 finds for `eventually[1,1]
/// holds(k.v = 1)` (`p` false at every position), asserted to be one.
fn zero_counted_lasso() -> (Case, TemporalCounterexample<ProtocolTransition>) {
    let case = fork_join("eventually[1,1] holds(k.v = 1)");
    let payload = refutation(&case, &CellV(is_one));
    let lasso = payload.lasso.as_ref().expect("a lasso");
    assert_eq!((lasso.entry, lasso.terminal_stutter), (0, false));
    let kinds: Vec<StepKind> = payload
        .prefix
        .iter()
        .map(|step| step.transition.kind())
        .collect();
    assert!(!kinds.is_empty());
    assert!(
        kinds
            .iter()
            .all(|kind| matches!(kind, StepKind::Fork | StepKind::Join)),
        "{kinds:?}"
    );
    assert!(payload.prefix.iter().all(|step| !step.transition.counts()));
    (case, payload)
}

/// Letters for the lasso of `payload` read by position: `p` holds at the
/// positions `at` names, each step counts as the replayed transition says.
struct LassoLetters {
    counted: Vec<bool>,
    p: fn(u64) -> bool,
    reads: Cell<u64>,
}

impl LassoLetters {
    fn of(payload: &TemporalCounterexample<ProtocolTransition>, p: fn(u64) -> bool) -> Self {
        Self {
            counted: payload
                .prefix
                .iter()
                .map(|step| step.transition.counts())
                .collect(),
            p,
            reads: Cell::new(0),
        }
    }

    /// The lasso's shape: no stem, the whole prefix is the loop.
    fn trace(&self) -> TemporalTrace {
        TemporalTrace::Lasso {
            prefix: 0,
            loop_len: u64::try_from(self.counted.len()).expect("a small loop"),
        }
    }
}

impl TemporalLetters for LassoLetters {
    fn atom(
        &mut self,
        _atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        self.reads.set(self.reads.get() + 1);
        Ok(if (self.p)(position.index()) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        panic!("these claims have no `when` condition");
    }

    fn counts_step(&self, position: TemporalPosition) -> bool {
        let step = usize::try_from(position.index()).expect("a small position") - 1;
        self.counted[step]
    }

    fn counts_loop_edge(&self) -> bool {
        *self.counted.last().expect("a loop")
    }
}

fn at_origin(case: &Case, letters: &mut LassoLetters) -> Result<TemporalOutcome, TemporalRefusal> {
    evaluate_at(
        case.clause(),
        letters.trace(),
        TemporalPosition::new(0),
        letters,
        &mut Meter::new(UNLIMITED),
    )
}

/// FR-329-AC-8: over a fair protocol lasso whose loop is only `fork` and
/// `join`, entered at interval distance 0 and replayed under a bounded
/// profile: with `p` false at every position `eventually[1,1] holds(p)` is
/// `Completed(false)` at position 0; with `p` true at position 0
/// `eventually[0,1] holds(p)` is `Completed(true)`; and `eventually[1,1]`
/// stays false even where `p` holds at every position, because no position
/// lies one counted step away. `p` true only at a loop position, at
/// distance 0, `eventually[0,1] holds(p)` is `Completed(true)`.
#[trace("TC-839", "FR-329-AC-8")]
#[test]
fn a_zero_counted_protocol_lasso_decides_intervals_in_counted_steps() {
    let (false_case, payload) = zero_counted_lasso();
    assert!(
        !payload.fairness.is_empty(),
        "the default scheduler constraints bind the lasso: it is a fair one"
    );

    // `p` false at every position: no position at distance 1.
    assert_reproduced_at(replay(&false_case, &CellV(is_one), &payload), 0);

    // `p` true at position 0 (and every position): `[0,1]` is met at distance
    // 0, `[1,1]` is not, since the loop never reaches distance 1.
    let met = fork_join("eventually[0,1] holds(k.v = 0)");
    assert_clause_holds(replay(&met, &CellV(is_zero), &payload));
    let unreachable = fork_join("eventually[1,1] holds(k.v = 0)");
    assert_reproduced_at(replay(&unreachable, &CellV(is_zero), &payload), 0);

    // `p` true only at a loop position: positions of the replayed lasso, the
    // steps' real `counts` flags.
    let loop_len = payload.prefix.len();
    assert!(loop_len >= 2, "a fork and a join");
    let later = fork_join("eventually[0,1] holds(k.v = 1)");
    let mut letters = LassoLetters::of(&payload, |position| position == 1);
    assert_eq!(
        at_origin(&later, &mut letters),
        Ok(TemporalOutcome::Holds),
        "position 1 is at distance 0 from position 0"
    );
    let beyond = fork_join("eventually[1,1] holds(k.v = 1)");
    let mut letters = LassoLetters::of(&payload, |position| position == 1);
    assert_eq!(
        at_origin(&beyond, &mut letters),
        Ok(TemporalOutcome::Violated {
            at: TemporalPosition::new(0)
        })
    );
}

/// The body of a protocol whose loop takes one counted step per pass.
fn counted_loop(infinite: bool) -> Case {
    let run = format!(
        "repeat Loop by w visible () while {{ true }} {}",
        attempt("T", "touch", "TouchPre", "t", "true")
    );
    let unit = contracts("self.v <= 1") + &protocol("P", "", &run, "d");
    let claim = if infinite {
        "temporal Claim using t0 over (k: M::Cell) clock \"model-steps\" on origin \
         { always eventually holds(k.v = 1) }\n"
            .to_owned()
    } else {
        claim("eventually[0,1] holds(k.v = 1)")
    };
    Case::new(&(unit + &claim))
}

/// FR-329-AC-9: under a bounded profile the evaluator refuses a lasso that
/// is not zero-counted, `unsupported_construct`/`expression-form`, reading no
/// atom and giving no truth value: a lasso whose steps all count (an
/// observed lasso, and a model subject's, whose every step counts, ADR-027
/// PB-3), and a protocol lasso whose loop has one counted step, whether the
/// step is in the loop or closes it. The zero-counted loop of AC-8 is read.
#[trace("TC-839", "FR-329-AC-9")]
#[test]
fn a_bounded_clause_refuses_a_lasso_that_is_not_zero_counted() {
    let (case, zero) = zero_counted_lasso();
    let mut letters = LassoLetters::of(&zero, |_| true);
    assert!(at_origin(&case, &mut letters).is_ok());

    let refused = |counted: Vec<bool>| {
        let mut letters = LassoLetters {
            counted,
            p: |_| true,
            reads: Cell::new(0),
        };
        let result = at_origin(&case, &mut letters);
        let Err(refusal @ TemporalRefusal::LassoUnderBoundedProfile { .. }) = result else {
            panic!("expected a refusal, got {result:?}");
        };
        let code = refusal.catalog_code();
        assert_eq!(
            (code.code(), code.cause()),
            ("unsupported_construct", "expression-form")
        );
        assert_eq!(letters.reads.get(), 0, "no atom is read: no truth value");
    };
    // Observed and model-subject lassos: every step counts.
    refused(vec![true, true, true]);
    // A protocol lasso with one counted step in its loop, and with the
    // counted step closing it.
    refused(vec![false, true, false]);
    refused(vec![false, false, true]);
}

/// FR-329-AC-9, the replay of the protocol lasso whose loop has one counted
/// step (FR-217-AC-7): the `touch` loop's lasso, found for the same package
/// under the infinite-trace profile, replayed against the bounded clause
/// gives no value.
#[trace("TC-839", "TC-662", "FR-329-AC-9", "FR-217-AC-7")]
#[test]
fn replaying_a_counted_protocol_lasso_under_a_bounded_clause_gives_no_value() {
    let infinite = counted_loop(true);
    let payload = refutation(&infinite, &CellV(is_one));
    let lasso = payload.lasso.as_ref().expect("a lasso");
    assert_eq!(
        payload.prefix[lasso.entry..]
            .iter()
            .filter(|step| step.transition.counts())
            .count(),
        1,
        "exactly one loop step counts"
    );
    assert!(
        !payload.fairness.is_empty(),
        "the loop meets the default scheduler constraints"
    );

    let bounded = counted_loop(false);
    // The bounded clause carries the package's scheduler set as the lasso does.
    let result = replay(&bounded, &CellV(is_one), &payload).expect("the lasso replays");
    match result.settlement {
        ModelTraceSettlement::Inconclusive(DisagreementCause::NoValue { .. }) => {}
        other => panic!("expected no value, got {other:?}"),
    }
}

/// A counted touch loop can starve an enabled sibling under adversarial
/// scheduling. Bounded replay has no value when fair, but checks the default
/// package's scheduler constraints before evaluating the bounded clause.
#[trace("TC-839", "TC-662", "FR-329-AC-9", "FR-217-AC-7")]
#[test]
fn replaying_a_counted_protocol_lasso_refuses_an_unfair_scheduler_loop() {
    let case = |infinite: bool, scheduling: &str| {
        let run = format!(
            "parallel Both {{ branch spinning repeat Loop by w visible () while {{ true }} {} \
             branch waiting {} }} join all [spinning, waiting];",
            attempt("T", "touch", "TouchPre", "t", "true"),
            attempt("A", "inc", "IncPre, IncPost", "a", "true")
        );
        let unit = contracts("self.v <= 1") + &protocol("P", scheduling, &run, "d");
        let formula = if infinite {
            "temporal Claim using t0 over (k: M::Cell) clock \"model-steps\" on origin \
             { always eventually holds(k.v = 1) }\n"
                .to_owned()
        } else {
            claim("eventually[0,1] holds(k.v = 1)")
        };
        Case::new(&(unit + &formula))
    };
    let infinite = case(true, "scheduling adversarial;");
    let payload = refutation(&infinite, &CellV(is_one));
    let lasso = payload.lasso.as_ref().expect("a lasso");
    assert_eq!(
        payload.prefix[lasso.entry..]
            .iter()
            .filter(|step| step.transition.counts())
            .count(),
        1,
        "the loop takes one touch while the sibling waits"
    );
    let bounded = case(false, "scheduling adversarial;");
    let result = replay(&bounded, &CellV(is_one), &payload).expect("the fair lasso replays");
    assert!(
        matches!(
            result.settlement,
            ModelTraceSettlement::Inconclusive(DisagreementCause::NoValue { .. })
        ),
        "a fair counted loop has no bounded truth value: {:?}",
        result.settlement
    );

    let default = case(false, "");
    assert_unfair_with_default_fairness(&default, &payload);
}

/// Carry the default package's fairness set, then require an unfairness refusal
/// naming one of its scheduler constraints rather than a bounded no-value result.
fn assert_unfair_with_default_fairness(
    default: &Case,
    payload: &TemporalCounterexample<ProtocolTransition>,
) {
    let stale = replay(default, &CellV(is_one), payload);
    let Err(ReplayRefusal::ModelTrace(stale)) = stale else {
        panic!("expected a fairness set refusal, got {stale:?}");
    };
    let ModelTraceRefusal::ProtocolFairnessSet { recompiled, .. } = *stale else {
        panic!("expected a fairness set refusal, got {stale:?}");
    };
    assert!(!recompiled.is_empty());
    let mut carried = payload.clone();
    carried.fairness.clone_from(&recompiled);
    let unfair = replay(default, &CellV(is_one), &carried);
    let Err(error) = unfair else {
        panic!("expected an unfair lasso, got {unfair:?}");
    };
    let ReplayRefusal::ModelTrace(refusal) = &error else {
        panic!("expected a model-trace refusal, got {error:?}");
    };
    let ModelTraceRefusal::UnfairProtocolLasso { constraint } = refusal.as_ref() else {
        panic!("expected an unfair protocol lasso, got {refusal:?}");
    };
    assert!(
        recompiled.contains(constraint),
        "the named constraint is one of the package's: {constraint:?}"
    );
    assert_eq!(error.code(), Code::InvalidRuntimeInput);
    assert_eq!(error.cause(), Some("invalid-value"));
}

/// FR-329-AC-10: under a bounded profile, a zero-counted protocol lasso that
/// is unfair for a scheduler constraint refuses `invalid_runtime_input`/
/// `invalid-value` naming the constraint; the same lasso replayed against the
/// package declaring `scheduling adversarial` is fair, and with `p` false at
/// every position `eventually[1,1] holds(p)` is `Completed(false)` at
/// position 0.
///
/// Echo sends once and rests at Rcv. Its concrete duplicate/lose loop
/// advances delivery identity and closes only in the unread cycle key.
#[trace("TC-839", "FR-329-AC-10")]
#[test]
fn an_unfair_zero_counted_lasso_refuses_naming_the_scheduler_constraint() {
    let adversarial = echo("scheduling adversarial;", "eventually[1,1] holds(k.v = 1)");
    let payload = echo_lasso(&adversarial);
    let lasso = payload.lasso.as_ref().expect("a lasso");
    let entry = &payload.prefix[lasso.entry - 1].post_state;
    let concrete_last = &payload.prefix.last().expect("loop post-state").post_state;
    assert_ne!(
        entry, concrete_last,
        "the concrete ProtocolKey comparison must reject this loop before fairness"
    );
    let protocol = crate::fill::concrete_sc_system(&adversarial.subject("Echo"));
    let mut concrete = protocol
        .try_initial()
        .expect("initial states settle")
        .remove(0);
    let mut states = Vec::new();
    for kind in [StepKind::Send, StepKind::Duplicate, StepKind::Lose] {
        let (_, next) = protocol
            .successors(&concrete)
            .expect("expansion")
            .successors
            .into_iter()
            .find(|(step, _)| step.kind() == kind)
            .expect("enabled Echo step");
        states.push(next.clone());
        concrete = next;
    }
    let identity_read = BTreeSet::from(["Echo::ch".to_owned()]);
    assert_ne!(
        protocol.cycle_key(&states[0], &identity_read).unwrap(),
        protocol.cycle_key(&states[2], &identity_read).unwrap(),
        "the adverse identity-reading closure rejects before scheduler fairness"
    );
    assert!(payload.fairness.is_empty(), "no scheduler constraint");
    assert!(payload.prefix[lasso.entry..]
        .iter()
        .all(|step| !step.transition.counts()));
    assert_eq!(
        payload.prefix[lasso.entry..]
            .iter()
            .map(|step| step.transition.kind())
            .collect::<Vec<_>>(),
        [StepKind::Duplicate, StepKind::Lose]
    );

    // The same lasso against the default scheduler: the first replay names
    // the set the package recompiles, the second carries it.
    let default = echo("", "eventually[1,1] holds(k.v = 1)");
    assert_unfair_with_default_fairness(&default, &payload);

    // Against the adversarial package it is fair; `p` false throughout, the
    // distance-1 window holds no `p`.
    assert_reproduced_at(replay(&adversarial, &CellV(is_one), &payload), 0);
}

fn echo(scheduling: &str, formula: &str) -> Case {
    Case::new(&(echo_body(scheduling) + &claim(formula)))
}

fn echo_infinite(scheduling: &str, formula: &str) -> Case {
    let clause = claim(formula).replace("using tb over", "using t0 over");
    Case::new(&(echo_body(scheduling) + &clause))
}

const ECHO_RUN: &str = "sequence Main { send Snd via ch as (s: Boolean) { s }; \
                       receive Rcv via ch of Snd as (r: Boolean) { r }; }";

pub(crate) fn echo_body(scheduling: &str) -> String {
    let members = format!(
        "channel ch from w to w carries Boolean ordering fifo by (m: Boolean) \
                   {{ true }} delivery unknown capacity 2 overflow block; {scheduling}"
    );
    contracts("self.v <= 1") + &protocol("Echo", &members, ECHO_RUN, "d")
}

/// Build the witness from enabled concrete transitions, without model search.
fn echo_lasso(case: &Case) -> TemporalCounterexample<ProtocolTransition> {
    let system = crate::fill::concrete_sc_system(&case.subject("Echo"));
    let mut state = system
        .try_initial()
        .expect("initial states settle")
        .remove(0);
    let mut prefix = Vec::new();
    let mut entry = None;
    for kind in [StepKind::Send, StepKind::Duplicate, StepKind::Lose] {
        let (transition, next) = system
            .successors(&state)
            .expect("expansion")
            .successors
            .into_iter()
            .find(|(step, _)| step.kind() == kind)
            .expect("enabled Echo step");
        let post_state = system.canonical_state_key(&next).expect("concrete key");
        if kind == StepKind::Send {
            entry = Some(next.clone());
        }
        prefix.push(ModelStep {
            transition,
            post_state,
            delay: None,
        });
        state = next;
    }
    let entry = entry.expect("send entry");
    assert_ne!(
        system.canonical_state_key(&entry).unwrap(),
        system.canonical_state_key(&state).unwrap()
    );
    assert_eq!(
        system.cycle_key(&entry, &BTreeSet::new()).unwrap(),
        system.cycle_key(&state, &BTreeSet::new()).unwrap()
    );
    TemporalCounterexample {
        initial_state: 0,
        prefix,
        over_binding: Some(instance("c")),
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso: Some(Lasso {
            entry: 1,
            terminal_stutter: false,
            terminal_stutter_delay: None,
        }),
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: Vec::new(),
    }
}

/// The explicit read-set seam is exercised here because the checked language
/// currently has no delivery-index-reading obligation form.
#[trace("TC-650", "FR-205-AC-10", "FR-205-AC-11")]
#[test]
fn echo_cycle_keys_preserve_read_delivery_labels_and_pair_unread_steps() {
    let case = echo_infinite("scheduling adversarial;", "always holds(k.v = 0)");
    let system = crate::fill::concrete_sc_system(&case.subject("Echo"));
    let step = |state: &fsl_eval::simulation::protocol::ProtocolState<
        fsl_eval::simulation::protocol::Sc,
    >,
                kind| {
        system
            .successors(state)
            .unwrap()
            .successors
            .into_iter()
            .find(|(transition, _)| transition.kind() == kind)
            .expect("enabled step")
    };
    let (_, s1) = step(
        &system.try_initial().expect("initial states settle")[0],
        StepKind::Send,
    );
    let (duplicate, both) = step(&s1, StepKind::Duplicate);
    assert_eq!(duplicate.delivery().unwrap().index(), 1);
    let losses: Vec<_> = system
        .successors(&both)
        .unwrap()
        .successors
        .into_iter()
        .filter(|(transition, _)| transition.kind() == StepKind::Lose)
        .collect();
    assert_eq!(losses.len(), 2);
    let s2 = &losses[0].1; // remove original delivery 0
    let s3 = &losses[1].1; // remove duplicate delivery 1
    let raw: BTreeSet<_> = [&s1, s2, s3]
        .into_iter()
        .map(|state| system.canonical_state_key(state).unwrap())
        .collect();
    assert_eq!(raw.len(), 3);
    let unread = BTreeSet::new();
    for state in [&s1, s2, s3] {
        assert_eq!(
            system.cycle_key(state, &unread).unwrap(),
            system.canonical_state_key(&s1).unwrap()
        );
        assert_eq!(crate::fill::v(state.model(), "c"), 0);
    }
    let (next_duplicate, _) = step(s2, StepKind::Duplicate);
    assert_eq!(
        next_duplicate.delivery().unwrap().index(),
        2,
        "concrete indices are never reused"
    );
    let reads = BTreeSet::from(["Echo::ch".to_owned()]);
    for state in [&s1, s2, s3] {
        assert_eq!(
            system.cycle_key(state, &reads).unwrap(),
            system.canonical_state_key(state).unwrap()
        );
    }
    assert_ne!(
        system.cycle_key(&s1, &reads).unwrap(),
        system.cycle_key(s2, &reads).unwrap()
    );

    let left = system.successors(&s1).unwrap().successors;
    let right = system.successors(s2).unwrap().successors;
    assert_eq!(left.len(), right.len());
    assert_eq!(left.len(), 3, "duplicate, lose and receive");
    for (transition, post) in &left {
        let (paired, paired_post) = right
            .iter()
            .find(|(candidate, _)| candidate.kind() == transition.kind())
            .unwrap();
        assert_eq!(transition.node(), paired.node());
        assert_eq!(transition.counts(), paired.counts());
        match (transition.binder_record(), paired.binder_record()) {
            (
                Some(filament_exact::Value::Boolean(left)),
                Some(filament_exact::Value::Boolean(right)),
            ) => assert_eq!(left, right),
            (None, None) => assert_ne!(transition.kind(), StepKind::Receive),
            records => panic!("paired Echo steps bind different payloads: {records:?}"),
        }
        assert_eq!(
            system.cycle_key(post, &unread).unwrap(),
            system.cycle_key(paired_post, &unread).unwrap()
        );
        assert_eq!(
            crate::fill::v(post.model(), "c"),
            crate::fill::v(paired_post.model(), "c")
        );
    }

    let reading_system = system.with_cycle_keys(reads);
    assert_ne!(
        reading_system.loop_state_key(&s1).unwrap(),
        reading_system.loop_state_key(s2).unwrap()
    );
    let explored = fsl_analyze::model_check::retain(
        &reading_system,
        fsl_eval::simulation::Limits::default(),
        fsl_foundation::IdentityLimits::default(),
        Some(8),
        || false,
    )
    .unwrap();
    assert!(
        matches!(
            explored.outcome,
            fsl_eval::simulation::Outcome::BoundReached { depth: 8, .. }
        ),
        "{:?}",
        explored.outcome
    );
}

/// FR-205-AC-12: reading message identity preserves the message ordinal while
/// unread delivery indices are still canonicalized, leaving finite EN-1
/// exploration.
#[trace("TC-650", "FR-205-AC-12")]
#[test]
fn echo_message_read_set_keeps_ordinals_and_canonicalizes_delivery_indices() {
    let case = echo_infinite("scheduling adversarial;", "always holds(k.v = 0)");
    let system = crate::fill::concrete_sc_system(&case.subject("Echo"));
    let step = |state: &fsl_eval::simulation::protocol::ProtocolState<
        fsl_eval::simulation::protocol::Sc,
    >,
                kind| {
        system
            .successors(state)
            .unwrap()
            .successors
            .into_iter()
            .find(|(transition, _)| transition.kind() == kind)
            .expect("enabled step")
    };
    let (_, s1) = step(
        &system.try_initial().expect("initial states settle")[0],
        StepKind::Send,
    );
    let (_, both) = step(&s1, StepKind::Duplicate);
    let losses: Vec<_> = system
        .successors(&both)
        .unwrap()
        .successors
        .into_iter()
        .filter(|(transition, _)| transition.kind() == StepKind::Lose)
        .collect();
    assert_eq!(losses.len(), 2);
    let s2 = &losses[0].1;
    let s3 = &losses[1].1;
    let message_reads = BTreeSet::from(["Echo::ch".to_owned()]);
    let no_delivery_reads = BTreeSet::new();

    let concrete_keys: BTreeSet<_> = [&s1, s2, s3]
        .into_iter()
        .map(|state| system.canonical_state_key(state).unwrap())
        .collect();
    assert_eq!(
        concrete_keys.len(),
        3,
        "concrete delivery identities differ"
    );
    let initial_cycle_key = system
        .canonical_state_key(&s1)
        .expect("first queued message key");
    for state in [&s1, s2, s3] {
        assert_eq!(
            system
                .cycle_key_with_reads(state, &message_reads, &no_delivery_reads)
                .unwrap(),
            initial_cycle_key,
            "message ordinal is preserved and unread delivery labels are quotiented"
        );
    }

    let message_read_system = system.with_cycle_read_sets(message_reads, no_delivery_reads);
    let explored = fsl_analyze::model_check::retain(
        &message_read_system,
        fsl_eval::simulation::Limits::default(),
        fsl_foundation::IdentityLimits::default(),
        Some(8),
        || false,
    )
    .unwrap();
    assert!(
        matches!(
            explored.outcome,
            fsl_eval::simulation::Outcome::Exhaustive(_)
        ),
        "message-only reads still yield finite exploration: {:?}",
        explored.outcome
    );

    // Reuse the same message-only read set over an unbounded repeat. The
    // source path is unchanged, so successive sends have occurrence ordinals
    // 0 and 1. Delivery labels remain unread and are still quotiented.
    let loop_channel = "channel ch from w to w carries Boolean ordering fifo by (m: Boolean) \
                        { true } delivery unknown capacity 2 overflow block;";
    let loop_run = "repeat Loop by w visible () while { true } sequence Body { \
                    send Snd via ch as (s: Boolean) { s }; \
                    receive Rcv via ch of Snd as (r: Boolean) { r }; }";
    let loop_body = contracts("self.v <= 1")
        + &protocol("Loop", loop_channel, loop_run, "d")
        + &claim("always holds(k.v = 0)").replace("using tb over", "using t0 over");
    let loop_case = Case::new(&loop_body);
    let loop_system = crate::fill::concrete_sc_system(&loop_case.subject("Loop"));
    let initial = loop_system
        .try_initial()
        .expect("initial states settle")
        .remove(0);
    let (send0, sent0) = loop_system
        .successors(&initial)
        .unwrap()
        .successors
        .into_iter()
        .find(|(transition, _)| transition.kind() == StepKind::Send)
        .expect("first repeated send");
    let message0 = send0.message().expect("message occurrence");
    assert_eq!(message0.ordinal(), 0);
    let (_, duplicated) = loop_system
        .successors(&sent0)
        .unwrap()
        .successors
        .into_iter()
        .find(|(transition, _)| transition.kind() == StepKind::Duplicate)
        .expect("duplicate the first message");
    let (_, after_loss) = loop_system
        .successors(&duplicated)
        .unwrap()
        .successors
        .into_iter()
        .find(|(transition, _)| transition.kind() == StepKind::Lose)
        .expect("lose one concrete delivery");
    assert_ne!(
        loop_system.canonical_state_key(&sent0).unwrap(),
        loop_system.canonical_state_key(&after_loss).unwrap(),
        "the concrete delivery index differs after duplicate/lose"
    );
    let loop_message_reads = BTreeSet::from(["Loop::ch".to_owned()]);
    let no_delivery_reads = BTreeSet::new();
    let first_occurrence_key = loop_system
        .cycle_key_with_reads(&sent0, &loop_message_reads, &no_delivery_reads)
        .unwrap();
    assert_eq!(
        loop_system
            .cycle_key_with_reads(&after_loss, &loop_message_reads, &no_delivery_reads)
            .unwrap(),
        first_occurrence_key,
        "unread delivery indices canonicalize even when message identity is read"
    );
    let (_, received0) = loop_system
        .successors(&after_loss)
        .unwrap()
        .successors
        .into_iter()
        .find(|(transition, _)| transition.kind() == StepKind::Receive)
        .expect("receive the remaining copy");
    let (send1, sent1) = loop_system
        .successors(&received0)
        .unwrap()
        .successors
        .into_iter()
        .find(|(transition, _)| transition.kind() == StepKind::Send)
        .expect("second repeated send");
    let message1 = send1.message().expect("message occurrence");
    assert_eq!(message1.node(), message0.node(), "same static send node");
    assert_eq!(
        message1.path(),
        message0.path(),
        "same repeated thread path"
    );
    assert_eq!(message1.ordinal(), 1);
    let second_occurrence_key = loop_system
        .cycle_key_with_reads(&sent1, &loop_message_reads, &no_delivery_reads)
        .unwrap();
    assert_ne!(
        first_occurrence_key, second_occurrence_key,
        "the message-only read set preserves occurrence ordinals 0 and 1"
    );
    let (_, received1) = loop_system
        .successors(&sent1)
        .unwrap()
        .successors
        .into_iter()
        .find(|(transition, _)| transition.kind() == StepKind::Receive)
        .expect("receive the second message");
    let loop_keys = [
        loop_system
            .cycle_key_with_reads(&initial, &loop_message_reads, &no_delivery_reads)
            .unwrap(),
        loop_system
            .cycle_key_with_reads(&received0, &loop_message_reads, &no_delivery_reads)
            .unwrap(),
        loop_system
            .cycle_key_with_reads(&received1, &loop_message_reads, &no_delivery_reads)
            .unwrap(),
    ];
    assert_eq!(
        loop_keys.iter().collect::<BTreeSet<_>>().len(),
        3,
        "received ordinals 0 and 1 stay distinct from the initial state"
    );
    let loop_read_system = loop_system.with_cycle_read_sets(loop_message_reads, no_delivery_reads);
    let bounded = fsl_analyze::model_check::retain(
        &loop_read_system,
        fsl_eval::simulation::Limits::default(),
        fsl_foundation::IdentityLimits::default(),
        Some(8),
        || false,
    )
    .unwrap();
    assert!(
        matches!(
            bounded.outcome,
            fsl_eval::simulation::Outcome::BoundReached { depth: 8, .. }
        ),
        "retained message ordinals have the specified finite-depth outcome: {:?}",
        bounded.outcome
    );
}

/// FR-205-AC-15: received-message bounds and unread message ordinals disappear
/// from cycle keys, while the concrete send transitions retain distinct
/// occurrence ordinals. EN-1 proves the unchanged model claim by closure.
#[trace("TC-650", "FR-205-AC-15")]
#[test]
fn loop_received_message_ordinals_canonicalize_and_en1_closes() {
    let channel = "channel ch from w to w carries Boolean ordering fifo by (m: Boolean) \
                   { true } delivery exactly-once-premise capacity 1 overflow block;";
    let run = "repeat Loop by w visible () while { true } sequence Body { \
               send Snd via ch as (s: Boolean) { s }; \
               receive Rcv via ch of Snd as (r: Boolean) { r }; }";
    let body = contracts("self.v <= 1")
        + &protocol("Loop", channel, run, "d")
        + &claim("always holds(k.v = 0)").replace("using tb over", "using t0 over");
    let case = Case::new(&body);
    let system = crate::fill::concrete_sc_system(&case.subject("Loop"));
    let mut state = system
        .try_initial()
        .expect("initial states settle")
        .remove(0);
    let mut loop_states = vec![state.clone()];
    let mut sent_states = Vec::new();
    let mut send_identities = Vec::new();
    for _ in 0..2 {
        let (send, sent) = system
            .successors(&state)
            .unwrap()
            .successors
            .into_iter()
            .find(|(transition, _)| transition.kind() == StepKind::Send)
            .expect("the repeated send is enabled");
        send_identities.push(send.message().expect("send message").ordinal());
        sent_states.push(sent.clone());
        let (_, received) = system
            .successors(&sent)
            .unwrap()
            .successors
            .into_iter()
            .find(|(transition, _)| transition.kind() == StepKind::Receive)
            .expect("the queued message is received");
        state = received;
        loop_states.push(state.clone());
    }

    assert_eq!(send_identities, [0, 1], "concrete sends remain distinct");
    let concrete_keys: BTreeSet<_> = loop_states
        .iter()
        .map(|state| system.canonical_state_key(state).unwrap())
        .collect();
    assert_eq!(
        concrete_keys.len(),
        3,
        "received-message next-delivery bounds and ordinal counters differ concretely"
    );
    let initial_cycle_key = system.loop_state_key(&loop_states[0]).unwrap();
    for state in &loop_states {
        assert_eq!(
            system.loop_state_key(state).unwrap(),
            initial_cycle_key,
            "unread message ordinal and received-message bounds are canonicalized"
        );
    }
    let sent_keys: BTreeSet<_> = sent_states
        .iter()
        .map(|state| system.canonical_state_key(state).unwrap())
        .collect();
    assert_eq!(sent_keys.len(), 2, "the concrete sent occurrences differ");
    let sent_cycle_key = system.loop_state_key(&sent_states[0]).unwrap();
    for state in &sent_states {
        assert_eq!(
            system.loop_state_key(state).unwrap(),
            sent_cycle_key,
            "post-send states retain one canonical queued-message identity"
        );
    }

    let ModelCheckOutcome::Holds {
        certificate: fsl_replay::ProofCertificate::Closure(_),
        stats,
    } = case.check("Loop", &CellV(is_zero))
    else {
        panic!("EN-1 should close the unread message loop");
    };
    assert!(stats.product_states < 100, "finite closure: {stats:?}");
}

#[trace("TC-650", "TC-525", "FR-205-AC-10", "FR-338-AC-26")]
#[test]
fn echo_unread_claim_has_a_finite_checked_cycle_closure() {
    use fsl_replay::{
        check_protocol_closure, ModelCheckItem, ModelCheckLimits, ProofCertificate,
        ProtocolCertificateRequest, TemporalCertificateCheck,
    };
    let case = echo_infinite("scheduling adversarial;", "always holds(k.v = 0)");
    let outcome = case.check("Echo", &CellV(is_zero));
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Closure(certificate),
        stats,
    } = outcome
    else {
        panic!("expected a finite closure: {outcome:?}");
    };
    assert!(
        stats.product_states < 100,
        "the unread delivery history is finite: {stats:?}"
    );
    let request = ProtocolCertificateRequest {
        temporal_clock_selections: &case.temporal_clock_selections,
        subject: case.subject("Echo"),
        item: ModelCheckItem::Clause {
            clause: case.clause(),
            letters: &CellV(is_zero),
        },
        max_store_buffer: None,
        max_messages: None,
        max_live_instances: None,
    };
    let check = |certificate: &fsl_replay::ClosureCertificate| {
        check_protocol_closure(
            &request,
            certificate,
            &ModelCheckLimits::default(),
            &filament_exact::Cancel::new(),
        )
        .unwrap()
    };
    assert_eq!(check(&certificate), TemporalCertificateCheck::Accepted);
    let payload = echo_lasso(&case);
    let canonical = &payload.prefix[0].post_state;
    let concrete = &payload.prefix[2].post_state;
    let mut incorrect = certificate.clone();
    let mut replaced = 0;
    for state in &mut incorrect.states {
        if state.model_state == *canonical {
            state.model_state = concrete.clone();
            replaced += 1;
        }
    }
    assert!(replaced > 0, "the certificate contains the cycle entry");
    assert!(
        matches!(check(&incorrect), TemporalCertificateCheck::Rejected(ref rejection)
        if rejection.rule == fsl_replay::CertificateRule::SuccessorMissing),
        "{:?}",
        check(&incorrect)
    );
}

#[trace("TC-839", "FR-329-AC-10", "FR-217-AC-7")]
#[test]
fn echo_en1_counterexample_replays_with_concrete_delivery_indices() {
    let case = echo("scheduling adversarial;", "eventually[1,1] holds(k.v = 1)");
    let payload = refutation(&case, &CellV(is_one));
    assert_reproduced_at(replay(&case, &CellV(is_one), &payload), 2);
}

#[trace("TC-650", "FR-205-AC-13")]
#[test]
fn echo_infinite_trace_search_and_component_certificate_use_cycle_keys() {
    let original = echo_infinite("scheduling adversarial;", "always holds(k.v = 0)");
    let body = original
        .body
        .replace("using tb over", "using t0 over")
        .replace("always holds(k.v = 0)", "always eventually holds(k.v = 0)");
    let case = Case::new(&body);
    let outcome = case.check("Echo", &CellV(is_zero));
    let ModelCheckOutcome::Holds { certificate, stats } = outcome else {
        panic!("the TP-4 claim holds on the finite quotient: {outcome:?}");
    };
    assert!(stats.product_states < 100, "{stats:?}");
    assert!(
        matches!(certificate, fsl_replay::ProofCertificate::Component(_)),
        "{certificate:?}"
    );
    let request = fsl_replay::ProtocolCertificateRequest {
        temporal_clock_selections: &case.temporal_clock_selections,
        subject: case.subject("Echo"),
        item: fsl_replay::ModelCheckItem::Clause {
            clause: case.clause(),
            letters: &CellV(is_zero),
        },
        max_store_buffer: None,
        max_messages: None,
        max_live_instances: None,
    };
    let fsl_replay::ProofCertificate::Component(certificate) = certificate else {
        unreachable!()
    };
    assert_eq!(
        fsl_replay::check_protocol_components(
            &request,
            &certificate,
            &fsl_replay::ModelCheckLimits::default(),
            &filament_exact::Cancel::new()
        )
        .unwrap(),
        fsl_replay::TemporalCertificateCheck::Accepted
    );
    let failing = Case::new(&body.replace(
        "always eventually holds(k.v = 0)",
        "eventually holds(k.v = 1)",
    ));
    let payload = refutation(&failing, &CellV(is_one));
    assert!(
        payload.lasso.is_some(),
        "the infinite-trace refutation is a lasso"
    );
    assert_reproduced_at(replay(&failing, &CellV(is_one), &payload), 0);
}

#[trace("TC-650", "FR-205-AC-10", "FR-205-AC-13")]
#[test]
fn echo_cycle_edges_lift_to_never_reused_concrete_delivery_indices() {
    let case = echo("scheduling adversarial;", "eventually[1,1] holds(k.v = 1)");
    let raw = crate::fill::concrete_sc_system(&case.subject("Echo"));
    let canonical =
        crate::fill::concrete_sc_system(&case.subject("Echo")).with_cycle_keys(BTreeSet::new());
    let mut payload = echo_lasso(&case);
    payload.prefix.truncate(1);
    let (_, mut concrete) = raw
        .successors(&raw.try_initial().expect("initial states settle")[0])
        .unwrap()
        .successors
        .into_iter()
        .find(|(step, _)| step.kind() == StepKind::Send)
        .unwrap();
    let entry = concrete.clone();
    for expected in [1, 2, 3] {
        let (duplicate, both) = canonical
            .successors(&entry)
            .unwrap()
            .successors
            .into_iter()
            .find(|(step, _)| step.kind() == StepKind::Duplicate)
            .unwrap();
        let (lose, closed) = canonical
            .successors(&both)
            .unwrap()
            .successors
            .into_iter()
            .find(|(step, _)| step.kind() == StepKind::Lose)
            .unwrap();
        for (step, target) in [(&duplicate, &both), (&lose, &closed)] {
            let (actual, next) =
                SubjectSystem::concrete_cycle_step(&canonical, &concrete, step, target)
                    .unwrap()
                    .expect("the canonical edge lifts");
            if actual.kind() == StepKind::Duplicate {
                assert_eq!(actual.delivery().unwrap().index(), expected);
            }
            payload.prefix.push(ModelStep {
                transition: actual,
                post_state: raw.canonical_state_key(&next).unwrap(),
                delay: None,
            });
            concrete = next;
        }
    }
    assert_ne!(
        payload.prefix.first().unwrap().post_state,
        payload.prefix.last().unwrap().post_state
    );
    assert_reproduced_at(replay(&case, &CellV(is_one), &payload), 0);
    let mut stale = payload.clone();
    stale.prefix[3].transition = stale.prefix[1].transition.clone();
    let error = replay(&case, &CellV(is_one), &stale)
        .expect_err("a reused duplicate identity is not enabled");
    assert_eq!(error.code(), Code::InvalidRuntimeInput);
    assert_eq!(error.cause(), Some("invalid-value"));
    assert!(
        matches!(error, ReplayRefusal::ModelTrace(ref refusal)
        if matches!(refusal.as_ref(), ModelTraceRefusal::ProtocolNotEnabled { step: 3, .. })),
        "{error:?}"
    );
}

#[trace("TC-650", "FR-205-AC-11")]
#[test]
fn echo_unordered_cycle_keys_renumber_nonconsecutive_copies() {
    let original = echo_infinite("scheduling adversarial;", "always holds(k.v = 0)");
    let case = Case::new(&original.body.replace(
        "ordering fifo by (m: Boolean) { true }",
        "ordering unordered",
    ));
    assert!(matches!(
        case.checked.graph().protocol("Echo").unwrap().channels[0].ordering,
        fsl_semantics::check::CheckedOrdering::Unordered
    ));
    let system = crate::fill::concrete_sc_system(&case.subject("Echo"));
    let take = |state: &fsl_eval::simulation::protocol::ProtocolState<
        fsl_eval::simulation::protocol::Sc,
    >,
                kind,
                indices: &[u64]| {
        system
            .successors(state)
            .unwrap()
            .successors
            .into_iter()
            .find(|(step, next)| {
                step.kind() == kind
                    && queue_delivery_indices(&system.canonical_state_key(next).unwrap()) == indices
            })
            .expect("the required concrete queue is reachable")
            .1
    };
    let s0 = system
        .try_initial()
        .expect("initial states settle")
        .remove(0);
    let s1 = take(&s0, StepKind::Send, &[0]);
    let both = take(&s1, StepKind::Duplicate, &[0, 1]);
    let one = take(&both, StepKind::Lose, &[1]);
    let both = take(&one, StepKind::Duplicate, &[1, 2]);
    let one = take(&both, StepKind::Lose, &[1]);
    let both = take(&one, StepKind::Duplicate, &[1, 3]);
    let normalized = system
        .delivery_canonical_state(&both, &BTreeSet::new())
        .unwrap();
    assert_eq!(
        queue_delivery_indices(&system.canonical_state_key(&normalized).unwrap()),
        [0, 1]
    );
    let expected = take(&s1, StepKind::Duplicate, &[0, 1]);
    assert_eq!(
        system.canonical_state_key(&normalized).unwrap(),
        system.canonical_state_key(&expected).unwrap(),
        "the next-delivery bound is 2 too"
    );
}

#[trace("TC-650", "FR-205-AC-10")]
#[test]
fn echo_pending_interval_send_keeps_its_delivery_labels_and_bound() {
    let original = echo_infinite("scheduling adversarial;", "always holds(k.v = 0)");
    // The interval-send rule needs a concurrent receiver to free capacity.
    // This is the same pending-send shape as protocol_delivery_interval.
    let concurrent = "parallel Main { \
        branch sending send Snd via ch as (s: Boolean) { s }; \
        branch receiving receive Rcv via ch of sending::Snd as (r: Boolean) { r }; \
        } join all [sending, receiving];";
    let case = Case::new(
        &original
            .body
            .replace("delivery unknown", "delivery [3,3]")
            .replace(ECHO_RUN, concurrent),
    );
    let system = crate::fill::concrete_sc_system(&case.subject("Echo"));
    let (_, forked) = system
        .successors(&system.try_initial().expect("initial states settle")[0])
        .unwrap()
        .successors
        .into_iter()
        .find(|(step, _)| step.kind() == StepKind::Fork)
        .expect("fork sender and receiver");
    let (_, pending) = system
        .successors(&forked)
        .unwrap()
        .successors
        .into_iter()
        .find(|(step, _)| step.kind() == StepKind::Send)
        .expect("partial send fills capacity 2");
    assert_eq!(
        queue_delivery_indices(&system.canonical_state_key(&pending).unwrap()),
        [0, 1]
    );
    let (_, received) = system
        .successors(&pending)
        .unwrap()
        .successors
        .into_iter()
        .find(|(step, next)| {
            step.kind() == StepKind::Receive
                && queue_delivery_indices(&system.canonical_state_key(next).unwrap()) == [1]
        })
        .expect("receive original while the interval send is pending");
    for state in [&pending, &received] {
        assert_eq!(
            system.cycle_key(state, &BTreeSet::new()).unwrap(),
            system.canonical_state_key(state).unwrap(),
            "pending sends retain labels and their next-delivery counter"
        );
    }
}

fn queue_delivery_indices(key: &fsl_eval::simulation::StateKey) -> Vec<u64> {
    fn collect(value: &serde_json::Value, indices: &mut Vec<u64>) {
        match value {
            serde_json::Value::Object(members) => {
                if members.get("type").and_then(|value| value.as_str()) == Some("queue-entry") {
                    indices.push(members["delivery"].as_str().unwrap().parse().unwrap());
                } else {
                    for value in members.values() {
                        collect(value, indices);
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    collect(value, indices);
                }
            }
            _ => {}
        }
    }
    let mut indices = Vec::new();
    collect(
        &serde_json::from_slice(key.as_bytes()).unwrap(),
        &mut indices,
    );
    indices
}

/// FR-128-AC-11: a protocol subject has no closure alphabet, so a listed
/// closure step refuses at its `/transition/type` through the production
/// protocol replay entry, even for an item that selects `stutter_closure`,
/// and before any step runs: the same payload with the step removed and an
/// initial state outside the subject refuses on that state instead.
#[trace("TC-523", "FR-128-AC-11")]
#[test]
fn a_protocol_subject_refuses_a_closure_step_before_executing_the_prefix() {
    let (case, mut payload) = zero_counted_lasso();
    payload.initial_state = 99;
    let letters = CellV(is_one);
    let control = replay(&case, &letters, &payload);
    assert!(
        matches!(
            control,
            Err(ReplayRefusal::ModelTrace(ref refusal))
                if matches!(**refusal, ModelTraceRefusal::InitialState { .. })
        ),
        "{control:?}"
    );
    payload.closure_stutters = vec![ClosureStutter {
        site: StepSite::Loop(0),
        post_state: payload.prefix[0].post_state.clone(),
        delay: None,
    }];
    for selected in [false, true] {
        let Err(ReplayRefusal::ModelTrace(refusal)) =
            replay_selecting(&case, &letters, &payload, selected)
        else {
            panic!("a protocol subject refuses the closure step");
        };
        assert_eq!(
            *refusal,
            ModelTraceRefusal::ClosureStutter {
                pointer: "/loop/0/transition/type".to_owned(),
                rule: ClosureRule::Alphabet,
            }
        );
    }
}

/// FR-338-AC-31 (`Echo`): the `on each` clause `holds(k.v = 0) implies
/// eventually[1,2] holds(k.v = 0)` has open debt only at an activation after
/// the origin, because the counted `send` discharges the origin's. Its
/// `duplicate` and `lose` steps are uncounted, so the loop they form keeps the
/// later activations' debt open. The certificate holding every reached product
/// state of the run is offered under two requests. The default package's root
/// scheduler constraint never takes the loop while `Rcv` stays enabled, so the
/// loop is unfair and the core accepts it, FR-127 settling `proved`,
/// `Certified`. Under `scheduling adversarial` the loop is fair: the engine
/// returns `Violated` with prefix `send` and loop `duplicate`, `lose`, which
/// the production replay refutes, and the same certificate is rejected
/// `WitnessFails` at the byte-least fair product state, the one send,
/// duplicate and receive reach, not at the origin, which a checker that starts
/// the negated root only at the origin would accept wrongly. `max_states` one below the closure stops `LimitReached{MaxStates}`.
#[trace("TC-525", "FR-338-AC-31")]
#[trace("TC-525", "FR-127-AC-1")]
#[test]
fn an_on_each_tp3_debt_after_the_origin_is_judged_by_the_fair_uncounted_loop() {
    use fsl_replay::{
        CertificateLocus, CertificateRejection, CertificateRule, Certification, ModelCheckLimit,
        ModelCheckLimits, ProofBasis, ProofCertificate, ResultLabel, TemporalCertificateCheck,
        TerminalValue,
    };
    let each = "holds(k.v = 0) implies eventually[1,2] holds(k.v = 0)";
    let build =
        |scheduling: &str| Case::new(&(echo_body(scheduling) + &tp3("on each (q: M::Cell)", each)));
    let letters = CellV(is_zero);

    let fair = build("");
    let outcome = fair.check("Echo", &letters);
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Closure(certificate),
        ..
    } = &outcome
    else {
        panic!(
            "the default scheduler excludes the loop: {:?}",
            outcome.stats()
        );
    };
    assert_eq!(
        closure_check_over(&fair, "Echo", &letters, certificate),
        TemporalCertificateCheck::Accepted
    );
    assert_eq!(
        fair.record("Echo", &letters, &outcome).value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );

    let adversarial = build("scheduling adversarial;");
    let outcome = adversarial.check("Echo", &letters);
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!(
            "the adversarial scheduler admits the loop: {:?}",
            outcome.stats()
        );
    };
    let lasso = counterexample.lasso.as_ref().expect("a lasso");
    let (stem, cycle) = counterexample.prefix.split_at(lasso.entry);
    assert_eq!(
        stem.iter()
            .map(|step| step.transition.kind())
            .collect::<Vec<_>>(),
        [StepKind::Send]
    );
    assert_eq!(
        cycle
            .iter()
            .map(|step| step.transition.kind())
            .collect::<Vec<_>>(),
        [StepKind::Duplicate, StepKind::Lose]
    );
    assert_eq!(
        adversarial.label("Echo", &letters, &outcome),
        ResultLabel::Refuted
    );

    let TemporalCertificateCheck::Rejected(CertificateRejection {
        rule: CertificateRule::WitnessFails,
        at: CertificateLocus::ProductState(at),
    }) = closure_check_over(&adversarial, "Echo", &letters, certificate)
    else {
        panic!("the fair loop keeps a later activation's debt open");
    };
    let subject = adversarial.subject("Echo");
    let system = crate::fill::concrete_sc_system(&subject).with_cycle_keys(BTreeSet::new());
    let origin = system
        .try_initial()
        .expect("initial states settle")
        .remove(0);
    let step = |from, kind| {
        system
            .successors(from)
            .expect("expansion")
            .successors
            .into_iter()
            .find(|(step, _)| step.kind() == kind)
            .unwrap_or_else(|| panic!("a {kind:?} step is enabled"))
            .1
    };
    let sent = step(&origin, StepKind::Send);
    let copied = step(&sent, StepKind::Duplicate);
    // The least passing product state: the receive that follows the copy
    // leaves the receiver done with the copy still queued, whose key orders
    // before every state that still has the receive pending.
    let received = step(&copied, StepKind::Receive);
    let key = |state| {
        let (bytes, _) = system.state_key(state).expect("encodes");
        fsl_eval::simulation::StateKey::from_canonical_bytes(bytes)
    };
    assert_ne!(at.model_state, key(&origin), "not at the origin");
    assert_eq!(
        at.model_state,
        key(&received),
        "at the byte-least product state of a fair component"
    );
    let monitors_at_least_fair_model_state = certificate
        .states
        .iter()
        .filter(|state| state.model_state == at.model_state)
        .map(|state| state.automaton_state.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        monitors_at_least_fair_model_state.len(),
        1,
        "the least fair model state has one reachable monitor state in this fixture"
    );
    let only_monitor = monitors_at_least_fair_model_state
        .first()
        .expect("the locus is a certified product state");
    assert_eq!(
        &at.automaton_state, only_monitor,
        "the unique monitor state at the least fair model state"
    );

    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = certificate.states.len() - 1;
    assert_eq!(
        fsl_replay::check_protocol_closure(
            &fsl_replay::ProtocolCertificateRequest {
                temporal_clock_selections: &fair.temporal_clock_selections,
                subject: fair.subject("Echo"),
                item: fsl_replay::ModelCheckItem::Clause {
                    clause: fair.clause(),
                    letters: &letters,
                },
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
            },
            certificate,
            &limits,
            &filament_exact::Cancel::new(),
        )
        .expect("checked"),
        TemporalCertificateCheck::Stopped(fsl_replay::IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: u64::try_from(certificate.states.len() - 1).expect("a small closure"),
            required: None,
        })
    );
}

/// FR-128 (`Echo`): the `on each` clause is false from the activation after
/// the counted `send`, not from the origin, whose debt the `send` discharges.
/// Replaying the adversarial lasso reproduces the violation at position 1,
/// the activating position where the clause is false, not at position 0.
#[trace("TC-523", "FR-128-AC-16")]
#[test]
fn an_on_each_lasso_replays_at_the_activation_where_the_clause_is_false() {
    let each = "holds(k.v = 0) implies eventually[1,2] holds(k.v = 0)";
    let case =
        Case::new(&(echo_body("scheduling adversarial;") + &tp3("on each (q: M::Cell)", each)));
    let letters = CellV(is_zero);
    let payload = refutation(&case, &letters);
    assert_eq!(
        payload.trace_position(),
        Some(0),
        "the record keeps the lasso origin"
    );
    assert_reproduced_at(replay(&case, &letters, &payload), 1);
}
