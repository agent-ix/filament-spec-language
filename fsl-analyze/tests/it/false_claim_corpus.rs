// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051's false-claim corpus and oracle isolation checks.

use ix_trace_rs::trace;

struct FalseSeed {
    issue: &'static str,
    route: &'static str,
    claim: &'static str,
    hand_value: &'static str,
    derivation: &'static str,
}

/// FR-1051-AC-17's six historical false proofs. Each row records the route,
/// claim, and the value calculation or concrete falsity witness from the
/// requirement's seed table.
#[trace("FR-1051-AC-17")]
#[test]
fn ac17_corpus_names_all_six_false_proofs_and_their_hand_values() {
    let corpus = [
        FalseSeed {
            issue: "QSL-923",
            route: "R-38",
            claim: "(not holds(l@a=1)) and eventually holds(l@a=1)",
            hand_value: "false",
            derivation: "step(0) loops at (0,0), so l is 0 at every position",
        },
        FalseSeed {
            issue: "QSL-3324",
            route: "R-17",
            claim: "always[0,0] eventually[2,2] P",
            hand_value: "false",
            derivation: "the race advances 3 time units, skipping the required position at 2",
        },
        FalseSeed {
            issue: "QSL-3414",
            route: "R-13",
            claim: ">= quantile with a complete claim-side upper bound",
            hand_value: "initial value 0; a reachable tight activation violates the bound",
            derivation: "the initial state is inactive and has value 0; breadth-first activation reaches the tight state",
        },
        FalseSeed {
            issue: "QSL-3444",
            route: "R-18",
            claim: "eventually[0 ms,4 ms] holds(c.late), probability <= 0.999",
            hand_value: "maximum 1, minimum 0",
            derivation: "first at 1/2 ms and third at absolute time 6/5 ms meet the strict guards; never taking third gives 0",
        },
        FalseSeed {
            issue: "QSL-3445",
            route: "R-17",
            claim: "elapsed quantile 0.4 <= 1 ms under every scheduler",
            hand_value: "P(M <= 1 ms)=0 for t in (0,1); integer delays give at least 1/2",
            derivation: "for lost, M=2-t; for not lost, M=t+1, both exceed 1 at every interior t",
        },
        FalseSeed {
            issue: "QSL-3478",
            route: "R-27 and R-28",
            claim: "always possible phase>=2; unique path phase=1 to phase>=2",
            hand_value: "false for both claims",
            derivation: "every t>0 run reaches a trapped phase-1 state; zero is the only delay enabling the phase-2 transition",
        },
    ];
    assert_eq!(corpus.len(), 6);
    for seed in corpus {
        assert!(!seed.issue.is_empty(), "seed issue is named");
        assert!(!seed.route.is_empty(), "{} has a route", seed.issue);
        assert!(!seed.claim.is_empty(), "{} has a claim", seed.issue);
        assert!(
            !seed.hand_value.is_empty(),
            "{} has a hand value",
            seed.issue
        );
        assert!(
            !seed.derivation.is_empty(),
            "{} has a derivation",
            seed.issue
        );
    }
}

#[trace("FR-1051-AC-17")]
#[test]
fn reference_interprets_a_checked_postcondition_over_pre_and_current_states() {
    use filament_exact::{
        Integer, NodeKey, ObjectId, ObjectReference, PathSegment, UniverseId, Value,
    };

    let mut model = crate::prob_world::coin("");
    model.body.push_str(
        "\npost Increment using ev on M::Coin::drop { pre(self.v) = 0 and self.v = 1 }\n",
    );
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the postcondition passes front-end checking");
    let graph = world.package.graph();
    let clause = graph
        .state_clause("Increment")
        .expect("the named postcondition is in the checked graph");
    let receiver = Value::Reference(ObjectReference::new(
        UniverseId::new(
            NodeKey::new("test/coin", vec![PathSegment::Text("Coin".to_owned())], 0)
                .expect("the fixture identity is well formed"),
        ),
        clause.context(),
        ObjectId::new("c").expect("the fixture object id is non-empty"),
    ));
    let before = fsl_definitional_reference::ReferenceFiniteState::new([(
        "c".to_owned(),
        "v".to_owned(),
        Value::Integer(Integer::from(0_i64)),
    )])
    .expect("the pre-state has one value per field");
    let after = fsl_definitional_reference::ReferenceFiniteState::new([(
        "c".to_owned(),
        "v".to_owned(),
        Value::Integer(Integer::from(1_i64)),
    )])
    .expect("the current state has one value per field");

    assert!(fsl_definitional_reference::evaluate_boolean(
        graph,
        clause.body(),
        &[receiver],
        &before,
        &after,
    )
    .expect("the reference evaluates checked equality and conjunction nodes"));
}

