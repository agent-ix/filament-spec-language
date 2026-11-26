// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 (ADR-018 EN-1, TP-2 and TP-3): EN-1 decides bounded MLTL and
//! infinite-trace safety clauses on the product of a model subject with the
//! clause's lazy bad-prefix monitor, over the subjects of `crate::model_world`.
//!
//! Every expected path is computed from the subject's own arithmetic and
//! read back by re-executing the counterexample's steps. A clause's `holds`
//! atoms are read through the test's own letters, each atom by a predicate
//! that computes the condition its source text spells. A guarded `on each`
//! clause's `when` condition is read the same way: the source writes the
//! placeholder `when (true)` and the guard's reading is the test's own
//! (FR-126-AC-16), given to the engine directly.
//!
//! Not covered here: FR-126-AC-1, AC-3 to AC-5, AC-7 and AC-8 rest on TP-4
//! claims (QSL-530); AC-9's TP-4 row has no fixture in this module. AC-6's
//! refused-contract row has none in this crate: every kernel refusal a
//! contract can raise is proved away by S3, a family refusal needs a
//! dispatch-family fixture whose `family_steps` or `ancestor_steps` ceiling refuses a dispatched
//! call, and the dispatch fixtures live in `fsl-eval`'s tests (Remaining
//! work: QSL-898). Replay's reading of a guarded condition
//! (`ReplayedLetters::activation`) is exercised once, over FR-315-AC-58's
//! hand-built package and its production letters.

