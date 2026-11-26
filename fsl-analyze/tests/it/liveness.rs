// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126's liveness phase (TP-4) and its weak-fairness filter over the
//! `Counter` subject (`crate::counter`) and its `setTo` variant: the
//! product's SCC phase, the fairness filter, the canonical lasso, the
//! terminal stutter loop, the limits and the refusals of what is not built.
//!
//! Every expected loop below is derived by hand from the model's arithmetic
//! and FR-101's canonical order (see each test), and every lasso is also
//! checked by an oracle written here that reads only the model's successor
//! relation: the loop closes, the claim is false on it, and each weak
//! constraint is taken in it or disabled somewhere in it.
//!
//! ADR-018 §6's ConfigVersion units and FR-126-AC-4's `Health` unit are
//! `example_units`. Not covered here: the `holds` atoms are read through the
//! test's own [`crate::counter::Letters`]; the lasso's replay is FR-128's.

use crate::liveness_support::{
    applies, assert_weakly_fair, counter, fixture, fixture_in_mode, instance, is_three, is_zero,
    lasso_of, violated, Fixture,
};
use filament_exact::{Cancel, CancelCause, Meter};
use fsl_analyze::model_check::{Product, ProductTransition, PropertyComponent};
use fsl_eval::simulation::{ExpansionStop, TransitionSystem};
use fsl_eval::temporal::{
    evaluate, AtomReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalTrace, UndefinedCause,
};
use fsl_replay::{
    check_components_over_stutter_closure, ComponentWitness, CounterexampleKind, IncompleteCause,
    Lasso, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, ReachedLimit,
    TemporalCertificateCheck, UndefinedLocus,
};
use fsl_semantics::check::FormulaNodeId;
use fsl_semantics::model::observation::AnchorKind;
use ix_trace_rs::trace;

use crate::counter::{document, document_with_ranged_parameter, value, CONTRACT, UNLIMITED, ZERO};

/// FR-126-AC-2: `eventually p` is already true when its sole initial state
/// satisfies `p`. The component checker must accept the producer's proof.
#[trace("TC-521", "FR-126-AC-2")]
#[test]
fn eventually_true_at_the_initial_state_proves_for_an_admitted_object() {
    let one_object = counter(document(false), "", "eventually holds(c.value <= 1)");
    let _ = one_object.certified(|value| Some(value <= 1));
}

/// FR-125-AC-7: an empty universe has no instance and must settle
/// inconclusive, without asking the closure checker to validate a proof.
#[trace("TC-520", "FR-125-AC-7")]
#[test]
fn an_empty_object_population_is_no_instance_not_a_closure_fault() {
    let no_object = fixture(
        document(false),
        CONTRACT,
        "",
        "eventually holds(c.value <= 1)",
        &[],
    );
    let cause = match no_object.decide(|value| Some(value <= 1)).expect("checked") {
        ModelCheckOutcome::Undecided { cause, .. } => cause,
        other => panic!("expected NoInstance for an empty object set, got {other:?}"),
    };
    assert_eq!(cause, fsl_replay::UndecidedCause::NoInstance);
}

/// FR-126-AC-3's last sentence, with the deadlock half in
/// `model_check::deadlock_freedom_is_violated_at_the_unintended_terminal_state`:
/// over the `Counter` subject (no `terminal` declaration), `always
/// eventually holds(c.value = 0)` under infinite-trace is violated by the
/// lasso whose loop is the terminal stutter step at value 3, with the
/// stutter marker set. The one behaviour is `0, 1, 2, 3, 3, ...`, so the
/// stem is `0 -inc-> 1 -inc-> 2 -inc-> 3` and the loop has no model step.
#[trace("TC-521", "FR-126-AC-3")]
#[test]
fn a_behaviour_that_ends_is_the_lasso_whose_loop_is_the_terminal_stutter_step() {
    let fixture = counter(document(false), "", "always eventually holds(c.value = 0)");
    let outcome = fixture.decide(is_zero).expect("checked");
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(fixture.values(counterexample, "c"), [0, 1, 2, 3]);
    assert_eq!(
        counterexample.lasso,
        Some(Lasso {
            entry: 3,
            terminal_stutter: true,
            terminal_stutter_delay: None,
        })
    );
    // The stem's three `inc` steps and the stutter step of the loop.
    assert_eq!(counterexample.len(), 4);
    assert_eq!(counterexample.prefix.len(), 3);
    assert_eq!(counterexample.initial_state, 0);
    assert_eq!(counterexample.over_binding, Some(instance("c")));
    assert_eq!(counterexample.trace_position(), Some(0));
    assert!(counterexample
        .prefix
        .iter()
        .all(|step| applies(step, "inc")));
}