#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_an_uninterpreted_conjunct_even_after_false() {
    use filament_exact::{
        Integer, NodeKey, ObjectId, ObjectReference, PathSegment, UniverseId, Value,
    };

    let mut model = crate::prob_world::coin("");
    model
        .body
        .push_str("\npost Refuse using ev on M::Coin::drop { false and self.v * self.v = 0 }\n");
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the checked graph admits the expression outside the reference fragment");
    let graph = world.package.graph();
    let clause = graph
        .state_clause("Refuse")
        .expect("the expression is present in the checked graph");
    let receiver = Value::Reference(ObjectReference::new(
        UniverseId::new(
            NodeKey::new("test/coin", vec![PathSegment::Text("Coin".to_owned())], 0)
                .expect("the seed receiver identity is valid"),
        ),
        clause.context(),
        ObjectId::new("c").expect("the seed object id is non-empty"),
    ));
    let state = fsl_definitional_reference::ReferenceFiniteState::new([(
        "c".to_owned(),
        "v".to_owned(),
        Value::Integer(Integer::zero()),
    )])
    .expect("the finite initial state is admitted");
    let refusal = fsl_definitional_reference::evaluate_boolean(
        graph,
        clause.body(),
        &[receiver],
        &state,
        &state,
    )
    .expect_err("the false left operand cannot hide an unimplemented construct");
    assert_eq!(refusal.construct, "multiplication");
}

/// QSL-923's `step(0)` lasso keeps `l = 0`, so `not l = 1 and eventually
/// l = 1` is false. The same hand valuation is used by both product copies.
#[trace("FR-1051-AC-17")]
#[test]
fn qsl_923_reference_refutes_the_never_set_output_lasso() {
    use filament_exact::{
        Integer, NodeKey, ObjectId, ObjectReference, PathSegment, UniverseId, Value,
    };

    let mut model = crate::prob_world::coin("");
    model
        .body
        .push_str("\ninvariant Output using ev on M::Coin at current { self.v = 1 }\n");
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the finite product seed is admitted by package checking");
    let graph = world.package.graph();
    let atom = graph
        .state_clause("Output")
        .expect("the seed's output predicate is in the checked graph");
    let receiver = Value::Reference(ObjectReference::new(
        UniverseId::new(
            NodeKey::new("test/coin", vec![PathSegment::Text("Coin".to_owned())], 0)
                .expect("the seed receiver identity is valid"),
        ),
        atom.context(),
        ObjectId::new("c").expect("the seed object id is non-empty"),
    ));
    let state = fsl_definitional_reference::ReferenceFiniteState::new([(
        "c".to_owned(),
        "v".to_owned(),
        Value::Integer(Integer::zero()),
    )])
    .expect("the seed has the single finite valuation l=0");
    let lasso = fsl_definitional_reference::ReferenceLasso::new(vec![state], 0)
        .expect("step(0) forever is a one-state lasso");
    assert!(!fsl_definitional_reference::evaluate_boolean(
        graph,
        atom.body(),
        &[receiver.clone()],
        lasso.initial(),
        lasso.initial(),
    )
    .expect("the checked predicate reads the seed state"));
    assert!(!lasso
        .eventually(graph, atom.body(), &[receiver])
        .expect("the lasso interpreter evaluates every distinct position"));
    // Hand derivation: P is false initially and forever, hence (not P) is
    // true but eventually P is false; true AND false is false.
}