use filament_exact::{Cancel, Charge, ChargePoint, Incomplete, LimitKind, Meter};
use fsl_analyze::model_check::{check_model, Atoms, ModelCheckRefusal, ModelCheckRequest, Por};
use fsl_eval::automaton::translate;
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimState, ModelSystem};
use fsl_eval::simulation::TransitionSystem;
use fsl_eval::temporal::UndefinedCause;
use fsl_eval::temporal::{AtomReading, LetterStop, TemporalPosition};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_replay::{
    replay_model_trace_with_temporal_selections, settle_model_check, CertificateRequest,
    CounterexampleKind, DisagreementCause, IncompleteCause, ModelCheckItem, ModelCheckLimit,
    ModelCheckLimits, ModelCheckOutcome, ModelStep, ModelTraceSettlement, ModelTraceSubject,
    ModelTraceValue, ReachedLimit, ReplayLimits, SmtEncodingLimits, TemporalCounterexample,
    TerminalValue, UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
use fsl_semantics::check::FormulaNodeId;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::counter::{CONTRACT, UNLIMITED};
use crate::model_settle_support::{envelope_at, request};
use crate::model_world::{self, Atom, Clause, Letters, World, BOUNDED, INFINITE};

const ORIGIN: &str = "on origin";

/// FR-126-AC-4's `Restless` unit: `fail` has no precondition.
const RESTLESS: &str = "post Failed using v on M::Health::fail { \
                        not self.healthy and self.failures = 1 }\n\
                        pre CanRecover using v on M::Health::recover { not self.healthy }\n\
                        post Recovered using v on M::Health::recover { self.healthy }\n";

fn each(world: &str) -> String {
    format!("on each (q: M::{world})")
}

/// The run of the world's clause `index` over the universe `keys`.
fn run(
    world: &World,
    index: usize,
    keys: &[&str],
    atoms: Vec<Atom>,
    limits: ModelCheckLimits,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let letters = Letters::new(&world.clauses[index], atoms);
    run_letters(world, index, keys, &letters, limits)
}

/// The run of the world's clause `index` over the universe `keys`, read
/// through `letters`.
fn run_letters(
    world: &World,
    index: usize,
    keys: &[&str],
    letters: &Letters,
    limits: ModelCheckLimits,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let clause = &world.clauses[index];
    let temporal_clock_selections = crate::fill::tcb_01_event_position_selection(&world.package);
    check_model(
        ModelCheckRequest {
            workers: 1,
            temporal_clock_selections: &temporal_clock_selections,
            subject: world.subject(keys, UNLIMITED),
            item: ModelCheckItem::Clause { clause, letters },
            max_depth: None,
            limits,
            smt_encoding: Default::default(),
            smt_run: Default::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
}

fn checked(world: &World, index: usize, keys: &[&str], atoms: Vec<Atom>) -> ModelCheckOutcome {
    run(world, index, keys, atoms, ModelCheckLimits::default()).expect("checked")
}

fn violated(outcome: &ModelCheckOutcome) -> &TemporalCounterexample {
    match outcome {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

fn holds(outcome: &ModelCheckOutcome) -> u64 {
    match outcome {
        ModelCheckOutcome::Holds { stats, .. } => stats.product_states,
        other => panic!("expected Holds, got {other:?}"),
    }
}

/// The integer field `field` of `object` at each position of
/// `counterexample`.
fn values(
    world: &World,
    keys: &[&str],
    counterexample: &TemporalCounterexample,
    object: &str,
    field: &str,
) -> Vec<i64> {
    world
        .positions(keys, counterexample)
        .iter()
        .map(|state| world.domain.integer(state, object, field))
        .collect()
}

/// The pure function whose divisor may be zero, which the Kernel checks
/// (FR-495-AC-3).
const DIVIDE: &str = "function q using v(x: Integer): Integer pure { 6 mod x }\n";

fn counter_world(profile: &str, clauses: &[Clause<'_>]) -> World {
    World::with_kernel_functions(
        model_world::counter(),
        CONTRACT,
        DIVIDE,
        profile,
        clauses,
        &[("c", json!({"value": {"integer": "0"}}))],
    )
}

/// The production letters of one instance over the states of one behaviour,
/// recording each position read with its reading.
struct RecordedLetters<'b, L> {
    letters: &'b L,
    instance: ModelInstance,
    states: &'b [ModelSimState],
    reads: Vec<(u64, AtomReading)>,
}

impl<L: ModelLetters> fsl_eval::temporal::TemporalLetters for RecordedLetters<'_, L> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let index = usize::try_from(position.index()).expect("a position");
        let reading = self
            .letters
            .atom(atom, &self.instance, &self.states[index], meter)?;
        self.reads.push((position.index(), reading.clone()));
        Ok(reading)
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        panic!("an `on origin` clause has no activation condition")
    }
}

/// FR-125-AC-6: over the Counter subject, the letter of `always holds(q(2 -
/// c.value) >= 0)` under infinite-trace is true at positions 0 and 1 and
/// undefined at position 2, cause division by zero; `eventually[0,1]
/// holds(q(2 - c.value) = 0 and c.value = 1)` under event-position
/// false-extension, `on origin`, reads positions 0 and 1 only and is true.
#[trace("TC-537", "FR-125-AC-6")]
#[test]
fn a_model_letter_is_read_up_to_its_first_undefined_position_or_its_horizon() {
    use fsl_eval::simulation::TransitionSystem;
    use fsl_eval::temporal::{evaluate, TemporalOutcome, TemporalTrace, UndefinedEvaluation};

    let read = |profile: &str, formula: &str, trace: TemporalTrace| {
        let world = counter_world(
            profile,
            &[Clause {
                activation: ORIGIN,
                formula,
            }],
        );
        let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation)
            .expect("the subject admits");
        let mut states = vec![system.initial_states()[0].clone()];
        loop {
            let expansion = system
                .successors(&states[states.len() - 1])
                .expect("the state expands");
            let Some((_, next)) = expansion.successors.into_iter().next() else {
                break;
            };
            states.push(next);
        }
        let values: Vec<_> = states
            .iter()
            .map(|state| world.domain.integer(state, "c", "value"))
            .collect();
        assert_eq!(values, [0, 1, 2, 3]);
        let node = &world.package.graph().temporal_clauses()[0];
        let letters = system.clause_letters(node).expect("the clause has letters");
        let mut recorded = RecordedLetters {
            letters: &letters,
            instance: ModelInstance {
                population: world.domain.key(world.domain.population),
                key: "c".to_owned(),
            },
            states: &states,
            reads: Vec::new(),
        };
        let outcome = evaluate(
            node.clause(),
            trace,
            &mut recorded,
            &mut Meter::new(UNLIMITED),
        )
        .expect("the trace is readable");
        (outcome, recorded.reads)
    };
    let division = UndefinedCause::Kernel(filament_exact::Undefined::DivisionByZero);

    let (outcome, reads) = read(
        INFINITE,
        "always holds(q(2 - c.value) >= 0)",
        TemporalTrace::Finite { len: 4 },
    );
    assert_eq!(
        outcome,
        TemporalOutcome::Undefined(UndefinedEvaluation {
            at: TemporalPosition::new(2),
            cause: division.clone(),
        })
    );
    assert_eq!(
        reads,
        [
            (0, AtomReading::True),
            (1, AtomReading::True),
            (2, AtomReading::Undefined(division)),
        ]
    );

    let (outcome, reads) = read(
        BOUNDED,
        "eventually[0,1] holds(q(2 - c.value) = 0 and c.value = 1)",
        TemporalTrace::Finite { len: 4 },
    );
    assert_eq!(outcome, TemporalOutcome::Holds);
    assert_eq!(reads, [(0, AtomReading::False), (1, AtomReading::True)]);
}

/// FR-125-AC-14: a key that is unused at initialization stays excluded from
/// the claim's instances even after `make` creates a value-1 Counter there.
/// The temporal proof covers only the initial instances `c` and `d`.
#[trace("TC-520", "FR-125-AC-14")]
#[test]
fn an_operation_created_key_stays_excluded_from_temporal_instances() {
    use fsl_eval::simulation::TransitionSystem;

    let mut domain = model_world::counter();
    let mut document: Value = serde_json::from_slice(&domain.document).expect("counter JSON");
    let counter = document["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|declared| declared["identity"] == domain.key("Counter").node)
        .expect("Counter declaration");
    let operations = counter["operations"].as_array_mut().expect("operations");
    let mut make = operations[0].clone();
    make["identity"] = json!(domain.key("Counter/make").node);
    make["name"] = json!("make");
    make["frame"] = json!({
        "modifies": [], "creates": [domain.key("Counter").node], "deletes": []
    });
    operations.push(make);
    domain.document = document.to_string().into_bytes();

    let clauses = [Clause {
        activation: ORIGIN,
        formula: "always holds(c.value = 0)",
    }];
    let unit = "pre Disabled using v on M::Counter::inc { false }\n";
    let initial = [
        ("c", json!({"value": {"integer": "0"}})),
        ("d", json!({"value": {"integer": "0"}})),
    ];
    let world = World::new(domain, unit, INFINITE, &clauses, &initial);
    let keys = ["c", "d", "z"];
    let system =
        ModelSystem::new(world.subject(&keys, UNLIMITED).simulation).expect("the subject admits");
    let [node] = world.package.graph().temporal_clauses() else {
        panic!("one temporal clause");
    };
    let clause = node.clause();
    assert_eq!(
        system.instances(&clause.over().1).expect("instances"),
        [
            ModelInstance {
                population: world.domain.key(world.domain.population),
                key: "c".to_owned(),
            },
            ModelInstance {
                population: world.domain.key(world.domain.population),
                key: "d".to_owned(),
            },
        ],
    );
    assert_eq!(
        system
            .excluded_keys(&clause.over().1)
            .expect("excluded keys"),
        ["z"],
    );

    let start = &system.initial_states()[0];
    let reachable_created_value_one = system
        .successors(start)
        .expect("the initial state expands")
        .successors
        .iter()
        .any(|(_, state)| {
            state
                .state()
                .populations
                .get(&world.domain.key(world.domain.population))
                .and_then(|objects| objects.get("z"))
                .is_some_and(|_| world.domain.integer(state, "z", "value") == 1)
        });
    assert!(reachable_created_value_one, "make reaches z.value = 1");

    let letters = system
        .clause_letters(node)
        .expect("the production temporal letters admit");
    let outcome = check_model(
        ModelCheckRequest {
            workers: 1,
            temporal_clock_selections: &[],
            subject: world.subject(&keys, UNLIMITED),
            item: ModelCheckItem::Clause {
                clause,
                letters: &letters,
            },
            max_depth: None,
            limits: ModelCheckLimits::default(),
            reductions: Default::default(),
            smt_encoding: Default::default(),
            smt_run: Default::default(),
        },
        &Cancel::new(),
    )
    .expect("checked");
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
}

/// `c.value` read by `test`; `None` is undefined.
fn value(test: fn(i64) -> Option<bool>) -> Atom {
    let domain = model_world::counter();
    Box::new(move |state, object| test(domain.integer(state, object, "value")))
}

/// The `Counter` rows of FR-126-AC-2: over the Counter subject with no
/// `terminal` declaration, under event-position false-extension, `on each`,
/// `eventually[0,3] holds(c.value = 3)` holds and `eventually[0,1]
/// holds(c.value = 3)` is violated with the prefix `0 -inc-> 1`. Its rows
/// over ADR-018 §6's subject are not verified: FR-120's frame grants
/// `attemptUpdate` the `versionNumber` of every object, so one expansion of
/// that subject enumerates 1001 values for each of `a` and `b` and stops at
/// the subject's `max_candidates`, rather than the §6 example's two
/// successors per state.
#[trace("TC-521")]
#[test]
fn bounded_each_clauses_are_decided_on_the_product() {
    let counter = each("Counter");
    let world = counter_world(
        BOUNDED,
        &[
            Clause {
                activation: &counter,
                formula: "eventually[0,3] holds(c.value = 3)",
            },
            Clause {
                activation: &counter,
                formula: "eventually[0,1] holds(c.value = 3)",
            },
        ],
    );
    holds(&checked(&world, 0, &["c"], vec![value(|v| Some(v == 3))]));
    let outcome = checked(&world, 1, &["c"], vec![value(|v| Some(v == 3))]);
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(values(&world, &["c"], counterexample, "c", "value"), [0, 1]);
}

/// FR-126 "Under a bounded profile": at a terminal model state a TP-2
/// monitor reads the rest of its horizon with every atom false. Over the
/// Counter subject, whose chain ends at value 3 after three steps,
/// `eventually[0,5] holds(c.value > 3)` `on origin` is violated by that
/// closure at the terminal state, with the prefix `0 -inc-> 1 -inc-> 2
/// -inc-> 3`, while `eventually[0,5] holds(c.value = 3)` holds.
#[test]
fn a_terminal_state_closes_a_bounded_monitor_with_false_letters() {
    let world = counter_world(
        BOUNDED,
        &[
            Clause {
                activation: ORIGIN,
                formula: "eventually[0,5] holds(c.value > 3)",
            },
            Clause {
                activation: ORIGIN,
                formula: "eventually[0,5] holds(c.value = 3)",
            },
        ],
    );
    let outcome = checked(&world, 0, &["c"], vec![value(|v| Some(v > 3))]);
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2, 3]
    );
    holds(&checked(&world, 1, &["c"], vec![value(|v| Some(v == 3))]));
}

/// FR-126 "Under infinite-trace, a terminal model state SHALL contribute one
/// terminal stutter edge": over the Counter subject, `always (holds(c.value =
/// 3) implies eventually[0,1] holds(c.value = 0))` (TP-3) is violated on the
/// stutter after value 3, whose behaviour is the prefix `0 -inc-> 1 -inc-> 2
/// -inc-> 3` stuttered forever; `always (holds(c.value = 2) implies
/// eventually[0,1] holds(c.value = 3))` holds.
#[test]
fn a_terminal_state_stutters_under_infinite_trace() {
    let world = counter_world(
        INFINITE,
        &[
            Clause {
                activation: ORIGIN,
                formula: "always (holds(c.value = 3) implies eventually[0,1] holds(c.value = 0))",
            },
            Clause {
                activation: ORIGIN,
                formula: "always (holds(c.value = 2) implies eventually[0,1] holds(c.value = 3))",
            },
        ],
    );
    let outcome = checked(
        &world,
        0,
        &["c"],
        vec![value(|v| Some(v == 3)), value(|v| Some(v == 0))],
    );
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2, 3]
    );
    let outcome = checked(
        &world,
        1,
        &["c"],
        vec![value(|v| Some(v == 2)), value(|v| Some(v == 3))],
    );
    assert_eq!(holds(&outcome), 4);
}

