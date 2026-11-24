// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-128-AC-10 to AC-14: the replay of FSpec FR-364's `closure-stutter` lasso
//! over the `Counter` subject. Every lasso is written by hand, since EN-1 may
//! find another violating lasso, and the item selects `stutter_closure`
//! through `ModelTraceSubject` or, on the reference-literal source route,
//! through the entry's own argument.

use filament_exact::Cancel;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::model_subject::{ModelInstance, ModelLetters};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::{StateKey, TransitionSystem};
use fsl_foundation::diagnostic::Code;
use fsl_replay::{
    replay_checked_source_model_trace, replay_model_trace_with_temporal_selections,
    settle_model_check_over_stutter_closure, ClosureRule, ClosureStutter, CounterexampleKind,
    DisagreementCause, InconclusiveCause, Lasso, ModelCheckLimits, ModelCheckOutcome, ModelStep,
    ModelTraceRefusal, ModelTraceReplayResult, ModelTraceSettlement, ModelTraceSubject,
    ModelTraceValue, ReplayLimits, ReplayRefusal, RunStats, StepSite, TemporalCounterexample,
    TerminalValue,
};
use ix_trace_rs::trace;

use crate::counter::{
    document, document_with_ranged_parameter, subject, value, Letters, UNLIMITED,
};
use crate::lasso_replay::{clause_identity, found, refusal, subject_of};
use crate::liveness::{counter, is_zero, Fixture};
use crate::model_settle::{envelope_at, request};

const FORMULA: &str = "always eventually holds(c.value = 3)";
const ZERO: &str = "always eventually holds(c.value = 0)";

fn is_three(value: i64) -> Option<bool> {
    Some(value == 3)
}

fn system(fixture: &Fixture) -> ModelSystem<'_> {
    ModelSystem::new(
        subject(
            &fixture.package,
            &fixture.documents,
            fixture.initial.clone(),
            &fixture.keys,
            UNLIMITED,
        )
        .simulation,
    )
    .expect("the subject admits")
}

fn key_of(system: &ModelSystem<'_>, state: &ModelSimState) -> StateKey {
    StateKey::from_canonical_bytes(system.state_key(state).expect("the state encodes").0)
}

/// The model steps that take `Counter` from its initial state through each
/// `(operation, value reached)` move, and the key of the state they end at.
fn walk(fixture: &Fixture, moves: &[(&str, i64)]) -> (Vec<ModelStep>, StateKey) {
    let system = system(fixture);
    let mut current = system.initial()[0].clone();
    let mut steps = Vec::new();
    for (operation, reached) in moves {
        let text = system
            .operations()
            .into_iter()
            .find(|(_, name, _)| name == *operation)
            .map(|(_, _, text)| text)
            .expect("the operation is declared");
        let (transition, next) = system
            .successors(&current)
            .expect("the state expands")
            .successors
            .into_iter()
            .find(|(transition, state)| {
                transition.operation() == text && value(state, "c") == *reached
            })
            .expect("a move of the operation reaches the value");
        steps.push(ModelStep {
            transition,
            post_state: key_of(&system, &next),
            delay: None,
        });
        current = next;
    }
    let key = key_of(&system, &current);
    (steps, key)
}

/// The counterexample whose sole loop step is a closure stutter at the
/// state `stem` reaches, recorded under `key`.
fn closure_lasso(fixture: &Fixture, stem: Vec<ModelStep>, key: StateKey) -> TemporalCounterexample {
    TemporalCounterexample {
        initial_state: 0,
        over_binding: Some(ModelInstance {
            population: crate::counter::key("counters"),
            key: "c".to_owned(),
        }),
        fairness: fixture.clauses[0].fairness().to_vec(),
        kind: CounterexampleKind::Formula,
        lasso: Some(Lasso {
            entry: stem.len(),
            terminal_stutter: false,
            terminal_stutter_delay: None,
        }),
        prefix: stem,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: vec![ClosureStutter {
            site: StepSite::Loop(0),
            post_state: key,
            delay: None,
        }],
    }
}