/// QSL-3324's three-unit race has no observation at the required time 2, so
/// the nested `[2,2]` eventuality is false even though the path later reaches P.
#[trace("FR-1051-AC-17")]
#[test]
fn qsl_3324_reference_does_not_invent_an_observation_during_a_race_delay() {
    use filament_exact::{
        Integer, NodeKey, ObjectId, ObjectReference, PathSegment, Rational, UniverseId, Value,
    };

    let mut model = crate::prob_world::coin("");
    model
        .body
        .push_str("\ninvariant P using ev on M::Coin at current { self.v = 1 }\n");
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the seed's finite state predicate is checked");
    let graph = world.package.graph();
    let predicate = graph
        .state_clause("P")
        .expect("P is a checked state predicate");
    let receiver = Value::Reference(ObjectReference::new(
        UniverseId::new(
            NodeKey::new("test/coin", vec![PathSegment::Text("Coin".to_owned())], 0)
                .expect("the seed receiver identity is valid"),
        ),
        predicate.context(),
        ObjectId::new("c").expect("the seed object id is non-empty"),
    ));
    let at_start = fsl_definitional_reference::ReferenceFiniteState::new([(
        "c".to_owned(),
        "v".to_owned(),
        Value::Integer(Integer::zero()),
    )])
    .expect("P is false at time zero");
    let after_race = fsl_definitional_reference::ReferenceFiniteState::new([(
        "c".to_owned(),
        "v".to_owned(),
        Value::Integer(Integer::one()),
    )])
    .expect("P becomes true after the race delay");
    let time = |value| Rational::from_integer(Integer::from(value));
    let path = fsl_definitional_reference::ReferenceTimedTrace::new(vec![
        (time(0_i64), at_start),
        (time(3_i64), after_race),
    ])
    .expect("the race jumps directly from 0 to 3");
    assert!(!path
        .eventually_within(
            graph,
            predicate.body(),
            &[receiver],
            &time(0_i64),
            &time(2_i64),
            &time(2_i64),
        )
        .expect("the closed singleton time window is interpreted exactly"));
    // Hand derivation: the only observations are at 0 and 3, neither at 2;
    // the claim's nested event at exactly 2 therefore has value false.
}

/// QSL-3444's strict witness waits to absolute time 1/2 for `first`, resets
/// x, then reaches absolute time 6/5 with x=7/10 and y=6/5 for `third`.
#[trace("FR-1051-AC-17")]
#[test]
fn qsl_3444_reference_evaluates_the_strict_two_clock_witness() {
    use filament_exact::{Integer, Rational};

    let document =
        crate::timed_world::document(&["x", "y"], &[("first", &["x"]), ("third", &["late"])]);
    let body = format!(
        "{}pre First using v on M::Call::first {{ self.x > 0 ms and self.x < 1 ms }}\n\
         post FirstDone using v on M::Call::first {{ self.x = 0 ms }}\n\
         pre Third using v on M::Call::third {{ self.x < 1 ms and self.y > 1 ms }}\n\
         post ThirdDone using v on M::Call::third {{ self.late = true }}\n\
         time invariant Cap when {{ true }} {{ self.y <= 3 ms }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world(
        &document,
        "profile ev = \"filament.temporal.timed/v1\";\n",
        &body,
    );
    let timed = &world.package.graph().timed_models()[0];
    let rational = |n, d| {
        Rational::new(Integer::from(n), Integer::from(d)).expect("nonzero rational denominator")
    };
    let before = fsl_definitional_reference::ReferenceClockState::new(
        timed,
        vec![rational(0_i64, 1_i64), rational(0_i64, 1_i64)],
    )
    .expect("both clocks start at zero");
    let at_first = before
        .delay(&rational(1_i64, 2_i64))
        .expect("wait to t=1/2");
    assert!(at_first
        .satisfies_guard(timed, 0)
        .expect("first has a checked strict clock conjunction"));
    let after_first = at_first
        .apply_zero_resets(timed, 0)
        .expect("first resets x to zero");
    let at_third = after_first
        .delay(&rational(7_i64, 10_i64))
        .expect("advance from absolute t=1/2 to t=6/5");
    assert!(at_third
        .satisfies_guard(timed, 1)
        .expect("third has a checked strict clock conjunction"));
    assert_eq!(at_third.values(), &[rational(7, 10), rational(6, 5)]);
    // Hand derivation: x=7/10<1 and y=6/5>1, so `third` sets late within
    // the [0,4] ms window. The maximizing scheduler therefore has value 1.
}