/// ADR-018 SM-6 on the product: the monitor keeps only states with an
/// infinite continuation, so it rejects at the first bad prefix. Over the
/// Counter subject, `always (holds(c.value <= 1) or eventually[0,2] false)`
/// is `always holds(c.value <= 1)`; it is violated at value 2, with the
/// prefix `0 -inc-> 1 -inc-> 2`, although the dead `eventually[0,2] false`
/// branch would only expire two positions later.
#[test]
fn the_monitor_rejects_at_the_first_bad_prefix() {
    let world = counter_world(
        INFINITE,
        &[Clause {
            activation: ORIGIN,
            formula: "always (holds(c.value <= 1) or eventually[0,2] false)",
        }],
    );
    let outcome = checked(&world, 0, &["c"], vec![value(|v| Some(v <= 1))]);
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2]
    );
}

/// The FR-126-AC-6 rows over the `Restless` subject (FR-126-AC-4: `fail`
/// can repeat forever): `always (holds(not c.healthy) implies
/// eventually[0,100] holds(c.healthy))` with `max_automaton_states` 50
/// stops `Stopped{ResourceExhausted, {MaxAutomatonStates, 50}}` with 50
/// automaton states and no counterexample; with the default limit it is
/// violated by a finite prefix with at least 101 consecutive unhealthy
/// positions.
#[trace("TC-521")]
#[test]
fn a_long_interval_materializes_automaton_states_as_the_product_reaches_them() {
    let world = World::new(
        model_world::health(),
        RESTLESS,
        INFINITE,
        &[Clause {
            activation: ORIGIN,
            formula: "always (holds(not c.healthy) implies eventually[0,100] holds(c.healthy))",
        }],
        &[(
            "s",
            json!({"healthy": {"boolean": true}, "failures": {"integer": "0"}}),
        )],
    );
    let healthy = |negated: bool| -> Atom {
        let domain = model_world::health();
        Box::new(move |state, object| Some(domain.boolean(state, object, "healthy") != negated))
    };
    let tight = ModelCheckLimits {
        max_automaton_states: 50,
        ..ModelCheckLimits::default()
    };
    let outcome = run(
        &world,
        0,
        &["s"],
        vec![healthy(true), healthy(false)],
        tight,
    )
    .expect("checked");
    let ModelCheckOutcome::Stopped {
        cause,
        limit,
        stats,
    } = outcome
    else {
        panic!("expected a stopped run, got {outcome:?}");
    };
    assert_eq!(cause, IncompleteCause::ResourceExhausted);
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxAutomatonStates,
            value: 50
        })
    );
    assert_eq!(stats.automaton_states, 50);

    let outcome = checked(&world, 0, &["s"], vec![healthy(true), healthy(false)]);
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    let mut run = 0;
    let mut longest = 0;
    for state in world.positions(&["s"], counterexample) {
        run = if world.domain.boolean(&state, "s", "healthy") {
            0
        } else {
            run + 1
        };
        longest = longest.max(run);
    }
    assert!(longest >= 101, "{longest} consecutive unhealthy positions");
}