/// The replay entry of a package-route-free source replay, with the item
/// selecting `stutter_closure` or not.
fn replay_entry(
    fixture: &Fixture,
    payload: TemporalCounterexample,
    letters: &dyn ModelLetters,
    selected: bool,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let (node, occurrence) = clause_identity(fixture);
    replay_model_trace_with_temporal_selections(
        request(
            &fixture.package,
            &fixture.temporal_clock_selections,
            &fixture.documents.document,
            fixture.documents.snapshot_bytes(),
            &fixture.unit,
        ),
        &ModelTraceSubject {
            stutter_closure: selected,
            ..subject_of(fixture)
        },
        &envelope_at(payload, node, occurrence),
        letters,
        &fixture.temporal_clock_selections,
        ReplayLimits::default(),
    )
}

/// The reference-literal source route, which bypasses `ModelTraceSubject`.
fn replay_reference_route(
    fixture: &Fixture,
    payload: TemporalCounterexample,
    letters: &dyn ModelLetters,
    selected: bool,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let (node_key, occurrence) = clause_identity(fixture);
    let system = system(fixture);
    replay_checked_source_model_trace(
        &fixture.package,
        &system,
        &fixture.package.graph().temporal_clauses()[0],
        &envelope_at(payload, node_key, occurrence),
        letters,
        UNLIMITED,
        selected,
    )
}

/// Both entries that replay a model trace, each over the same payload.
fn both_routes(
    fixture: &Fixture,
    payload: &TemporalCounterexample,
    letters: &dyn ModelLetters,
    selected: bool,
) -> [Result<ModelTraceReplayResult, ReplayRefusal>; 2] {
    [
        replay_entry(fixture, payload.clone(), letters, selected),
        replay_reference_route(fixture, payload.clone(), letters, selected),
    ]
}

fn reproduced_at(result: Result<ModelTraceReplayResult, ReplayRefusal>) -> Option<u64> {
    match result.expect("replays").settlement {
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position,
        } => trace_position,
        other => panic!("expected a reproduction, got {other:?}"),
    }
}

/// The pointer and rule a closure-stutter refusal names, with its catalog code.
fn closure_refusal(result: Result<ModelTraceReplayResult, ReplayRefusal>) -> (String, ClosureRule) {
    let (refused, code, cause) = refusal(result);
    assert_eq!((code, cause), (Code::InvalidRuntimeInput, "invalid-value"));
    match refused {
        ModelTraceRefusal::ClosureStutter { pointer, rule } => (pointer, rule),
        other => panic!("expected a closure-stutter refusal, got {other:?}"),
    }
}

/// FR-128-AC-10: the closure stutter at the initial state (empty stem) and at
/// the state one `inc` reaches (non-empty stem) reproduce at position 0 when
/// the item selects `stutter_closure`, and refuse at `/loop/0/transition/type`
/// when it does not.
#[trace("TC-523", "FR-128-AC-10")]
#[test]
fn a_closure_lasso_reproduces_over_an_empty_and_a_non_empty_stem() {
    let empty = counter(document(false), "", FORMULA);
    let (stem, key) = walk(&empty, &[]);
    assert!(stem.is_empty());
    let payload = closure_lasso(&empty, stem, key);
    for result in both_routes(
        &empty,
        &payload,
        &crate::counter::Letters { atom: is_three },
        true,
    ) {
        assert_eq!(reproduced_at(result), Some(0));
    }

    let stemmed = counter(document(false), "", ZERO);
    let (stem, key) = walk(&stemmed, &[("inc", 1)]);
    let payload = closure_lasso(&stemmed, stem, key);
    for result in both_routes(&stemmed, &payload, &Letters { atom: is_zero }, true) {
        assert_eq!(reproduced_at(result), Some(0));
    }

    for (fixture, payload, atom) in [
        (&stemmed, payload, is_zero as fn(i64) -> Option<bool>),
        (
            &empty,
            closure_lasso(&empty, Vec::new(), walk(&empty, &[]).1),
            is_three,
        ),
    ] {
        for result in both_routes(fixture, &payload, &Letters { atom }, false) {
            assert_eq!(
                closure_refusal(result),
                ("/loop/0/transition/type".to_owned(), ClosureRule::Alphabet)
            );
        }
    }
}