/// QSL-3445 uses an interior first delay. Both outcomes then take 3/2 ms,
/// giving zero mass at or below 1 ms, less than the claimed 0.4 quantile.
#[trace("FR-1051-AC-17")]
#[test]
fn qsl_3445_reference_computes_zero_mass_for_the_elapsed_quantile_bound() {
    use filament_exact::{Integer, Rational};
    use fsl_definitional_reference::ReferenceMeasureOutcome;

    let half = Rational::new(Integer::one(), Integer::from(2_i64)).unwrap();
    let three_halves = Rational::new(Integer::from(3_i64), Integer::from(2_i64)).unwrap();
    let outcomes = [
        ReferenceMeasureOutcome {
            measure: three_halves.clone(),
            probability: half.clone(),
        },
        ReferenceMeasureOutcome {
            measure: three_halves.clone(),
            probability: half,
        },
    ];
    let mass = fsl_definitional_reference::measure_mass_at_most(
        &outcomes,
        &Rational::from_integer(Integer::one()),
    )
    .expect("the two literal path weights sum exactly to one");
    assert_eq!(mass, Rational::from_integer(Integer::zero()));
    // For first delay t in (0,1), lost gives M=2-t and not-lost gives
    // M=t+1, so both exceed 1. The CDF is 0<0.4 and the claim is false.
}

/// QSL-3478's trapped phase has no transition to phase 2; its checked
/// next-state clause yields a closed component containing only phase 1.
#[trace("FR-1051-AC-17")]
#[test]
fn qsl_3478_reference_finds_no_path_from_the_trapped_phase() {
    use filament_exact::{
        Integer, NodeKey, ObjectId, ObjectReference, PathSegment, UniverseId, Value,
    };

    let mut model = crate::prob_world::coin("");
    model.operations[0].name = "step";
    model.operations[0].parameters.clear();
    model.operations[0].modifies = vec!["v"];
    model.operations[0].deletes = false;
    model.body = "bound B on M::coins [1]\n\
                  post Stay using ev on M::Coin::step { self.v = pre(self.v) }\n"
        .to_owned();
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the seed's finite operation relation is checked");
    let graph = world.package.graph();
    let clause = graph
        .state_clause("Stay")
        .expect("the stuttering post is checked");
    let receiver = Value::Reference(ObjectReference::new(
        UniverseId::new(
            NodeKey::new("test/coin", vec![PathSegment::Text("Coin".to_owned())], 0)
                .expect("the seed receiver identity is valid"),
        ),
        clause.context(),
        ObjectId::new("c").expect("the seed object id is non-empty"),
    ));
    let states = fsl_definitional_reference::ReferenceFiniteState::enumerate(vec![(
        "c".to_owned(),
        "v".to_owned(),
        vec![
            Value::Integer(Integer::zero()),
            Value::Integer(Integer::one()),
        ],
    )])
    .expect("phase is a bounded two-value field");
    let transition = fsl_definitional_reference::ReferenceTransitionGraph::from_operation(
        graph,
        "step",
        &[receiver],
        states,
    )
    .expect("the reference interprets the checked next-state clause");
    assert!(!transition
        .can_reach(0, &[false, true])
        .expect("the target set matches the finite state space"));
    // Hand derivation: Stay preserves phase 1, so the reachable component has
    // no state with phase >= 2; both existential reachability claims are false.
}

#[trace("FR-1051-AC-17")]
#[test]
fn reference_interprets_checked_random_support_probability_and_reward() {
    use filament_exact::{
        Integer, NodeKey, ObjectId, ObjectReference, PathSegment, UniverseId, Value,
    };

    let mut model = crate::prob_world::coin("");
    model.body.push_str(
        "\npost DrawOne using ev on M::Coin::drop { b = 1 }\n\
             reward Benefit on Coin::drop = 3;\n",
    );
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the finite random support is checked");
    let graph = world.package.graph();
    let clause = graph
        .state_clause("DrawOne")
        .expect("the random outcome predicate is checked");
    let reward = graph
        .probabilistic_model()
        .rewards()
        .iter()
        .find(|reward| reward.name() == "Benefit")
        .expect("the operation reward expression is checked");
    let receiver = Value::Reference(ObjectReference::new(
        UniverseId::new(
            NodeKey::new("test/coin", vec![PathSegment::Text("Coin".to_owned())], 0)
                .expect("the seed receiver identity is valid"),
        ),
        clause.context(),
        ObjectId::new("c").expect("the seed object id is non-empty"),
    ));
    let state = fsl_definitional_reference::ReferenceFiniteState::new([(
        "c".to_owned(),
        "v".to_owned(),
        Value::Integer(Integer::zero()),
    )])
    .expect("the finite initial state is admitted");
    let random = graph
        .probabilistic_model()
        .random_parameters()
        .iter()
        .find(|random| random.operation().name() == "drop" && random.parameter() == "b")
        .expect("the operation parameter has checked support");
    let locals = [receiver, Value::Integer(Integer::zero())];
    let probability = fsl_definitional_reference::checked_support_probability(
        graph,
        random,
        1,
        &locals,
        &state,
        &state,
        clause.body(),
    )
    .expect("the reference evaluates each checked support value");
    let expected_reward = fsl_definitional_reference::checked_support_expected_reward(
        graph,
        random,
        0,
        &[Value::Integer(Integer::zero())],
        &state,
        &state,
        reward.expression(),
    )
    .expect("the reference evaluates the checked integer reward");
    let half = filament_exact::Rational::new(Integer::one(), Integer::from(2_i64)).unwrap();
    assert_eq!(probability, half);
    assert_eq!(
        expected_reward,
        filament_exact::Rational::from_integer(Integer::from(3_i64))
    );
}