/// The TP-3 and TP-2 rows of FR-126-AC-9 over the Counter subject, the
/// division written `mod` as in `crate::model_check` (S3 types `mod` over
/// integers): `always (holds(c.value <= 0) and holds(6 mod (2 - c.value) >=
/// 0))` is violated with `kind: Formula` and the prefix `0 -inc-> 1`, before
/// the atom is undefined at position 2; `eventually[0,1] holds(6 mod (2 -
/// c.value) = 0 and c.value = 1)` under event-position false-extension, `on
/// origin`, false at position 0 and true at 1, holds, reading positions 0
/// and 1 only, so the undefined atom at position 2 is never evaluated.
#[trace("TC-537", "FR-126-AC-9")]
#[test]
fn a_formula_violation_before_an_undefined_position_and_an_unread_position() {
    let world = counter_world(
        INFINITE,
        &[Clause {
            activation: ORIGIN,
            formula: "always (holds(c.value <= 0) and holds(q(2 - c.value) >= 0))",
        }],
    );
    let outcome = checked(
        &world,
        0,
        &["c"],
        vec![
            value(|v| Some(v <= 0)),
            value(|v| (v != 2).then(|| 6 % (2 - v) >= 0)),
        ],
    );
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(values(&world, &["c"], counterexample, "c", "value"), [0, 1]);

    let world = counter_world(
        BOUNDED,
        &[Clause {
            activation: ORIGIN,
            formula: "eventually[0,1] holds(q(2 - c.value) = 0 and c.value = 1)",
        }],
    );
    let outcome = checked(
        &world,
        0,
        &["c"],
        vec![value(|v| (v != 2).then(|| 6 % (2 - v) == 0 && v == 1))],
    );
    holds(&outcome);
}

/// The counterexample's undefined evaluation and the counter values on its
/// prefix.
fn undefined(world: &World, outcome: &ModelCheckOutcome) -> (u64, Vec<i64>) {
    let counterexample = violated(outcome);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(filament_exact::Undefined::DivisionByZero)
    );
    (
        evaluation.site.position,
        values(world, &["c"], counterexample, "c", "value"),
    )
}

/// ADR-018 UE-1: a claim reads every atom of the claim at every position it
/// reads, also once its monitor no longer depends on the atom. Over the
/// Counter subject under infinite-trace, `holds(c.value >= 0) release
/// holds(6 mod (2 - c.value) >= 0)` is discharged at position 0, yet its
/// second atom is still read at position 2, where it is undefined: the run is
/// violated with `UndefinedEvaluation` at position 2, prefix `0 -inc-> 1
/// -inc-> 2`.
#[test]
fn a_discharged_infinite_trace_claim_still_reads_every_atom() {
    let world = counter_world(
        INFINITE,
        &[Clause {
            activation: ORIGIN,
            formula: "holds(c.value >= 0) release holds(q(2 - c.value) >= 0)",
        }],
    );
    let outcome = checked(
        &world,
        0,
        &["c"],
        vec![
            value(|v| Some(v >= 0)),
            value(|v| (v != 2).then(|| 6 % (2 - v) >= 0)),
        ],
    );
    assert_eq!(undefined(&world, &outcome), (2, vec![0, 1, 2]));
}

/// ADR-018 UE-1 for TP-2 `on origin`: the claim reads positions 0 to its
/// horizon `h`. Over the Counter subject under event-position
/// false-extension, `holds(c.value = 1) implies eventually[0,3] holds(6 mod
/// (2 - c.value) = 0)` is decided true at position 0, whose antecedent is
/// false, yet position 2 is within the horizon 3 and reads the consequent's
/// atom undefined there: violated with `UndefinedEvaluation` at position 2.
#[test]
fn a_decided_bounded_claim_still_reads_every_atom_up_to_its_horizon() {
    let world = counter_world(
        BOUNDED,
        &[Clause {
            activation: ORIGIN,
            formula: "holds(c.value = 1) implies eventually[0,3] holds(q(2 - c.value) = 0)",
        }],
    );
    let outcome = checked(
        &world,
        0,
        &["c"],
        vec![
            value(|v| Some(v == 1)),
            value(|v| (v != 2).then(|| 6 % (2 - v) == 0)),
        ],
    );
    assert_eq!(undefined(&world, &outcome), (2, vec![0, 1, 2]));
}