/// FR-126 "Second phase": the first phase's states are the product's: a
/// terminal state's stutter edge is a product edge, so the single behaviour
/// `0, 1, 2, 3, 3, ...` is a path of the product. `always eventually
/// holds(c.value = 3)` holds on it, so no component passes and the run holds
/// with FR-339's component certificate, which the core's checker accepts.
/// The negation `eventually always not p` is a two-state automaton whose
/// second state reads `not p`, so the product holds 3 + 3 states at values 0
/// to 2 and, at value 3 where `p` holds, the first automaton state alone: 7.
/// The second state has no move at value 3, but that model state is also a
/// product state, so the model-only pass finds nothing left to read.
#[test]
fn a_run_with_no_fair_accepting_component_holds_with_a_component_certificate() {
    let fixture = counter(document(false), "", "always eventually holds(c.value = 3)");
    let (certificate, stats) = fixture.certified(is_three);
    assert_eq!(stats.product_states, 7);
    assert_eq!(stats.automaton_states, 2);
    assert_eq!(stats.depth, 3);
    assert_eq!(certificate.closure.states.len(), 7);
    // Six states are components of their own; the terminal stutter cycle at
    // value 3 holds no state of the accepting set, so it is the last.
    let witnesses: Vec<_> = certificate
        .components
        .iter()
        .map(|component| (component.states.len(), &component.witness))
        .collect();
    assert_eq!(
        witnesses,
        [(1, &ComponentWitness::Trivial); 6]
            .into_iter()
            .chain([(1, &ComponentWitness::MissingAcceptance { set: 0 })])
            .collect::<Vec<_>>()
    );
}

/// FSpec FR-380: the component certificate of the plain graph, accepted by
/// the core there, is not accepted over the stutter closure. At values 0 to 2
/// the closure adds a self-loop, which the plain certificate's `Trivial`
/// witnesses ("no edge to itself") deny, so a proof the core did not check
/// over the closure is never reported as one it did.
#[test]
fn a_certificate_valid_only_over_the_plain_graph_is_rejected_over_the_stutter_closure() {
    let fixture = counter(document(false), "", "always eventually holds(c.value = 3)");
    let (certificate, _) = fixture.certified(is_three);
    let letters = fixture.letters(is_three);
    let request = fixture.certificate_request(&letters, UNLIMITED);
    let over_closure = check_components_over_stutter_closure(
        &request,
        &certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the request is checked");
    assert!(
        matches!(over_closure, TemporalCertificateCheck::Rejected(_)),
        "{over_closure:?}"
    );
}

/// ADR-018 UE-1: a model state reached only where the negation automaton
/// has no move still has its letter read. `eventually holds(c.value = 1)`
/// negates to `always not p`, which has no move at value 1, so values 2 and
/// 3 are reached by no product state: the product holds value 0 alone and
/// counts one state and no edge. The model-only pass reads the rest: with
/// the atom undefined at value 2 the claim is refuted at position 2 by the
/// prefix `0 -inc-> 1 -inc-> 2`, and with it defined everywhere the run is
/// the run holds over the same one product state.
#[test]
fn positions_past_a_dead_automaton_are_still_read() {
    let fixture = counter(document(false), "", "eventually holds(c.value = 1)");
    let outcome = fixture
        .decide(|v| match v {
            1 => Some(true),
            2 => None,
            _ => Some(false),
        })
        .expect("checked");
    let counterexample = violated(&outcome);
    assert_eq!(fixture.values(counterexample, "c"), [0, 1, 2]);
    assert_eq!(counterexample.lasso, None);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.position, 2);
    assert_eq!(outcome.stats().product_states, 1);

    // The model-only pass is charged to the run's budgets: with none left it
    // stops the run naming the limit instead of walking the model.
    let stops = |limits: ModelCheckLimits| match fixture
        .run(|v| Some(v == 1), None, limits, UNLIMITED, &Cancel::new())
        .expect("checked")
    {
        ModelCheckOutcome::Stopped { cause, limit, .. } => (cause, limit),
        other => panic!("expected a stopped run, got {other:?}"),
    };
    let mut states = ModelCheckLimits::default();
    states.limits.max_states = 1;
    assert_eq!(
        stops(states),
        (
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit {
                limit: ModelCheckLimit::MaxStates,
                value: 1
            })
        )
    );
    let mut edges = ModelCheckLimits::default();
    edges.limits.max_transitions = 0;
    assert_eq!(
        stops(edges),
        (
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit {
                limit: ModelCheckLimit::MaxTransitions,
                value: 0
            })
        )
    );

    let (certificate, stats) = fixture.certified(|v| Some(v == 1));
    // The one edge leads to a model state with no product state.
    assert_eq!((stats.product_states, stats.product_edges), (1, 0));
    assert_eq!(certificate.closure.states.len(), 1);
}