/// FR-128-AC-11 and FR-330 "Admit": the native model entry reads the closed
/// alphabet, selecting no `stutter_closure`, so it refuses a closure lasso at
/// its `/loop/0/transition/type` as the alphabet refusal, never as an open
/// loop, while the selected model entry reproduces the same payload.
#[trace("TC-523", "FR-128-AC-11")]
#[test]
fn the_native_admit_entry_refuses_a_closure_lasso_at_its_alphabet() {
    let fixture = counter(document(false), "", FORMULA);
    let (stem, key) = walk(&fixture, &[]);
    let payload = closure_lasso(&fixture, stem, key);
    assert_eq!(
        reproduced_at(replay_entry(
            &fixture,
            payload.clone(),
            &crate::counter::Letters { atom: is_three },
            true,
        )),
        Some(0)
    );
    let system = system(&fixture);
    let letters = system
        .clause_letters(&fixture.package.graph().temporal_clauses()[0])
        .expect("the clause's letters");
    let Err(ReplayRefusal::ModelTrace(refusal)) = fsl_replay::evaluate_model_trace(
        &system,
        &payload,
        &fixture.clauses[0],
        &letters,
        UNLIMITED,
    ) else {
        panic!("the native entry refuses the closure lasso");
    };
    assert_eq!(
        *refusal,
        ModelTraceRefusal::ClosureStutter {
            pointer: "/loop/0/transition/type".to_owned(),
            rule: ClosureRule::Alphabet,
        }
    );
}

/// FR-128-AC-11: each refusal names FSpec FR-364's pointer, in its order, and
/// settles no result; the reference-literal source route refuses each alike.
#[trace("TC-523", "FR-128-AC-11")]
#[test]
fn a_misplaced_closure_step_refuses_at_its_pointer_in_the_normative_order() {
    let fixture = counter(document(false), "", ZERO);
    let letters = Letters { atom: is_zero };
    let (stem, key) = walk(&fixture, &[("inc", 1)]);
    let sole = closure_lasso(&fixture, stem.clone(), key.clone());
    let site = |site, delay| ClosureStutter {
        site,
        post_state: key.clone(),
        delay,
    };
    let zero = Some(
        filament_exact::Rational::new(
            filament_exact::Integer::from(0_i64),
            filament_exact::Integer::from(1_i64),
        )
        .expect("a nonzero denominator"),
    );

    let mut cases: Vec<(&str, TemporalCounterexample, &str, ClosureRule)> = Vec::new();

    let mut in_prefix = sole.clone();
    in_prefix
        .closure_stutters
        .insert(0, site(StepSite::Prefix(0), None));
    cases.push((
        "a closure step in the prefix",
        in_prefix,
        "/prefix/0/transition/type",
        ClosureRule::Placement,
    ));

    // The loop is a closure step and a model step.
    let mut two_step = sole.clone();
    two_step.lasso = Some(Lasso {
        entry: 0,
        terminal_stutter: false,
        terminal_stutter_delay: None,
    });
    cases.push((
        "a loop of two steps, the closure first",
        two_step,
        "/loop/0/transition/type",
        ClosureRule::Placement,
    ));

    let mut undefined = sole.clone();
    undefined.kind = CounterexampleKind::UndefinedEvaluation(fsl_replay::UndefinedEvaluation {
        site: fsl_replay::UndefinedSite {
            position: 1,
            state_key: key.clone(),
            locus: fsl_replay::UndefinedLocus::Activation,
            transition: None,
            time: None,
        },
        cause: fsl_eval::temporal::UndefinedCause::Kernel(
            filament_exact::Undefined::DivisionByZero,
        ),
    });
    cases.push((
        "an undefined-evaluation counterexample",
        undefined,
        "/loop/0/transition/type",
        ClosureRule::Kind,
    ));

    let mut delayed = sole.clone();
    delayed.closure_stutters[0].delay = zero.clone();
    cases.push((
        "a delay member of zero",
        delayed,
        "/loop/0/delay",
        ClosureRule::Delay,
    ));

    // The order: the prefix step is refused at its type, not its delay, and
    // before a later loop step that fails too.
    let mut prefix_with_delay = sole.clone();
    prefix_with_delay
        .closure_stutters
        .insert(0, site(StepSite::Prefix(0), zero.clone()));
    cases.push((
        "a prefix step with a delay",
        prefix_with_delay,
        "/prefix/0/transition/type",
        ClosureRule::Placement,
    ));
    let mut prefix_then_loop = sole.clone();
    prefix_then_loop.closure_stutters[0].delay = zero.clone();
    prefix_then_loop
        .closure_stutters
        .insert(0, site(StepSite::Prefix(0), None));
    cases.push((
        "a prefix step before a failing loop step",
        prefix_then_loop,
        "/prefix/0/transition/type",
        ClosureRule::Placement,
    ));

    // After re-execution: the loop at the terminal value 3, and a key that is
    // not the entry's.
    let (to_three, three) = walk(&fixture, &[("inc", 1), ("inc", 2), ("inc", 3)]);
    cases.push((
        "a terminal loop state",
        closure_lasso(&fixture, to_three, three),
        "/loop/0/transition",
        ClosureRule::Terminal,
    ));
    let (_, two) = walk(&fixture, &[("inc", 1), ("inc", 2)]);
    cases.push((
        "another state's key",
        closure_lasso(&fixture, stem, two),
        "/loop/0/post_state",
        ClosureRule::PostState,
    ));

    for (name, payload, pointer, rule) in cases {
        for result in both_routes(&fixture, &payload, &letters, true) {
            assert_eq!(
                closure_refusal(result),
                (pointer.to_owned(), rule),
                "{name}"
            );
        }
    }
}