/// FR-125-AC-18 for the `Counter` unit through its production letters: the
/// guard `q.value >= 1` of `on each (q: Counter)` is read from the state at
/// each position, its footprint is the `Counter.value` read through the
/// receiver (the trigger aliases the clause's `over`), and a run under it
/// violates only where the guard holds while `when (false)` holds vacuously.
#[trace("TC-520", "FR-125-AC-18")]
#[test]
fn the_production_letters_read_the_counter_guard_and_its_footprint() {
    use fsl_eval::simulation::model::ModelSystem;
    use fsl_semantics::check::footprint::{FootprintObject, SymbolicLocation};

    let run = |guard: &str| {
        let activation = format!("{} when ({guard})", each("Counter"));
        let world = counter_world(
            INFINITE,
            &[Clause {
                activation: &activation,
                formula: "always holds(c.value <= 2)",
            }],
        );
        let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation)
            .expect("the subject admits");
        let node = &world.package.graph().temporal_clauses()[0];
        let letters = system.clause_letters(node).expect("the clause has letters");
        let instance = ModelInstance {
            population: world.domain.key(world.domain.population),
            key: "c".to_owned(),
        };
        let start = system.initial_states()[0].clone();
        let reading = letters
            .activation(&instance, &start, &mut Meter::new(UNLIMITED))
            .expect("the guard reads");
        let reads = letters
            .activation_reads()
            .expect("the guard has a footprint");
        let outcome = check_model(
            ModelCheckRequest {
                workers: 1,
                temporal_clock_selections: &crate::fill::tcb_01_event_position_selection(
                    &world.package,
                ),
                subject: world.subject(&["c"], UNLIMITED),
                item: ModelCheckItem::Clause {
                    clause: node.clause(),
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                smt_encoding: Default::default(),
                smt_run: Default::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("checked");
        (reading, reads, outcome, world.domain.key("Counter/value"))
    };
    let (reading, reads, outcome, value_key) = run("q.value >= 1");
    // The initial value is 0, so the guard is false there.
    assert_eq!(reading, AtomReading::False);
    assert_eq!(
        reads.into_iter().collect::<Vec<_>>(),
        [SymbolicLocation::Field {
            object: FootprintObject::Receiver,
            field: value_key,
        }]
    );
    assert_eq!(violated(&outcome).kind, CounterexampleKind::Formula);

    let (reading, reads, outcome, _) = run("false");
    assert_eq!(reading, AtomReading::False);
    assert!(reads.is_empty());
    holds(&outcome);
}

/// FR-125-AC-18: each guarded row is compiled from source and decided through
/// the production ClauseLetters, and POR visibility from the guard-only
/// footprint.
#[trace("TC-520", "FR-125-AC-18")]
#[test]
fn production_letters_decide_each_guard_row_and_include_guard_only_por_visibility() {
    use fsl_eval::simulation::model::ModelSystem;
    use fsl_eval::simulation::TransitionSystem;
    use fsl_semantics::check::footprint::{FootprintObject, SymbolicLocation};

    let run = |profile: &str, guard: &str, formula: &str| {
        let activation = format!("{} when ({guard})", each("Counter"));
        let world = counter_world(
            profile,
            &[Clause {
                activation: &activation,
                formula,
            }],
        );
        let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation)
            .expect("the subject admits");
        let node = &world.package.graph().temporal_clauses()[0];
        let letters = system
            .clause_letters(node)
            .expect("the production letters build");
        let outcome = check_model(
            ModelCheckRequest {
                workers: 1,
                temporal_clock_selections: &crate::fill::tcb_01_event_position_selection(
                    &world.package,
                ),
                subject: world.subject(&["c"], UNLIMITED),
                item: ModelCheckItem::Clause {
                    clause: node.clause(),
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                smt_encoding: Default::default(),
                smt_run: Default::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("checked");
        (world, outcome)
    };

    let (event_world, event) = run(BOUNDED, "c.value = 1", "eventually[0,1] holds(c.value = 3)");
    let event = violated(&event);
    assert_eq!(values(&event_world, &["c"], event, "c", "value"), [0, 1, 2]);

    let (_, exhaustive) = run(
        BOUNDED,
        "c.value >= 2",
        "eventually[0,1] holds(c.value = 3)",
    );
    assert!(matches!(exhaustive, ModelCheckOutcome::Holds { .. }));

    let (live_world, live) = run(INFINITE, "c.value >= 3", "always holds(c.value <= 1)");
    let live = violated(&live);
    assert_eq!(
        values(&live_world, &["c"], live, "c", "value"),
        [0, 1, 2, 3]
    );

    let (undefined_world, undefined_outcome) =
        run(INFINITE, "q(2 - c.value) = 0", "always holds(true)");
    assert_eq!(
        undefined(&undefined_world, &undefined_outcome),
        (2, vec![0, 1, 2])
    );
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &violated(&undefined_outcome).kind
    else {
        panic!("expected an undefined evaluation");
    };
    assert_eq!(evaluation.site.locus, UndefinedLocus::Activation);

    // The atom has no reads; the activation guard alone reads Counter.value.
    let activation = format!("{} when (c.value >= 3)", each("Counter"));
    let por_world = counter_world(
        INFINITE,
        &[Clause {
            activation: &activation,
            formula: "always holds(true)",
        }],
    );
    let system = ModelSystem::new(por_world.subject(&["c"], UNLIMITED).simulation)
        .expect("the subject admits");
    let node = &por_world.package.graph().temporal_clauses()[0];
    let letters = system
        .clause_letters(node)
        .expect("the production letters build");
    let automaton = translate(node.clause(), 1 << 20).expect("the clause translates");
    let instance = ModelInstance {
        population: por_world.domain.key(por_world.domain.population),
        key: "c".to_owned(),
    };
    let por = Por::new(
        &system,
        Some(Atoms {
            letters: &letters,
            nodes: automaton.atoms(),
            guarded: true,
        }),
        std::slice::from_ref(&instance),
        node.clause().fairness(),
        None,
    )
    .expect("the production footprints derive");
    let start = system.initial_states()[0].clone();
    let inc = system
        .successors(&start)
        .expect("the initial state expands")
        .successors
        .into_iter()
        .find(|(_, state)| por_world.domain.integer(state, "c", "value") == 1)
        .expect("inc is enabled")
        .0;
    assert!(por
        .visible(&start, &inc)
        .expect("the visibility is derived"));
    assert!(letters
        .activation_reads()
        .expect("the guard has a footprint")
        .contains(&SymbolicLocation::Field {
            object: FootprintObject::Receiver,
            field: por_world.domain.key("Counter/value"),
        }));
}

/// A TP-4 clause is decided by the liveness engine (here its proof is
/// FR-339's component certificate), and a guarded `on each` activation is
/// no reason to refuse it: the engine reads the `when` condition through the
/// letters (FR-126-AC-16).
#[trace("TC-521", "FR-126-AC-16")]
#[test]
fn a_liveness_clause_is_decided_with_or_without_a_guarded_activation() {
    let guarded = format!("{} when (true)", each("Counter"));
    let proved = |profile, activation: &str, formula, guarded: bool| {
        let world = counter_world(
            profile,
            &[Clause {
                activation,
                formula,
            }],
        );
        let letters = Letters::new(&world.clauses[0], vec![value(|v| Some(v == 3))]);
        let letters = if guarded {
            letters.guarded(value(|_| Some(true)), crate::units::unread())
        } else {
            letters
        };
        run_letters(&world, 0, &["c"], &letters, ModelCheckLimits::default()).expect("checked")
    };
    let refusal = proved(
        INFINITE,
        ORIGIN,
        "always eventually holds(c.value = 3)",
        false,
    );
    assert!(
        matches!(
            refusal,
            ModelCheckOutcome::Holds {
                certificate: fsl_replay::ProofCertificate::Component(_),
                ..
            }
        ),
        "{refusal:?}"
    );
    let refusal = proved(
        INFINITE,
        &guarded,
        "always eventually holds(c.value = 3)",
        true,
    );
    assert!(
        matches!(
            refusal,
            ModelCheckOutcome::Holds {
                certificate: fsl_replay::ProofCertificate::Component(_),
                ..
            }
        ),
        "{refusal:?}"
    );
}

/// FR-126-AC-6's empty-prefix row: from an initial state whose `pre`
/// evaluates undefined, `always holds(true)` is violated by the empty prefix
/// at that state, with `kind: UndefinedEvaluation` at position 0. Over the
/// Counter subject with `pre CanInc { 6 mod (2 - self.value) >= 0 }` from
/// value 2, the modulus is undefined (`DivisionByZero`) in the initial
/// state's own expansion. FR-120-AC-9's `test/tallies` subject, whose
/// `sum<Low>` would be undefined (`SumOutOfDomain`), is not constructible:
/// FSpec FR-145's range proof refuses at S3 a `sum<N>` it cannot prove within
/// `N`, and `SumOutOfDomain` is raised only by a `sum` checked under
/// `CheckMode::Kernel`, so no linked package reaches it.
#[trace("TC-521", "FR-126-AC-6")]
#[test]
fn an_undefined_precondition_at_the_initial_state_is_an_empty_prefix_violation() {
    let world = World::with_kernel_functions(
        model_world::counter(),
        "pre CanInc using v on M::Counter::inc { q(2 - self.value) >= 0 }\n\
         post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n",
        DIVIDE,
        INFINITE,
        &[Clause {
            activation: ORIGIN,
            formula: "always holds(true)",
        }],
        &[("c", json!({"value": {"integer": "2"}}))],
    );
    let outcome = checked(&world, 0, &["c"], vec![Box::new(|_, _| Some(true))]);
    let counterexample = violated(&outcome);
    assert!(counterexample.prefix.is_empty(), "{counterexample:?}");
    assert_eq!(counterexample.lasso, None);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.position, 0);
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(filament_exact::Undefined::DivisionByZero)
    );
    assert_eq!(values(&world, &["c"], counterexample, "c", "value"), [2]);
}

/// A guarded world: the Counter subject with the one `on each` clause
/// `activation` over `profile` and `formula`.
fn guarded_world(profile: &str, formula: &str) -> World {
    let activation = format!("{} when (true)", each("Counter"));
    counter_world(
        profile,
        &[Clause {
            activation: &activation,
            formula,
        }],
    )
}

/// The run of the guarded world's one clause, its `holds` atoms read by
/// `atoms` and its `when` condition by `guard`.
fn guarded_run(
    world: &World,
    atoms: Vec<Atom>,
    guard: fn(i64) -> Option<bool>,
) -> ModelCheckOutcome {
    let letters =
        Letters::new(&world.clauses[0], atoms).guarded(value(guard), crate::units::unread());
    run_letters(world, 0, &["c"], &letters, ModelCheckLimits::default()).expect("checked")
}

/// A letter seam that denies the selected read through the real evaluation
/// meter, preserving the counter named by the denied charge.
struct MeteredLetters<'a> {
    inner: &'a Letters,
    charge_atom: bool,
    charge_activation: bool,
    counter: LimitKind,
}

impl MeteredLetters<'_> {
    fn charge(&self, meter: &mut Meter) -> Result<(), Incomplete> {
        let charge = Charge::new(ChargePoint::GraphExpand);
        let charge = match self.counter {
            LimitKind::WorkUnits => charge,
            counter => charge.size(counter, 1),
        };
        meter.charge(charge)
    }
}