/// QSL-3414's value-bounds route has initial value 0 and reaches a tight
/// activation. On this finite checked fragment, each of the two literal
/// supports has mass 1/2, so one-step activation has exact value 1/2.
#[trace("FR-1051-AC-17")]
#[test]
fn qsl_3414_reference_builds_the_one_step_mdp_from_checked_support() {
    use filament_exact::{
        Integer, NodeKey, ObjectId, ObjectReference, PathSegment, UniverseId, Value,
    };

    let mut model = crate::prob_world::tossed_coin("");
    model.operations[0].deletes = false;
    model.operations[0].modifies = vec!["v"];
    model.body = "bound B on M::coins [1]\n\
                  post Activate using ev on M::Coin::toss { self.v = b }\n\
                  random Coin::toss(b) ~ { 0: 1, 1: 1 };\n"
        .to_owned();
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the finite bounded operation and literal support are checked");
    let graph = world.package.graph();
    let clause = graph
        .state_clause("Activate")
        .expect("the activation relation is checked");
    let receiver = Value::Reference(ObjectReference::new(
        UniverseId::new(
            NodeKey::new("test/tossed", vec![PathSegment::Text("Coin".to_owned())], 0)
                .expect("the seed receiver identity is valid"),
        ),
        clause.context(),
        ObjectId::new("c").expect("the seed object id is non-empty"),
    ));
    let locals = [receiver, Value::Integer(Integer::zero())];
    let states = fsl_definitional_reference::ReferenceFiniteState::enumerate(vec![(
        "c".to_owned(),
        "v".to_owned(),
        vec![
            Value::Integer(Integer::zero()),
            Value::Integer(Integer::one()),
        ],
    )])
    .expect("the bounded field has two enumerated values");
    let mdp = fsl_definitional_reference::checked_random_operation_mdp(
        graph, "toss", "b", &locals, states,
    )
    .expect("the checked literal support determines each unique post-state");
    let value = mdp
        .some_scheduler_reachability_within(0, &[false, true], 1)
        .expect("one-step finite path enumeration is exact");
    assert_eq!(
        value,
        filament_exact::Rational::new(Integer::one(), Integer::from(2_i64)).unwrap()
    );
    // Hand derivation: only b=1 activates the target, with weight 1 of total
    // weight 2; b=0 keeps the initial value 0.
}