/// FR-128-AC-12: the closure step takes no transition and satisfies no
/// constraint, so a weak or strong constraint, whole or `each`, that is
/// enabled at the loop's state makes the lasso unfair, and one that is
/// disabled there leaves it fair. The terminal stutter is unchanged.
#[trace("TC-523", "FR-128-AC-12")]
#[test]
fn a_closure_lasso_is_fair_only_where_the_constraint_is_disabled() {
    for fairness in [
        "fair weak M::Counter::inc;",
        "fair strong M::Counter::inc;",
        "fair weak each M::Counter::inc;",
        "fair strong each M::Counter::inc;",
    ] {
        let fixture = counter(document_with_ranged_parameter(), fairness, ZERO);
        let letters = Letters { atom: is_zero };

        // At value 3 `inc` is disabled, while `setTo` keeps the state
        // non-terminal.
        let (stem, key) = walk(&fixture, &[("inc", 1), ("inc", 2), ("inc", 3)]);
        let disabled = closure_lasso(&fixture, stem, key);
        for result in both_routes(&fixture, &disabled, &letters, true) {
            assert_eq!(reproduced_at(result), Some(0), "{fairness}");
        }

        let (stem, key) = walk(&fixture, &[("inc", 1)]);
        let enabled = closure_lasso(&fixture, stem, key);
        let constraint = fixture.clauses[0].fairness()[0].clone();
        for result in both_routes(&fixture, &enabled, &letters, true) {
            let Err(ReplayRefusal::UnfairLasso { constraint: named }) = result else {
                panic!("{fairness}: expected an unfair lasso, got {result:?}");
            };
            assert_eq!(named, constraint, "{fairness}");
        }
    }

    // The terminal stutter at value 3 replays as before, with or without the
    // flag.
    let terminal = counter(document(false), "", ZERO);
    let payload = found(&terminal, is_zero);
    assert!(payload
        .lasso
        .as_ref()
        .is_some_and(|lasso| lasso.terminal_stutter));
    assert!(payload.closure_stutters.is_empty());
    for selected in [false, true] {
        for result in both_routes(&terminal, &payload, &Letters { atom: is_zero }, selected) {
            assert_eq!(reproduced_at(result), Some(0));
        }
    }
}