impl ModelLetters for MeteredLetters<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        if self.charge_atom {
            self.charge(meter)?;
        }
        self.inner.atom(atom, instance, state, meter)
    }

    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        if self.charge_activation {
            self.charge(meter).map_err(AtomStop::Incomplete)?;
        }
        self.inner.activation(instance, state, meter)
    }

    fn reads(&self, atom: FormulaNodeId) -> crate::units::Reads {
        self.inner.reads(atom)
    }

    fn activation_reads(&self) -> crate::units::Reads {
        self.inner.activation_reads()
    }
}

#[trace("TC-521", "FR-126-AC-5", "FR-126-AC-19", "FR-127-AC-4")]
#[test]
fn a_denied_letter_or_guard_charge_stops_with_the_accounting_setting() {
    for (guarded, counter, setting) in [
        (false, LimitKind::WorkUnits, "accounting.work_units"),
        (true, LimitKind::WorkUnits, "accounting.work_units"),
        (
            false,
            LimitKind::ValueOccurrences,
            "accounting.value_occurrences",
        ),
        (
            true,
            LimitKind::ValueOccurrences,
            "accounting.value_occurrences",
        ),
    ] {
        let world = if guarded {
            guarded_world(BOUNDED, "always[0,1] holds(c.value <= 3)")
        } else {
            counter_world(
                BOUNDED,
                &[Clause {
                    activation: ORIGIN,
                    formula: "always[0,1] holds(c.value <= 3)",
                }],
            )
        };
        let letters = Letters::new(&world.clauses[0], vec![value(|v| Some(v <= 3))]);
        let letters = if guarded {
            letters.guarded(value(|_| Some(true)), crate::units::unread())
        } else {
            letters
        };
        let metered = MeteredLetters {
            inner: &letters,
            charge_atom: !guarded,
            charge_activation: guarded,
            counter,
        };
        let mut meter = UNLIMITED;
        match counter {
            LimitKind::WorkUnits => meter.work_units = 0,
            LimitKind::ValueOccurrences => meter.value_occurrences = 0,
            _ => unreachable!("test cases choose a configured counter"),
        }
        let outcome = check_model(
            ModelCheckRequest {
                workers: 1,
                temporal_clock_selections: &crate::fill::tcb_01_event_position_selection(
                    &world.package,
                ),
                subject: world.subject(&["c"], meter),
                item: ModelCheckItem::Clause {
                    clause: &world.clauses[0],
                    letters: &metered,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                smt_encoding: Default::default(),
                smt_run: Default::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("checked");
        let ModelCheckOutcome::Stopped { cause, limit, .. } = outcome else {
            panic!("letter or guard exhaustion must stop the run: {outcome:?}");
        };
        assert_eq!(cause, IncompleteCause::ResourceExhausted);
        let reached = limit.expect("the denied meter counter is named");
        assert_eq!(reached.limit, ModelCheckLimit::EvaluationMeter { counter });
        assert_eq!(reached.value, 0);
        assert_eq!(reached.limit.setting(), setting);
    }
}

/// FR-126-AC-16 for TP-2: a guarded `on each` clause activates at the
/// positions where its `when` condition is true. Over the Counter subject,
/// `eventually[0,1] holds(c.value = 3)` is violated unguarded by the prefix
/// `0 -inc-> 1`; guarded by `c.value = 1` it activates at position 1 alone
/// and is violated by the prefix `0 -inc-> 1 -inc-> 2`; guarded by
/// `c.value >= 2` it activates at values 2 and 3, each of which reaches value 3 within
/// its window, and holds.
#[trace("TC-521", "FR-126-AC-16")]
#[test]
fn a_guarded_bounded_clause_activates_where_its_condition_holds() {
    let world = guarded_world(BOUNDED, "eventually[0,1] holds(c.value = 3)");
    let outcome = guarded_run(&world, vec![value(|v| Some(v == 3))], |v| Some(v == 1));
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2]
    );
    let outcome = guarded_run(&world, vec![value(|v| Some(v == 3))], |v| Some(v >= 2));
    holds(&outcome);
}

/// FR-126-AC-16 for TP-1: `always holds(c.value <= 1)` `on each`, guarded by
/// `c.value >= 3`, is violated at the guarded position 3 by the prefix `0
/// -inc-> 1 -inc-> 2 -inc-> 3`, where the unguarded clause is violated at
/// value 2; guarded by `c.value = 7`, a condition no position meets, it
/// holds. A guarded proof carries no closure certificate.
#[trace("TC-521", "FR-126-AC-16")]
#[test]
fn a_guarded_invariant_clause_is_read_from_its_activating_positions() {
    let activation = format!("{} when (true)", each("Counter"));
    let world = counter_world(
        INFINITE,
        &[Clause {
            activation: &activation,
            formula: "always holds(c.value <= 1)",
        }],
    );
    let outcome = guarded_run(&world, vec![value(|v| Some(v <= 1))], |v| Some(v >= 3));
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2, 3]
    );
    let outcome = guarded_run(&world, vec![value(|v| Some(v <= 1))], |v| Some(v == 7));
    let ModelCheckOutcome::Holds { certificate, .. } = &outcome else {
        panic!("expected Holds, got {outcome:?}");
    };
    let fsl_replay::ProofCertificate::Closure(certificate) = certificate else {
        panic!("a safety clause holds with a closure: {certificate:?}");
    };
    assert!(certificate.states.is_empty(), "{certificate:?}");
}