/// The same hand-valued activation is sent through the analyzer route: a
/// one-step success mass of 1/2 cannot prove a lower bound of 3/4.
#[trace("FR-1051-AC-17")]
#[test]
fn qsl_3414_analysis_does_not_prove_a_bound_above_the_reference_value() {
    use filament_exact::{
        Cancel, Integer, NodeKey, ObjectId, ObjectReference, PathSegment, UniverseId, Value,
    };
    use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
    use fsl_foundation::RequestIndex;
    use fsl_replay::{ModelCheckLimits, TerminalValue};

    let mut model = crate::prob_world::tossed_coin("");
    model.operations[0].deletes = false;
    model.operations[0].modifies = vec!["v"];
    model.body = "bound B on M::coins [1]\n\
                  post Activate using ev on M::Coin::toss { self.v = b }\n\
                  random Coin::toss(b) ~ { 0: 1, 1: 1 };\n\
                  probabilistic FalseBound using ev on M over (c: M::Coin) under every scheduler on origin { probability >= 0.75 [ eventually[0,1] holds(c.v = 1) ] }\n\
                  probabilistic TrueBound using ev on M over (c: M::Coin) under every scheduler on origin { probability >= 0.5 [ eventually[0,1] holds(c.v = 1) ] }\n"
        .to_owned();
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the finite activation model and false bound are checked");
    let graph = world.package.graph();
    let clause = graph
        .state_clause("Activate")
        .expect("the activation clause is checked");
    let receiver = Value::Reference(ObjectReference::new(
        UniverseId::new(
            NodeKey::new("test/tossed", vec![PathSegment::Text("Coin".to_owned())], 0)
                .expect("the fixture identity is valid"),
        ),
        clause.context(),
        ObjectId::new("c").expect("the fixture object id is valid"),
    ));
    let state = fsl_definitional_reference::ReferenceFiniteState::new([(
        "c".to_owned(),
        "v".to_owned(),
        Value::Integer(Integer::zero()),
    )])
    .expect("the initial value is zero");
    let random = graph
        .probabilistic_model()
        .random_parameters()
        .iter()
        .find(|random| random.operation().name() == "toss" && random.parameter() == "b")
        .expect("the checked operation has its finite support");
    let hand_value = fsl_definitional_reference::checked_support_probability(
        graph,
        random,
        1,
        &[receiver, Value::Integer(Integer::zero())],
        &state,
        &state,
        clause.body(),
    )
    .expect("the reference evaluates both support outcomes");
    assert_eq!(
        hand_value,
        filament_exact::Rational::new(Integer::one(), Integer::from(2_i64)).unwrap()
    );

    let engines = Engines::fsl().expect("the analyzer engines are available");
    let item = exact_probabilistic_item(
        RequestIndex::new(0),
        world.request("FalseBound", ModelCheckLimits::default()),
    );
    let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
    assert_eq!(
        record.terminal().expect("the route settles").value(),
        &TerminalValue::Refuted,
        "the hand value 1/2 is below the claimed lower bound 3/4"
    );

    // Hand derivation: b has literal weights 1 and 1, exactly one of the two
    // outcomes sets v=1, so the one-step probability is 1/(1+1)=1/2.
}

#[trace("FR-1051-AC-17")]
#[test]
fn reference_interprets_checked_clock_guards_invariants_and_zero_resets() {
    use filament_exact::{Integer, Rational};

    let document = crate::timed_world::document(&["x"], &[("tick", &["x"])]);
    let body = format!(
        "{}pre Ready using v on M::Call::tick {{ self.x >= 1 ms }}\n\
         post Reset using v on M::Call::tick {{ self.x = 0 ms }}\n\
         time invariant Cap when {{ true }} {{ self.x <= 2 ms }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world(
        &document,
        "profile ev = \"filament.temporal.timed/v1\";\n",
        &body,
    );
    let timed = &world.package.graph().timed_models()[0];
    let clock = |numerator: i64| {
        Rational::new(Integer::from(numerator), Integer::from(2_i64))
            .expect("positive rational clock value")
    };
    let state = fsl_definitional_reference::ReferenceClockState::new(timed, vec![clock(3)])
        .expect("one clock at 3/2 ms");
    assert!(state
        .satisfies_guard(timed, 0)
        .expect("the checked guard is a single clock comparison"));
    assert!(state
        .satisfies_invariant(timed, 0)
        .expect("the checked invariant is clock-only with a constant predicate"));
    assert_eq!(
        state
            .apply_zero_resets(timed, 0)
            .expect("the checked postcondition resets x to zero")
            .values()[0],
        Rational::from_integer(Integer::zero())
    );
    let delayed = state.delay(&clock(2)).expect("one unit of dense time");
    assert!(!delayed
        .satisfies_invariant(timed, 0)
        .expect("the invariant comparison remains in the checked fragment"));
}