/// FR-126 "Explicit-state limits": `max_states` and `max_transitions` are the
/// run's, not each instance's. Over two counters a clause binds two
/// instances, each its own product; with `max_states` 20 the run stops
/// naming 20 and the products together hold at most 20 states, and a run
/// with no such limit reports more. The negation automaton of both
/// instances is one automaton: two states, not two per instance.
#[test]
fn the_state_budget_is_shared_by_every_instance() {
    // `p` holds everywhere, so the negation has no run and nothing is
    // violated: each instance's product is the whole model.
    let formula = "always eventually holds(c.value = 3)";
    let always = |_: i64| Some(true);
    let one = fixture(document(false), CONTRACT, "", formula, &[("a", 0)]);
    let alone = one.certified(always).1;
    let both = fixture(
        document(false),
        CONTRACT,
        "",
        formula,
        &[("a", 0), ("b", 0)],
    );
    let together = both.certified(always).1;
    assert!(together.product_states > 20, "{together:?}");
    assert_eq!(together.automaton_states, alone.automaton_states);

    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 20;
    let outcome = both
        .run(always, None, limits, UNLIMITED, &Cancel::new())
        .expect("checked");
    let ModelCheckOutcome::Stopped { limit, stats, .. } = outcome else {
        panic!("expected a stopped run, got {outcome:?}");
    };
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxStates,
            value: 20
        })
    );
    assert!(stats.product_states <= 20, "{stats:?}");
}

/// FR-126 "Fairness filter" and "Counterexamples" over the Counter with
/// `setTo(n)`, whose post-state is any value in `0..=3` (it has no `post`),
/// so every value reaches every value and `inc` also steps `v -> v + 1` for
/// `v < 3`. `always eventually holds(c.value = 3)` is violated by a run
/// that stays in `0..=2`; the product's accepting component is
/// `{(0, q1), (1, q1), (2, q1)}` whose first state `(0, q1)` is an initial
/// state, so the stem is empty.
///
/// With no constraint the loop is the shortest cycle through the entry: the
/// first edge in canonical order that returns to it, `setTo(0)` back to
/// value 0. A weak `inc` forces the loop to take `inc`, nearest first
/// (`0 -inc-> 1`), then back by the shortest path to the entry
/// (`1 -setTo(0)-> 0`).
#[trace("TC-521")]
#[test]
fn a_weak_constraint_forces_the_loop_through_the_operation() {
    let formula = "always eventually holds(c.value = 3)";
    let free = counter(document_with_ranged_parameter(), "", formula);
    let outcome = free.decide(is_three).expect("checked");
    let counterexample = violated(&outcome);
    assert_eq!(free.values(counterexample, "c"), [0, 0]);
    assert_eq!(
        lasso_of(counterexample),
        Lasso {
            entry: 0,
            terminal_stutter: false,
            terminal_stutter_delay: None,
        }
    );
    assert!(applies(&counterexample.prefix[0], "setTo"));
    assert!(counterexample.fairness.is_empty());

    for (fairness, each) in [
        ("fair weak M::Counter::inc;", false),
        ("fair weak whole M::Counter::inc;", false),
        ("fair weak each M::Counter::inc;", true),
    ] {
        let fixture = counter(document_with_ranged_parameter(), fairness, formula);
        let outcome = fixture.decide(is_three).expect("checked");
        let counterexample = violated(&outcome);
        assert_eq!(fixture.values(counterexample, "c"), [0, 1, 0], "{fairness}");
        assert_eq!(
            lasso_of(counterexample),
            Lasso {
                entry: 0,
                terminal_stutter: false,
                terminal_stutter_delay: None,
            }
        );
        assert!(applies(&counterexample.prefix[0], "inc"));
        assert!(applies(&counterexample.prefix[1], "setTo"));
        assert_eq!(counterexample.fairness.len(), 1);
        assert_weakly_fair(&fixture, counterexample, "inc", each);
        assert_eq!(counterexample.len(), 2);
    }
}