/// FR-128-AC-13: a closure lasso whose atoms read true at every position
/// settles `inconclusive`, `ReplayParity` by `Verdicts`, and one whose atom
/// reads undefined settles `inconclusive` with `NoValue`.
#[trace("TC-523", "FR-128-AC-13")]
#[test]
fn a_closure_lasso_the_evaluator_does_not_refute_disagrees() {
    let fixture = counter(document(false), "", FORMULA);
    let (stem, key) = walk(&fixture, &[]);
    let payload = closure_lasso(&fixture, stem, key);

    for result in both_routes(
        &fixture,
        &payload,
        &Letters {
            atom: |_| Some(true),
        },
        true,
    ) {
        let ModelTraceSettlement::Inconclusive(cause) = result.expect("replays").settlement else {
            panic!("a true claim disagrees with the refutation");
        };
        assert!(
            matches!(cause, DisagreementCause::Verdicts { .. }),
            "{cause:?}"
        );
    }
    for result in both_routes(&fixture, &payload, &Letters { atom: |_| None }, true) {
        let ModelTraceSettlement::Inconclusive(cause) = result.expect("replays").settlement else {
            panic!("an undefined atom completes no value");
        };
        assert!(
            matches!(cause, DisagreementCause::NoValue { .. }),
            "{cause:?}"
        );
    }

    // The item that offered the lasso settles `inconclusive` with the item
    // cause `ReplayParity`, carrying that disagreement.
    let undefined = Letters { atom: |_| None };
    let settled = settle_model_check_over_stutter_closure(
        &ModelCheckOutcome::Violated {
            counterexample: Box::new(payload.clone()),
            stats: RunStats::default(),
        },
        &ModelCheckLimits::default(),
        &fixture.certificate_request(&undefined, UNLIMITED),
        &Cancel::new(),
        |found| replay_entry(&fixture, found.clone(), &undefined, true),
    );
    assert!(
        matches!(
            &settled.value,
            TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
                DisagreementCause::NoValue { .. }
            ))
        ),
        "{:?}",
        settled.value
    );
    assert!(settled.counterexample.is_none());
}

/// FR-128-AC-13, FSpec FR-364 "Named limit": a constructed engine lasso with a
/// closure stutter among its model steps is written without the stutter, and
/// the written lasso, on which the stutter-sensitive clause is true, settles
/// `inconclusive`, `ReplayParity` by `Verdicts`, never `reproduced`. The
/// mixed form itself is no closure lasso and refuses placement.
#[trace("TC-523", "FR-128-AC-13", "FR-364-AC-29")]
#[test]
fn a_mixed_loop_is_written_without_its_closure_stutter_and_never_refutes() {
    let clause = "(always (holds(c.value = 1) implies not once[1,1] holds(c.value = 1))) \
                  and (always eventually holds(c.value = 2))";
    let fixture = counter(document_with_ranged_parameter(), "", clause);
    let system = system(&fixture);
    let letters = system
        .clause_letters(&fixture.package.graph().temporal_clauses()[0])
        .expect("the production letters, whose atoms differ, build");

    // Product walk: inc (to 1); then the loop closure at 1, inc (to 2),
    // setTo(1) (back to 1).
    let (steps, key) = walk(&fixture, &[("inc", 1), ("inc", 2), ("setTo", 1)]);
    let mut engine = closure_lasso(&fixture, steps, key);
    engine.lasso = Some(Lasso {
        entry: 1,
        terminal_stutter: false,
        terminal_stutter_delay: None,
    });
    assert!(engine.closure_loop().is_none());

    let (pointer, rule) = closure_refusal(replay_entry(&fixture, engine.clone(), &letters, true));
    assert_eq!(
        (pointer.as_str(), rule),
        ("/loop/0/transition/type", ClosureRule::Placement)
    );

    let mut unbound = engine.clone();
    unbound.over_binding = None;
    let wire = fsl_replay::temporal_wire(&unbound, None, None).expect("the lasso is spelled");
    assert_eq!(wire["prefix"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        wire["loop"].as_array().map(Vec::len),
        Some(2),
        "the closure stutter is dropped: inc and setTo remain"
    );
    let mut written = fsl_replay::read_temporal_counterexample::<ModelTransition>(&wire)
        .expect("the written lasso reads back");
    assert!(written.closure_stutters.is_empty());
    written.over_binding = engine.over_binding.clone();
    for result in both_routes(&fixture, &written, &letters, true) {
        let ModelTraceSettlement::Inconclusive(cause) = result.expect("replays").settlement else {
            panic!("the written lasso is true, so it never reproduces");
        };
        assert!(
            matches!(cause, DisagreementCause::Verdicts { .. }),
            "{cause:?}"
        );
    }
}