fn direct_manifest_dependencies(manifest: &str) -> Vec<(String, String)> {
    let mut dependencies = Vec::new();
    let mut dependency_table: Option<String> = None;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            dependency_table = match line {
                "[dependencies]" | "[build-dependencies]" | "[dev-dependencies]" => {
                    Some(line.trim_matches(['[', ']']).to_owned())
                }
                _ if line.starts_with("[dependencies.") => {
                    let name = line
                        .strip_prefix("[dependencies.")
                        .and_then(|tail| tail.strip_suffix(']'))
                        .unwrap_or_default();
                    dependencies.push((
                        name.trim_matches(['"', '\'']).to_owned(),
                        "dependencies".to_owned(),
                    ));
                    None
                }
                _ if line.starts_with("[target.") => [
                    ".dependencies]",
                    ".build-dependencies]",
                    ".dev-dependencies]",
                ]
                .iter()
                .find(|suffix| line.ends_with(**suffix))
                .map(|suffix| suffix.trim_matches(['.', ']']).to_owned()),
                _ => None,
            };
            continue;
        }
        let Some(table) = &dependency_table else {
            continue;
        };
        if line.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            dependencies.push((
                name.trim().trim_matches(['"', '\'']).to_owned(),
                table.clone(),
            ));
        }
    }
    dependencies
}

/// The oracle stays isolated from producer and checker crates and is linked
/// only into tests. Runtime checks are added alongside each route's seed.
#[trace("FR-1051-AC-18")]
#[test]
fn reference_crate_has_only_the_declared_semantic_dependencies() {
    let manifest = include_str!("../../../fsl-definitional-reference/Cargo.toml");
    let dependencies = direct_manifest_dependencies(manifest);
    let allowed = [
        "fsl-foundation",
        "fsl-semantics",
        "fsl-package",
        "filament-exact",
        "filament-checked",
        "filament-contract-model",
        "filament-semantic-value",
        "ix-trace-rs",
    ];
    for (name, _) in &dependencies {
        assert!(
            allowed.contains(&name.as_str()),
            "unexpected direct dependency {name}"
        );
    }
    for required in [
        "fsl-foundation =",
        "fsl-semantics =",
        "fsl-package =",
        "filament-exact =",
        "filament-checked =",
        "filament-contract-model =",
        "filament-semantic-value =",
    ] {
        assert!(
            dependencies
                .iter()
                .any(|(name, _)| name == required.trim_end_matches(" =")),
            "reference dependency section omits required crate {required}"
        );
    }
    for forbidden in [
        "fsl-eval",
        "fsl-analyze",
        "fsl-replay",
        "fsl-hybrid-reference",
    ] {
        assert!(
            !dependencies.iter().any(|(name, _)| name == forbidden),
            "reference dependency section names forbidden crate {forbidden}"
        );
    }
    let reference_source = include_str!("../../../fsl-definitional-reference/src/lib.rs");
    assert!(reference_source.contains("impl RewardEvaluator for ReferenceRewardEvaluator"));
    assert!(!reference_source.contains("S6aRewardEvaluator"));

    let analyze_manifest = include_str!("../../../fsl-analyze/Cargo.toml");
    let analyze_dependencies = direct_manifest_dependencies(analyze_manifest);
    assert_eq!(
        analyze_dependencies
            .iter()
            .filter(|(name, _)| name == "fsl-definitional-reference")
            .count(),
        1,
        "the reference is linked only once"
    );
    assert_eq!(
        analyze_dependencies
            .iter()
            .find(|(name, _)| name == "fsl-definitional-reference")
            .map(|(_, table)| table.as_str()),
        Some("dev-dependencies"),
        "the reference is linked from analyzer test dependencies only"
    );
}

/// The manifest reader includes dependency tables after lints and target
/// dependency tables, so later sections cannot hide a forbidden edge.
#[trace("FR-1051-AC-18")]
#[test]
fn manifest_dependency_reader_covers_later_and_target_tables() {
    let manifest = "[dependencies]\nfsl-semantics = \"0.1\"\n\
         [lints.rust]\nunsafe_code = \"forbid\"\n\
         [dependencies.fsl-eval]\npath = \"../fsl-eval\"\n\
         [target.'cfg(unix)'.build-dependencies]\nfsl-replay = \"0.1\"\n";
    let dependencies = direct_manifest_dependencies(manifest);
    assert!(dependencies.iter().any(|(name, _)| name == "fsl-eval"));
    assert!(dependencies.iter().any(|(name, _)| name == "fsl-replay"));
    assert!(dependencies.iter().any(|(name, _)| name == "fsl-semantics"));
}