/// ADR-018 FA-1 and FA-6: over two counters a weak `whole` constraint on
/// `inc` is met by `inc` on either counter, a weak `each` constraint needs
/// both `inc(a)` and `inc(b)` in the loop (or disabled in it). The claim
/// over `c = a` (the first binding) is `always eventually holds(c.value =
/// 3)`: the lasso keeps `a` below 3.
#[trace("TC-521")]
#[test]
fn each_needs_every_transition_of_the_operation_and_whole_needs_one() {
    let formula = "always eventually holds(c.value = 3)";
    let starts = [("a", 0), ("b", 0)];
    for (fairness, each) in [
        ("fair weak whole M::Counter::inc;", false),
        ("fair weak each M::Counter::inc;", true),
    ] {
        let fixture = fixture(
            document_with_ranged_parameter(),
            CONTRACT,
            fairness,
            formula,
            &starts,
        );
        let outcome = fixture.decide(is_three).expect("checked");
        let counterexample = violated(&outcome);
        assert_eq!(counterexample.over_binding, Some(instance("a")));
        assert_weakly_fair(&fixture, counterexample, "inc", each);
        let a = fixture.values(counterexample, "a");
        let lasso = lasso_of(counterexample);
        assert!(
            a[lasso.entry..].iter().all(|value| *value != 3),
            "a never reaches 3 in the loop: {a:?}"
        );
        let loop_steps = &counterexample.prefix[lasso.entry..];
        let incs = loop_steps
            .iter()
            .filter(|step| applies(step, "inc"))
            .count();
        assert!(incs >= 1, "{fairness}: the loop takes inc");
    }
}