/// FR-127 and FR-338-AC-30: a TP-3 clause over an unguarded `on each` carries
/// a closure certificate keyed by its monitor's configurations, which the core
/// checker accepts, and the settlement map settles `proved`, `Certified`; the
/// empty certificate is no proof of it. Over the Counter subject, `on each
/// always (p implies eventually[0,2] p)` holds with `p` true at every value.
#[trace("TC-521", "FR-127-AC-1", "FR-338-AC-30")]
#[test]
fn an_on_each_past_or_interval_proof_settles_certified() {
    let activation = each("Counter");
    let world = counter_world(
        INFINITE,
        &[Clause {
            activation: &activation,
            formula: "always (holds(c.value <= 3) implies eventually[0,2] holds(c.value <= 3))",
        }],
    );
    let letters = Letters::new(
        &world.clauses[0],
        vec![value(|v| Some(v <= 3)), value(|v| Some(v <= 3))],
    );
    let outcome = run_letters(&world, 0, &["c"], &letters, ModelCheckLimits::default())
        .expect("checked");
    let ModelCheckOutcome::Holds { certificate, .. } = &outcome else {
        panic!("expected Holds, got {outcome:?}");
    };
    let fsl_replay::ProofCertificate::Closure(certificate) = certificate else {
        panic!("a safety clause holds with a closure: {certificate:?}");
    };
    assert!(!certificate.states.is_empty(), "{certificate:?}");
    let temporal_clock_selections = crate::fill::tcb_01_event_position_selection(&world.package);
    let request = CertificateRequest {
        temporal_clock_selections: &temporal_clock_selections,
        subject: world.subject(&["c"], UNLIMITED),
        item: ModelCheckItem::Clause {
            clause: &world.clauses[0],
            letters: &letters,
        },
        smt_encoding: SmtEncodingLimits::default(),
    };
    let record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &request,
        &Cancel::new(),
        |_| panic!("a proof replays nothing"),
    );
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: fsl_replay::ProofBasis::Exhaustive,
            certification: fsl_replay::Certification::Certified,
        }
    );
}

/// FR-126-AC-16: a `when` condition with no value at a position the monitor
/// reads ends the run with `UndefinedEvaluation` located at the activation,
/// not read as false. `eventually[0,3] holds(c.value = 3)` `on each`, its
/// condition undefined at value 2, is violated at position 2 with the prefix
/// `0 -inc-> 1 -inc-> 2`.
#[trace("TC-521", "FR-126-AC-16")]
#[test]
fn an_undefined_activation_condition_is_an_undefined_evaluation() {
    let world = guarded_world(BOUNDED, "eventually[0,3] holds(c.value = 3)");
    let outcome = guarded_run(&world, vec![value(|v| Some(v == 3))], |v| {
        (v != 2).then_some(true)
    });
    let counterexample = violated(&outcome);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.locus, UndefinedLocus::Activation);
    assert_eq!(evaluation.site.position, 2);
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(filament_exact::Undefined::DivisionByZero)
    );
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2]
    );
}

/// The `Counter` package whose object type also declares `held`, an optional
/// field of `value`'s type that no operation modifies, and whose `inc` writes
/// its receiver's `value` only (FR-154).
fn holding_counter() -> model_world::Domain {
    let counter = model_world::counter();
    let mut document: Value =
        serde_json::from_slice(&counter.document).expect("the Counter document is JSON");
    let object = document["types"]
        .as_array_mut()
        .expect("the document declares types")
        .iter_mut()
        .find(|declared| declared.get("fields").is_some())
        .expect("the document declares the object type");
    for operation in object["operations"]
        .as_array_mut()
        .expect("the object type declares operations")
    {
        let written = operation["frame"]["modifies"].take();
        operation["frame"]["modifies"] = json!([]);
        operation["frame"]["modifies_receiver"] = written;
    }
    let fields = object["fields"]
        .as_array_mut()
        .expect("the object type declares its fields");
    let mut held: Value =
        serde_json::from_str(&fields[0].to_string().replace("/value\"", "/held\""))
            .expect("the renamed field is JSON");
    held["name"] = json!("held");
    held["presence"] = json!("optional");
    fields.push(held);
    model_world::Domain {
        document: document.to_string().into_bytes(),
        ..counter
    }
}