/// FR-1051-AC-19: a disjunctive guard is refused before the reference can
/// return a truth value. The guard is read from the checked operation graph.
#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_disjunctive_operation_guards() {
    let mut model = crate::prob_world::coin("");
    model
        .body
        .push_str("\npre Disjunctive using v on M::Coin::drop { self.v = 0 or self.v = 1 }\n");
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the model with a disjunctive guard passes front-end checking");

    let refusal = fsl_definitional_reference::validate_conjunctive_guards(world.package.graph())
        .expect_err("the tiny reference refuses disjunctive guards");
    assert_eq!(refusal.construct, "disjunctive operation guard");
}

/// A multi-part timed guard carries disjunction outside ordinary preconditions.
#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_disjunctive_timed_guards() {
    const PROFILES: &str = "profile ev = \"filament.temporal.timed/v1\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n";
    let document = crate::timed_world::document(&["x"], &[("send", &["phase"])]);
    let body = format!(
        "{}pre Disjunctive using v on M::Call::send {{ self.x > 0 ms or self.x < 2 ms }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world(&document, PROFILES, &body);

    let refusal = fsl_definitional_reference::validate_conjunctive_guards(world.package.graph())
        .expect_err("the reference refuses a timed disjunction");
    assert_eq!(refusal.construct, "disjunctive timed guard");
}

/// A checked workload names one fixed scheduler, not existential scheduling.
/// The reference refuses this unsupported quantile route by its actual kind.
#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_quantiles_under_unsupported_declared_workload() {
    let model = crate::prob_world::tossed_coin(
        "probabilistic Q using ev on M over (c: M::Coin) under Even on origin \
         { quantile 0.5 of (steps from holds(c.v = 0) until holds(c.v = 1) within 2) <= 1 }",
    );
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the checked package admits the quantile claim");

    let refusal = fsl_definitional_reference::validate_tiny_fragment(world.package.graph())
        .expect_err("the named workload quantile solver is unsupported");
    assert_eq!(
        refusal.construct,
        "quantile over an unsupported declared workload"
    );
}

/// FR-1051-AC-19: region constraints in the admitted reference fragment use
/// integer constants, even though the exact clock valuation is rational.
#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_non_integer_clock_constants() {
    const PROFILES: &str = "profile ev = \"filament.temporal.timed/v1\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n";
    let document = crate::timed_world::document(&["x"], &[("send", &["phase"])]);
    let body = format!(
        "{}pre Half using v on M::Call::send {{ self.x > rational(1, 2) ms }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world(&document, PROFILES, &body);

    let refusal = fsl_definitional_reference::validate_tiny_fragment(world.package.graph())
        .expect_err("a half-unit clock boundary is outside the region fragment");
    assert_eq!(
        refusal.construct,
        "clock compared with a non-integer constant"
    );
}

/// FR-1051-AC-19: an unbounded native integer has infinitely many valuations.
#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_unbounded_integer_fields() {
    let mut model = crate::prob_world::coin("");
    model.fields[0].1 = "ix://filament/native/Integer".to_owned();
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the front end admits an unbounded integer field");

    let refusal = fsl_definitional_reference::validate_tiny_fragment(world.package.graph())
        .expect_err("the reference does not enumerate an unbounded integer field");
    assert_eq!(refusal.construct, "unbounded integer field");
}

/// FR-1051-AC-19: hybrid dynamics in the reference fragment use literal
/// rational rates rather than state-dependent flow expressions.
#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_state_dependent_hybrid_rates() {
    let source = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid H using p { mode A invariant { x >= 0 } flow { x' = x; } }\n";
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("false-claim-corpus", "hybrid-rate"),
        "unit.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the source is within parser limits");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("the source builds as a unit");
    let package = fsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "false-claim-corpus".to_owned(),
        unit,
        fsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the hybrid unit assembles");
    let graph = package
        .check(
            filament_semantic_value::checking::CheckMode::Linked,
            filament_semantic_value::checking::CheckingLimits::default(),
            &fsl_definitional_reference::ReferenceRewardEvaluator,
        )
        .expect("the state-dependent rate passes package checking");

    let refusal = fsl_definitional_reference::validate_tiny_fragment(&graph)
        .expect_err("the reference refuses a state-dependent hybrid rate");
    assert_eq!(refusal.construct, "hybrid mode with a non-constant rate");
}