/// FR-125-AC-6's TP-4 half and FR-126-AC-9's TP-4 claim. `always eventually
/// holds(q(2 - c.value) = 0 and c.value = 1)`, with `q` the function
/// `6 mod x`, has a letter that is false at value 0, true at 1, undefined at
/// 2 and false at 3. It returns the undefined-evaluation counterexample, the
/// prefix `0 -inc-> 1 -inc-> 2` with no lasso. With the letter at 2 read as
/// false instead, the same claim is refuted by the terminal stutter lasso at
/// value 3, so the undefined position is what withholds that lasso. The same
/// holds for an undefined precondition guard (ADR-018 UE-6): the item is
/// refuted at the expanded state, naming the guarded transition.
#[trace("TC-537", "FR-126-AC-9")]
#[test]
fn an_undefined_letter_refutes_a_liveness_claim_before_any_lasso() {
    let helper = "function undefined_remainder using v(x: Integer): Integer pure { 6 mod x }\n";
    let formula = "always eventually holds(undefined_remainder(2 - c.value) = 0 and c.value = 1)";
    let fixture = fixture_in_mode(
        document(false),
        &format!("{CONTRACT}{helper}"),
        "",
        formula,
        &[("c", 0)],
        filament_semantic_value::checking::CheckMode::Kernel,
    );
    let outcome = fixture
        .decide(|v| (v != 2).then(|| 6 % (2 - v) == 0 && v == 1))
        .expect("checked");
    let counterexample = violated(&outcome);
    assert_eq!(fixture.values(counterexample, "c"), [0, 1, 2]);
    assert_eq!(counterexample.lasso, None);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.position, 2);
    assert!(matches!(evaluation.site.locus, UndefinedLocus::Atom(_)));
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(filament_exact::Undefined::DivisionByZero)
    );
    assert_eq!(counterexample.trace_position(), Some(2));

    let outcome = fixture.decide(|v| Some(v == 1)).expect("checked");
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(fixture.values(counterexample, "c"), [0, 1, 2, 3]);
    assert_eq!(
        counterexample.lasso,
        Some(Lasso {
            entry: 3,
            terminal_stutter: true,
            terminal_stutter_delay: None,
        })
    );

    let guarded =
        "pre CanInc using v on M::Counter::inc { undefined_remainder(2 - self.value) >= 0 }\n\
                   post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n";
    let fixture = fixture_in_mode(
        document(false),
        &format!("{helper}{guarded}"),
        "",
        "always eventually holds(c.value = 0)",
        &[("c", 0)],
        filament_semantic_value::checking::CheckMode::Kernel,
    );
    let outcome = fixture.decide(is_zero).expect("checked");
    let counterexample = violated(&outcome);
    assert_eq!(fixture.values(counterexample, "c"), [0, 1, 2]);
    assert_eq!(counterexample.lasso, None);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.position, 2);
    assert_eq!(
        evaluation.site.locus,
        UndefinedLocus::Contract("CanInc".to_owned())
    );
    assert!(
        format!("{:?}", evaluation.site.transition).contains("Counter/inc"),
        "the guard names inc"
    );
}

/// FR-126 "Explicit-state limits" for a TP-4 item over the Counter subject
/// and `always eventually holds(c.value = 0)` (the stutter lasso at value 3):
/// the lasso needs the depth-3 product state expanded, so a horizon of 3
/// settles `BoundReached{3}` and a horizon of 4 finds it, found by the
/// second phase over the retained graph although the frontier is not empty;
/// `max_states` 2 and `max_transitions` 3 stop the run naming the limit; an
/// already-cancelled handle stops it with no limit; a zero evaluation meter
/// stops it at `inc`'s precondition; `max_automaton_states` 1 stops it at
/// the negation's second state.
#[trace("TC-521")]
#[test]
fn limits_and_cancellation_stop_a_liveness_run_naming_what_stopped_it() {
    let fixture = counter(document(false), "", "always eventually holds(c.value = 0)");
    let defaults = ModelCheckLimits::default();
    let with = |max_depth, limits, meter, cancel: &Cancel| {
        fixture
            .run(is_zero, max_depth, limits, meter, cancel)
            .expect("checked")
    };
    let live = Cancel::new();
    assert!(matches!(
        with(Some(3), defaults, UNLIMITED, &live),
        ModelCheckOutcome::BoundReached { depth: 3, .. }
    ));
    assert_eq!(
        lasso_of(violated(&with(Some(4), defaults, UNLIMITED, &live))),
        Lasso {
            entry: 3,
            terminal_stutter: true,
            terminal_stutter_delay: None,
        }
    );
    let stopped = |outcome: ModelCheckOutcome| match outcome {
        ModelCheckOutcome::Stopped { cause, limit, .. } => (cause, limit),
        other => panic!("expected a stopped run, got {other:?}"),
    };
    let reached = |limit, value| {
        (
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit { limit, value }),
        )
    };
    let mut states = defaults;
    states.limits.max_states = 2;
    assert_eq!(
        stopped(with(None, states, UNLIMITED, &live)),
        reached(ModelCheckLimit::MaxStates, 2)
    );
    let mut transitions = defaults;
    transitions.limits.max_transitions = 3;
    assert_eq!(
        stopped(with(None, transitions, UNLIMITED, &live)),
        reached(ModelCheckLimit::MaxTransitions, 3)
    );
    let mut automaton = defaults;
    automaton.max_automaton_states = 1;
    assert_eq!(
        stopped(with(None, automaton, UNLIMITED, &live)),
        reached(ModelCheckLimit::MaxAutomatonStates, 1)
    );
    assert_eq!(
        stopped(with(None, defaults, ZERO, &live)),
        reached(
            ModelCheckLimit::EvaluationMeter {
                counter: filament_exact::LimitKind::ValueOccurrences
            },
            0
        )
    );
    let cancelled = Cancel::new();
    cancelled.cancel(CancelCause::Requested);
    assert_eq!(
        stopped(with(None, defaults, UNLIMITED, &cancelled)),
        (
            IncompleteCause::Cancelled {
                source: CancelCause::Requested
            },
            None
        )
    );
}