/// FR-315-AC-58: over the hand-built package whose guard reads `value(c.held)`
/// where `c.value >= 2`, the one path FR-315's query admits, `inc` on `d`
/// twice, ends where the guard of `d`, which holds no `held`, has no value.
/// FR-128 replays the counterexample read from that model, its condition read
/// by the production letters, as an undefined activation, and the same payload
/// bound over `c` is not reproduced. EN-1 searches every depth and reports the
/// first instance in canonical order with a finding (FR-126-AC-11), `c`, whose
/// formula a longer path violates.
#[trace("TC-1201", "FR-315-AC-58")]
#[test]
fn a_hand_built_guards_undefined_activation_replays_for_its_instance() {
    let activation = format!(
        "{} when (present(c.held) and (c.value < 2 or value(c.held) = 0))",
        each("Counter")
    );
    let clauses = [Clause {
        activation: &activation,
        formula: "eventually[0,3] holds(c.value = 3)",
    }];
    let start = [
        (
            "c",
            json!({"value": {"integer": "0"}, "held": {"present": {"integer": "0"}}}),
        ),
        (
            "d",
            json!({"value": {"integer": "0"}, "held": {"absent": {}}}),
        ),
    ];
    let source = World::new(holding_counter(), CONTRACT, BOUNDED, &clauses, &start);
    let built = World::with_edited_graph(
        holding_counter(),
        CONTRACT,
        |graph| graph.drop_temporal_guard_left_conjunct_after_s3(0),
        BOUNDED,
        &clauses,
        &start,
    );
    let keys = ["c", "d"];
    let node = &built.package.graph().temporal_clauses()[0];
    let system =
        ModelSystem::new(built.subject(&keys, UNLIMITED).simulation).expect("the subject admits");
    let letters = system.clause_letters(node).expect("the clause has letters");
    let selections = crate::fill::tcb_01_event_position_selection(&built.package);
    let outcome = check_model(
        ModelCheckRequest {
            workers: 1,
            temporal_clock_selections: &selections,
            subject: built.subject(&keys, UNLIMITED),
            item: ModelCheckItem::Clause {
                clause: &built.clauses[0],
                letters: &letters,
            },
            max_depth: None,
            limits: ModelCheckLimits::default(),
            smt_encoding: Default::default(),
            smt_run: Default::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
    .expect("checked");
    let found = violated(&outcome);
    assert_eq!(
        found.over_binding.as_ref().map(|over| over.key.as_str()),
        Some("c")
    );
    assert_eq!(found.kind, CounterexampleKind::Formula);

    // The counterexample read from the query's satisfiable model (FR-315 rules
    // 4b and 4c): `inc` on `d` twice, bound over `d`.
    let mut state = system.initial()[0].clone();
    let mut prefix = Vec::new();
    for raised in [1, 2] {
        let (transition, next) = system
            .successors(&state)
            .expect("the state expands")
            .successors
            .into_iter()
            .find(|(_, next)| built.domain.integer(next, "d", "value") == raised)
            .expect("`inc` on `d` is enabled");
        prefix.push(ModelStep {
            transition,
            post_state: system
                .state_key(&next, fsl_foundation::IdentityLimits::default())
                .expect("the state encodes")
                .1,
            delay: None,
        });
        state = next;
    }
    let bound = |key: &str| ModelInstance {
        population: built.domain.key(built.domain.population),
        key: key.to_owned(),
    };
    let claimed = UndefinedEvaluation {
        site: UndefinedSite {
            position: 2,
            state_key: prefix[1].post_state.clone(),
            locus: UndefinedLocus::Activation,
            transition: None,
            time: None,
        },
        cause: UndefinedCause::Kernel(filament_exact::Undefined::NoneValue),
    };
    let counterexample = TemporalCounterexample {
        initial_state: 0,
        prefix,
        over_binding: Some(bound("d")),
        fairness: node.clause().fairness().to_vec(),
        kind: CounterexampleKind::UndefinedEvaluation(claimed.clone()),
        lasso: None,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: Vec::new(),
    };
    assert_eq!(
        values(&built, &keys, &counterexample, "c", "value"),
        [0, 0, 0]
    );
    assert_eq!(
        values(&built, &keys, &counterexample, "d", "value"),
        [0, 1, 2]
    );

    let identity = node.identity().clone();
    let occurrence = OccurrenceKey::new(identity.clone(), node.claim().clone());
    let subject = ModelTraceSubject {
        timed: None,
        initial: source.initial.clone(),
        universes: crate::counter::universe(&keys),
        expansion: ExpansionLimits {
            max_candidates: 1_000_000,
        },
        stutter_closure: false,
    };
    let replay = |payload: TemporalCounterexample| {
        replay_model_trace_with_temporal_selections(
            request(
                &source.package,
                &selections,
                source.domain.document.as_slice(),
                source.snapshot_bytes(),
                &source.unit,
            ),
            &subject,
            &envelope_at(payload, identity.clone(), occurrence.clone()),
            &letters,
            &selections,
            ReplayLimits::default(),
        )
        .expect("replays")
        .settlement
    };
    assert_eq!(
        replay(counterexample.clone()),
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Undefined(claimed),
            trace_position: Some(2),
        }
    );
    let mut misbound = counterexample;
    misbound.over_binding = Some(bound("c"));
    let settled = replay(misbound);
    assert!(
        matches!(
            settled,
            ModelTraceSettlement::Inconclusive(DisagreementCause::Verdicts { .. })
        ),
        "{settled:?}"
    );
}

/// FR-126-AC-16 under partial-order reduction (FR-156): a partial-order run
/// of a guarded clause asks the letter seam for the guard's footprint, and a
/// guard whose footprint has no rule refuses the run `Footprint` before any
/// state is explored; the unreduced run reads no footprint. A guard that
/// reads nothing leaves the reduced and the unreduced run both violating the
/// clause. Which transitions the footprint makes visible is
/// `crate::partial_order`'s test of the same name.
#[trace("TC-521", "FR-126-AC-16")]
#[test]
fn a_partial_order_run_reads_the_guards_footprint() {
    let activation = format!("{} when (true)", each("Counter"));
    let world = counter_world(
        INFINITE,
        &[Clause {
            activation: &activation,
            formula: "always holds(c.value <= 2)",
        }],
    );
    let run = |partial_order: bool, reads: crate::units::Reads| {
        let letters = Letters::new(&world.clauses[0], vec![Box::new(|_, _| Some(false))])
            .guarded(value(|v| Some(v >= 1)), reads);
        check_model(
            ModelCheckRequest {
                workers: 1,
                temporal_clock_selections: &[],
                subject: world.subject(&["c"], UNLIMITED),
                item: ModelCheckItem::Clause {
                    clause: &world.clauses[0],
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                smt_encoding: Default::default(),
                smt_run: Default::default(),
                reductions: fsl_route::reductions::Reductions {
                    partial_order,
                    ..Default::default()
                },
            },
            &Cancel::new(),
        )
    };
    let no_rule = || {
        Err(
            fsl_semantics::check::footprint::FootprintRefusal::UnsupportedForm {
                clause: "C0".to_owned(),
                form: "when",
            },
        )
    };
    for (partial_order, reads) in [
        (false, crate::units::unread()),
        (true, crate::units::unread()),
        (false, no_rule()),
    ] {
        let outcome = run(partial_order, reads).expect("checked");
        assert_eq!(violated(&outcome).kind, CounterexampleKind::Formula);
    }
    let refusal = run(true, no_rule()).expect_err("the footprint has no rule");
    assert!(
        matches!(refusal, ModelCheckRefusal::Footprint(_)),
        "{refusal:?}"
    );
}