/// FR-126 "Determinism" for a TP-4 item: two runs of one request give equal
/// outcomes and byte-equal counterexamples.
#[trace("TC-521")]
#[test]
fn two_liveness_runs_give_equal_outcomes() {
    let fixture = counter(
        document_with_ranged_parameter(),
        "fair weak each M::Counter::inc;",
        "always eventually holds(c.value = 3)",
    );
    let once = || fixture.decide(is_three).expect("checked");
    assert_eq!(once(), once());
}

/// FR-126 "Bindings and ranking (TP-4)" and "Model-only pass (TP-4)": the
/// run's `max_states` is shared by every binding, and each state a model-only
/// pass creates is charged to it and counted in no product total. Over two
/// counters, `eventually holds(c.value = 1)` is read as always true, so
/// neither `a` nor `b` is refuted and both are decided. The negation then has
/// no move at the first position: no binding has a product state, and each
/// binding's pass reads the whole model from the initial state. The frame
/// lets `inc` on one counter give the other any value, so all 4 x 4 states
/// are reached. Two passes of 16 states need `max_states` 32; 31 runs out in
/// `b`'s pass, on what `a`'s pass left.
#[trace("TC-1085")]
#[test]
fn the_model_only_pass_spends_the_budget_the_next_instance_gets() {
    let fixture = fixture(
        document(false),
        CONTRACT,
        "",
        "eventually holds(c.value = 1)",
        &[("a", 0), ("b", 0)],
    );
    let run = |limits| {
        fixture
            .run(|_| Some(true), None, limits, UNLIMITED, &Cancel::new())
            .expect("checked")
    };
    let budget = |max_states| {
        let mut limits = ModelCheckLimits::default();
        limits.limits.max_states = max_states;
        run(limits)
    };
    for outcome in [budget(32), run(ModelCheckLimits::default())] {
        let ModelCheckOutcome::Holds { stats, .. } = outcome else {
            panic!("expected a proof: {outcome:?}");
        };
        assert_eq!((stats.product_states, stats.product_edges), (0, 0));
    }
    let ModelCheckOutcome::Stopped {
        cause,
        limit,
        stats,
        ..
    } = budget(31)
    else {
        panic!("31 states do not cover both passes");
    };
    assert_eq!(cause, IncompleteCause::ResourceExhausted);
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxStates,
            value: 31
        })
    );
    assert_eq!((stats.product_states, stats.product_edges), (0, 0));
}

fn is_at_most_three(value: i64) -> Option<bool> {
    Some(value <= 3)
}

/// The values of `c` along the one behaviour of a single-counter fixture's
/// model, read from its successor relation: each state's only successor
/// until a state has none (the terminal state).
fn behaviour_values(fixture: &Fixture) -> Vec<i64> {
    let system = fixture.system();
    let mut initial = system.initial();
    assert_eq!(initial.len(), 1, "one initial state");
    let mut current = initial.remove(0);
    let mut values = vec![value(&current, "c")];
    loop {
        let mut successors = system.successors(&current).expect("expands").successors;
        match successors.len() {
            0 => return values,
            1 => current = successors.remove(0).1,
            more => panic!("{more} successors: more than one behaviour"),
        }
        values.push(value(&current, "c"));
        assert!(values.len() < 16, "the behaviour ends");
    }
}

/// The TemporalTrace evaluator's value of `fixture`'s clause on `trace`,
/// whose position `i` reads `values[i]` through `atom`. As in the repo's
/// other evaluator tests, the letter is a test closure: the clause's atom
/// text is not itself evaluated here (that is the one clause evaluator's,
/// FR-107), so the formula structure is what these readings exercise.
fn evaluated(
    fixture: &Fixture,
    trace: TemporalTrace,
    values: &[i64],
    atom: fn(i64) -> Option<bool>,
) -> TemporalOutcome {
    struct Positions<'a> {
        values: &'a [i64],
        atom: fn(i64) -> Option<bool>,
    }
    impl TemporalLetters for Positions<'_> {
        fn atom(
            &mut self,
            _atom: FormulaNodeId,
            position: TemporalPosition,
            _meter: &mut Meter,
        ) -> Result<AtomReading, LetterStop> {
            let index = usize::try_from(position.index()).expect("a small position");
            Ok(match (self.atom)(self.values[index]) {
                Some(true) => AtomReading::True,
                _ => AtomReading::False,
            })
        }

        fn activation(
            &mut self,
            _position: TemporalPosition,
            _meter: &mut Meter,
        ) -> Result<AtomReading, LetterStop> {
            panic!("an `on origin` clause has no activation condition")
        }
    }
    let mut letters = Positions { values, atom };
    evaluate(
        &fixture.clauses[0],
        trace,
        &mut letters,
        &mut Meter::new(UNLIMITED),
    )
    .expect("the trace is readable")
}

/// FR-125-AC-2, over the `Counter` subject whose one behaviour is `0, 1, 2,
/// 3` and then the terminal stutter at `3`: `always[0,5] holds(c.value <= 3)`
/// from position 0 is false under event-position false-extension (positions 4
/// and 5 are past closure) and true under infinite-trace (the stutter repeats
/// `value = 3`); `eventually holds(c.value = 3)` is true under infinite-trace
/// and `always eventually holds(c.value = 0)` is false. The model checker
/// agrees where it decides (the first two), and the evaluator reads the
/// behaviour the model's own successor relation gives.
#[trace("TC-520", "FR-125-AC-2")]
#[test]
fn a_bounded_always_reads_past_closure_as_false_only_under_event_position_false_extension() {
    let window = "always[0,5] holds(c.value <= 3)";
    let closed = crate::lasso_support::bounded_fixture(window);
    let infinite = counter(document(false), "", window);
    let reaches = counter(document(false), "", "eventually holds(c.value = 3)");
    let recurs = counter(document(false), "", "always eventually holds(c.value = 0)");

    let values = behaviour_values(&infinite);
    assert_eq!(values, [0, 1, 2, 3]);
    assert_eq!(behaviour_values(&closed), values);
    let prefix = u64::try_from(values.len() - 1).expect("small");
    let stuttered = TemporalTrace::Lasso {
        prefix,
        loop_len: 1,
    };
    let closed_trace = TemporalTrace::Finite { len: prefix + 1 };

    assert!(matches!(
        evaluated(&closed, closed_trace, &values, is_at_most_three),
        TemporalOutcome::Violated { .. }
    ));
    assert_eq!(
        evaluated(&infinite, stuttered, &values, is_at_most_three),
        TemporalOutcome::Holds
    );
    assert_eq!(
        evaluated(&reaches, stuttered, &values, is_three),
        TemporalOutcome::Holds
    );
    assert!(matches!(
        evaluated(&recurs, stuttered, &values, is_zero),
        TemporalOutcome::Violated { .. }
    ));

    let outcome = closed.decide(is_at_most_three).expect("checked");
    assert_eq!(violated(&outcome).kind, CounterexampleKind::Formula);
    assert!(matches!(
        infinite.decide(is_at_most_three),
        Ok(ModelCheckOutcome::Holds { .. })
    ));
    let outcome = recurs.decide(is_zero).expect("checked");
    assert!(violated(&outcome)
        .lasso
        .as_ref()
        .is_some_and(|lasso| lasso.terminal_stutter));
}

/// A property component that reads nothing, so a [`Product`] over it is the
/// model's own behaviours.
struct Silent;

impl<M: TransitionSystem> PropertyComponent<M> for Silent {
    type State = usize;
    type Key = usize;
    type Finding = ();

    fn initial(&self, _model: &M::State) -> impl Iterator<Item = usize> + use<M> {
        std::iter::once(0)
    }

    fn key(&self, state: &usize) -> usize {
        *state
    }

    fn advance(
        &self,
        state: &usize,
        _source: &M::State,
        _transition: &ProductTransition<M::TransitionId>,
        _target: &M::State,
    ) -> Result<Vec<usize>, ExpansionStop> {
        Ok(vec![*state])
    }

    fn findings(&self, _state: &usize, _model: &M::State) -> Result<Vec<()>, ExpansionStop> {
        Ok(Vec::new())
    }

    fn close(&self, _state: &usize, _model: &M::State) -> Result<Vec<()>, ExpansionStop> {
        Ok(Vec::new())
    }
}

/// FR-125-AC-1, over the `Counter` subject (universe `{c}`, initial value 0,
/// no `terminal` declaration): the one behaviour is positions `0, 1, 2, 3`
/// anchored `initialization`, `inc`, `inc`, `inc`; under infinite-trace it
/// continues with the terminal stutter step repeating position 3, whose
/// transition identity differs from every `inc` identity.
#[trace("TC-520", "FR-125-AC-1")]
#[test]
fn the_counter_behaviour_is_positions_anchored_by_initialization_then_each_inc_and_a_stutter() {
    let fixture = counter(document(false), "", "always holds(c.value <= 3)");
    let model = fixture.system();
    let property = Silent;
    let product = Product::infinite(&model, &property);

    let mut initial = product.initial();
    assert_eq!(initial.len(), 1, "one initial state");
    let mut state = initial.remove(0);
    let mut values = vec![value(&state.model, "c")];
    let mut anchors = vec![state
        .model
        .required_anchor()
        .expect("an admitted initial anchor")
        .clone()];
    let mut incs = Vec::new();
    for _ in 0..3 {
        let mut steps = product.successors(&state).expect("expands").successors;
        assert_eq!(steps.len(), 1, "one behaviour: {steps:?}");
        let (transition, next) = steps.remove(0);
        let ProductTransition::Model(identity) = transition else {
            panic!("a model step before the terminal state, got {transition:?}");
        };
        assert!(identity.operation().contains("Counter/inc"));
        incs.push(identity);
        state = next;
        values.push(value(&state.model, "c"));
        anchors.push(
            state
                .model
                .required_anchor()
                .expect("an actual handler anchor")
                .clone(),
        );
    }
    assert_eq!(values, [0, 1, 2, 3]);
    assert_eq!(anchors[0].kind, AnchorKind::Initialization);
    assert!(anchors[1..].iter().all(|anchor| {
        anchor.kind == AnchorKind::Handler && anchor.name.contains("ix://test/counter/Counter/inc")
    }));
    assert_eq!(anchors[1..].iter().collect::<Vec<_>>(), [&anchors[1]; 3]);

    assert!(model
        .successors(&state.model)
        .expect("expands")
        .successors
        .is_empty());
    let mut steps = product.successors(&state).expect("expands").successors;
    assert_eq!(steps.len(), 1, "the stutter is the only step: {steps:?}");
    let (stutter, again) = steps.remove(0);
    assert_eq!(stutter, ProductTransition::TerminalStutter);
    assert_eq!(value(&again.model, "c"), 3, "position 3 repeats");
    assert_eq!(
        model
            .state_key(&again.model, fsl_foundation::IdentityLimits::default())
            .expect("encodes")
            .1,
        model
            .state_key(&state.model, fsl_foundation::IdentityLimits::default())
            .expect("encodes")
            .1,
        "the stutter returns to the same state"
    );
    let written = |step: &ProductTransition<_>| {
        quire_canonical::to_vec(step, quire_canonical::Limits::new(1 << 16)).expect("encodes")
    };
    let stutter_identity = written(&stutter);
    for inc in incs {
        assert_ne!(stutter_identity, written(&ProductTransition::Model(inc)));
    }
}
