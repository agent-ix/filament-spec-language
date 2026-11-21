// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-187 (TC-622): `actions`, `weighted_steps`, `NotMarkov`, the negative
//! reward refusal and the support graph, over the fixtures of
//! [`crate::simulation::sampled::world`].

use std::collections::{BTreeSet, VecDeque};

use filament_exact::{Integer, Rational, Value};
use fsl_foundation::diagnostic::Code;
use fsl_semantics::model::observation::AdmissionFailure;
use ix_trace_rs::trace;

use super::{NotMarkov, TransitionId, Unsteppable, WeightedStep};
use crate::simulation::model::ModelSimState;
use crate::simulation::sampled::world::{self, field, initial, World};
use crate::simulation::TransitionSystem;

#[trace("TC-636", "FR-201-AC-22", "FR-202-AC-22", "FR-282-AC-17")]
#[test]
fn caller_cancellation_enters_qualified_contract_and_state_predicate_meters() {
    use super::super::PredicateReading;
    use filament_exact::{Cancel, CancelCause};
    use fsl_semantics::check::{ProbForm, ReachTarget};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};
    let mut model = world::link();
    model.body.push_str("invariant Valid using v on M::Msg at current { self.attempts >= 0 }\nprobabilistic Check using inf on M over (m: M::Msg) under every scheduler on origin { probability >= 0.5 [ eventually holds(m.delivered) ] }\n");
    let world = World::with_profiles(&model, &[("inf", "filament.temporal.infinite-trace/v1")]);
    let system = world.system();
    let start = initial(&system);
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("Check")
        .unwrap();
    let ProbForm::Reach {
        target: ReachTarget::Eventually(predicate),
        ..
    } = &claim.form
    else {
        panic!("a checked eventually state predicate");
    };
    let walk = super::super::walk::walk_of(&system, &start, None).unwrap();
    let binding = &walk.rows.first().unwrap().reference;
    assert_eq!(
        system
            .state_predicates(&start, &[(predicate, Some(binding))])
            .unwrap(),
        vec![PredicateReading::False]
    );
    assert_eq!(system.actions(&start).unwrap().unwrap().len(), 2);
    assert_eq!(
        system
            .weighted_steps(&start, world.workload("Even"))
            .unwrap()
            .unwrap()
            .len(),
        4
    );
    // Invariants run before candidate generation: the observer is entered
    // by a qualified invariant/predicate meter, with no poll offset oracle.
    for (mode, cause) in [
        ("actions", CancelCause::Requested),
        ("workload", CancelCause::Deadline),
        ("predicate", CancelCause::Requested),
    ] {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let calls = Arc::new(AtomicU32::new(0));
        let seen = Arc::clone(&calls);
        let cancel = Cancel::observing(move || {
            seen.fetch_add(1, Ordering::Relaxed);
            weak.upgrade().unwrap().get().unwrap().cancel(cause);
        });
        slot.set(cancel.clone()).unwrap();
        let stopped = match mode {
            "actions" => system.actions_with_cancel(&start, &cancel).is_err(),
            "workload" => system
                .weighted_steps_with_cancel(&start, world.workload("Even"), &cancel)
                .is_err(),
            "predicate" => system
                .state_predicates_with_cancel(&start, &[(predicate, Some(binding))], &cancel)
                .is_err(),
            _ => unreachable!(),
        };
        assert!(stopped, "{mode} must stop inside qualified evaluation");
        assert_eq!(cancel.tripped(), Some(cause));
        assert!(calls.load(Ordering::Relaxed) > 0);
    }
    assert_eq!(system.actions(&start).unwrap().unwrap().len(), 2);
}

/// The identity text of `system`'s operation `name`.
fn operation_text(system: &super::ModelSystem<'_>, name: &str) -> String {
    system
        .operations()
        .into_iter()
        .find(|(_, operation, _)| *operation == name)
        .map(|(_, _, text)| text.to_owned())
        .unwrap_or_else(|| panic!("no operation {name}"))
}

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn integer_of(value: &Value) -> i64 {
    match value {
        Value::Integer(integer) => integer.to_string().parse().expect("small"),
        other => panic!("not an integer: {other:?}"),
    }
}

fn boolean_of(value: &Value) -> bool {
    match value {
        Value::Boolean(flag) => *flag,
        other => panic!("not a Boolean: {other:?}"),
    }
}

fn steps(
    system: &super::ModelSystem<'_>,
    state: &ModelSimState,
    world: &World,
    workload: &str,
) -> Vec<WeightedStep> {
    system
        .weighted_steps(state, world.workload(workload))
        .expect("the state expands")
        .expect("the state is Markov")
}

fn sum(steps: &[WeightedStep]) -> Rational {
    steps
        .iter()
        .fold(Rational::from_integer(Integer::zero()), |total, step| {
            total.add(&step.probability)
        })
}

/// The one reward value of a step.
fn duration(step: &WeightedStep) -> i64 {
    let [(name, value)] = &step.rewards[..] else {
        panic!("one reward: {:?}", step.rewards);
    };
    assert_eq!(name.as_str(), "duration");
    integer_of(value)
}

/// The transition's argument texts.
fn arguments(step: &WeightedStep) -> Vec<String> {
    match &step.transition {
        TransitionId::Operation(transition) => transition.arguments().to_vec(),
        TransitionId::Stutter => panic!("a stutter step"),
    }
}

/// FR-187-AC-1 (TC-622 step 1): `Service` (`Phase` as `Int[0, 2]`,
/// `outcome` a `Boolean`, `d` typed `Millis`) at `phase = 1, attempts = 0`
/// under `Steady` has six `attempt` steps whose probabilities are the
/// products of `d`'s and `outcome`'s, summing to 1, with `duration = d`; at
/// `phase = 0` the only step is `request`, probability 1, `duration` 0.
#[trace("TC-622", "FR-187-AC-1")]
#[test]
fn service_steps_carry_the_products_of_their_draws() {
    let world = World::new(&world::service(""));
    let system = world.system();
    let idle = initial(&system);
    let [request] = <[_; 1]>::try_from(steps(&system, &idle, &world, "Steady")).expect("one step");
    assert_eq!(request.probability, ratio(1, 1));
    assert_eq!(duration(&request), 0);
    assert_eq!(integer_of(&field(&request.post, "phase")), 1);
    assert_eq!(integer_of(&field(&request.post, "attempts")), 0);

    let busy = steps(&system, &request.post, &world, "Steady");
    assert_eq!(busy.len(), 6);
    assert_eq!(sum(&busy), ratio(1, 1));
    let expected = [
        (1, true, ratio(441, 500)),
        (1, false, ratio(9, 500)),
        (3, true, ratio(343, 5000)),
        (3, false, ratio(7, 5000)),
        (12, true, ratio(147, 5000)),
        (12, false, ratio(3, 5000)),
    ];
    for (d, ok, probability) in expected {
        let texts = [
            crate::simulation::model::frames::text_of(&int(d), system.typing()).expect("text"),
            crate::simulation::model::frames::text_of(&Value::Boolean(ok), system.typing())
                .expect("text"),
        ];
        let step = busy
            .iter()
            .find(|step| arguments(step) == texts)
            .unwrap_or_else(|| panic!("a step with d = {d}, ok = {ok}"));
        assert_eq!(step.probability, probability, "d = {d}, ok = {ok}");
        assert_eq!(duration(step), d);
        let phase = integer_of(&field(&step.post, "phase"));
        assert_eq!(phase, if ok { 2 } else { 1 });
    }
}

/// FR-187-AC-2 (TC-622 step 2): `Link` under `Even` at its initial state
/// gives `send_a` delivered `9/20`, lost `1/20`, `send_b` delivered `2/5`,
/// lost `1/10`; `actions` gives two actions in canonical order whose draws
/// each sum to 1; at a delivered state the only step is the stutter step.
#[trace("TC-622", "FR-187-AC-2")]
#[test]
fn link_steps_weigh_operations_then_draws() {
    let world = World::new(&world::link());
    let system = world.system();
    let start = initial(&system);
    let weighted = steps(&system, &start, &world, "Even");
    assert_eq!(weighted.len(), 4);
    assert_eq!(sum(&weighted), ratio(1, 1));
    for step in &weighted {
        let TransitionId::Operation(transition) = &step.transition else {
            panic!("an operation step");
        };
        let identity = step
            .identity
            .as_ref()
            .expect("the step's scheduled identity");
        assert_eq!(identity.operation, transition.operation());
        assert!(identity.arguments.is_empty());
        let [lost] = &step.random[..] else {
            panic!("one drawn value: {:?}", step.random);
        };
        assert_eq!(
            boolean_of(lost),
            !boolean_of(&field(&step.post, "delivered"))
        );
    }
    let probability = |operation: &str, lost: bool| {
        weighted
            .iter()
            .find(|step| {
                let TransitionId::Operation(transition) = &step.transition else {
                    return false;
                };
                transition.operation() == operation_text(&system, operation).as_str()
                    && boolean_of(&field(&step.post, "delivered")) != lost
            })
            .map(|step| step.probability.clone())
            .unwrap_or_else(|| panic!("{operation} lost = {lost}"))
    };
    assert_eq!(probability("send_a", false), ratio(9, 20));
    assert_eq!(probability("send_a", true), ratio(1, 20));
    assert_eq!(probability("send_b", false), ratio(2, 5));
    assert_eq!(probability("send_b", true), ratio(1, 10));

    let actions = system
        .actions(&start)
        .expect("the state expands")
        .expect("decided");
    assert_eq!(actions.len(), 2);
    let operations: Vec<String> = actions
        .iter()
        .map(|action| action.identity.operation.clone())
        .collect();
    let mut ordered = vec![
        operation_text(&system, "send_a"),
        operation_text(&system, "send_b"),
    ];
    ordered.sort_by_key(|text| {
        super::identity_bytes(text, &actions[0].identity.receiver, &[], system.typing())
            .expect("bytes")
    });
    assert_eq!(operations, ordered);
    for action in &actions {
        assert!(action.identity.arguments.is_empty());
        assert_eq!(action.draws.len(), 2);
        let total = action
            .draws
            .iter()
            .fold(Rational::from_integer(Integer::zero()), |total, draw| {
                total.add(&draw.probability)
            });
        assert_eq!(total, ratio(1, 1));
        // Canonical order of the drawn vectors: `false` before `true`.
        let drawn: Vec<bool> = action
            .draws
            .iter()
            .map(|draw| boolean_of(&draw.random[0]))
            .collect();
        assert_eq!(drawn, [false, true]);
    }

    let delivered = weighted
        .iter()
        .find(|step| boolean_of(&field(&step.post, "delivered")))
        .expect("a delivering step");
    let [stutter] =
        <[_; 1]>::try_from(steps(&system, &delivered.post, &world, "Even")).expect("one step");
    assert_eq!(stutter.transition, TransitionId::Stutter);
    assert!(stutter.identity.is_none());
    assert!(stutter.random.is_empty());
    assert_eq!(stutter.probability, ratio(1, 1));
    assert!(stutter.rewards.is_empty());
}

/// FR-187-AC-3 (TC-622 step 3): a `Health` whose `tick` leaves `spare`
/// unconstrained is not Markov under `Steady`: `weighted_steps` names the
/// state key, `tick`, the first drawn `fault` (`false`) and 2 post-states,
/// and `actions` gives one action whose draws each carry two successors. A
/// `Health` with no post-state for `fault = true` gives `NotMarkov` with 0
/// post-states, and its action's `true` draw carries none.
#[trace("TC-622", "FR-187-AC-3")]
#[test]
fn health_variants_are_not_markov() {
    let world = World::new(&world::loose_health());
    let system = world.system();
    let start = initial(&system);
    let state_key = system.canonical_state_key(&start).expect("a key");
    let not_markov = not_markov_of(
        system
            .weighted_steps(&start, world.workload("Steady"))
            .expect("the state expands"),
        "two post-states per draw",
    );
    assert_eq!(not_markov.state, state_key);
    assert_eq!(
        not_markov.transition.operation,
        operation_text(&system, "tick")
    );
    assert!(not_markov.transition.arguments.is_empty());
    assert!(super::same_values(
        &not_markov.random,
        &[Value::Boolean(false)]
    ));
    assert_eq!(not_markov.post_states, 2);
    let [action] = <[_; 1]>::try_from(system.actions(&start).expect("expands").expect("decided"))
        .expect("one");
    assert_eq!(action.draws.len(), 2);
    assert!(action.draws.iter().all(|draw| draw.successors.len() == 2));

    let world = World::new(&world::blocked_health());
    let system = world.system();
    let start = initial(&system);
    let not_markov = not_markov_of(
        system
            .weighted_steps(&start, world.workload("Steady"))
            .expect("the state expands"),
        "no post-state for fault = true",
    );
    assert!(super::same_values(
        &not_markov.random,
        &[Value::Boolean(true)]
    ));
    assert_eq!(not_markov.post_states, 0);
    let [action] = <[_; 1]>::try_from(system.actions(&start).expect("expands").expect("decided"))
        .expect("one");
    let counts: Vec<(bool, usize)> = action
        .draws
        .iter()
        .map(|draw| (boolean_of(&draw.random[0]), draw.successors.len()))
        .collect();
    assert_eq!(counts, [(false, 1), (true, 0)]);
}

/// FR-187-AC-4 (TC-622 step 4): `refund = n - 3` over `n: Int[0, 5]`
/// refuses admission `invalid_probabilistic_model`/`negative-reward`,
/// naming the operation, the reward and `n = 0`.
#[trace("FSpec-TC-1561", "FSpec-FR-411-AC-17")]
#[trace("TC-622", "FR-187-AC-4")]
#[test]
fn a_negative_reward_refuses_admission() {
    let model = world::refund();
    let world = World::with_s3_reward_signs_unchecked(&model);
    let Err(AdmissionFailure::Refused(record)) = world.admit() else {
        panic!("admission refuses");
    };
    assert_eq!(
        (record.catalog.code(), record.catalog.cause()),
        (Code::InvalidProbabilisticModel.as_str(), "negative-reward")
    );
    assert_eq!(
        record.fields.get("reward").map(String::as_str),
        Some("refund")
    );
    assert_eq!(
        record.fields.get("operation"),
        Some(&model.ident("Account/pay"))
    );
    assert_eq!(
        record.fields.get("arguments").map(String::as_str),
        Some("n = 0")
    );
}

/// FR-187-AC-4 (TC-622 step 4): over every state of `Service`, the
/// transitions `weighted_steps` gives positive probability under `Steady`
/// are exactly FR-120's successors (ADR-024 PM-6). Unlike FR-187-AC-1, this
/// clause does not depend on `Service`'s value types: PM-6 compares
/// transition sets.
#[trace("TC-622", "FR-187-AC-4")]
#[test]
fn the_support_graph_is_the_state_graph() {
    let world = World::new(&world::service(""));
    let system = world.system();
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([initial(&system)]);
    while let Some(state) = queue.pop_front() {
        let (key, _) = system.state_key(&state).expect("a key");
        if !seen.insert(key) {
            continue;
        }
        let successors: BTreeSet<(Vec<u8>, Vec<u8>)> = system
            .successors(&state)
            .expect("expands")
            .successors
            .iter()
            .map(|(transition, post)| {
                (
                    system.transition_key(transition).expect("bytes"),
                    system.state_key(post).expect("key").0,
                )
            })
            .collect();
        let weighted = steps(&system, &state, &world, "Steady");
        let positive: BTreeSet<(Vec<u8>, Vec<u8>)> = weighted
            .iter()
            .filter(|step| !step.probability.is_zero())
            .map(|step| {
                let TransitionId::Operation(transition) = &step.transition else {
                    panic!("Service has no terminal state");
                };
                (
                    system.transition_key(transition).expect("bytes"),
                    system.state_key(&step.post).expect("key").0,
                )
            })
            .collect();
        assert_eq!(positive, successors);
        assert_eq!(positive.len(), weighted.len());
        queue.extend(weighted.into_iter().map(|step| step.post));
    }
    // Idle at 0, 1 and 2 attempts (`reset` keeps them), Busy at 0 and 1,
    // and Done at 1 and 2.
    assert_eq!(seen.len(), 7);
}

/// FR-185-AC-7 (TC-620 step 7): `Service`, with `d` typed `Millis = Int[1,
/// 12]`, checks at S3 with its record intact; S4 writes its random
/// parameters, workload and reward, and refuses at `unsupported_construct`/
/// `not-yet-implemented`, naming its Boolean random parameter `ok` (IR-1569),
/// `random-parameter` and the selected profile.
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
fn service_checks_and_refuses_emission_naming_its_boolean_random_parameter() {
    let world = World::new(&world::service(""));
    let model = world.package.graph().probabilistic_model();
    assert_eq!(model.random_parameters().len(), 2);
    assert_eq!(model.rewards().len(), 1);
    assert_eq!(model.workloads().len(), 1);
    let refusal = fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
        .expect_err("S4 refuses");
    assert!(matches!(
        &refusal,
        fsl_package::EmitRefusal::ProbabilisticModelNotEmitted {
            declaration,
            feature: fsl_semantics::check::ProbabilisticFeature::RandomParameter,
            profile: Some("filament.model.complete/v1"),
            region,
        } if declaration == "attempt(ok)" && region.start() < region.end()
    ));
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
}

/// The [`NotMarkov`] of a `weighted_steps` result that is not steppable.
fn not_markov_of(result: Result<Vec<WeightedStep>, Unsteppable>, why: &str) -> NotMarkov {
    match result {
        Err(Unsteppable::NotMarkov(not_markov)) => not_markov,
        other => panic!("NotMarkov ({why}): {other:?}"),
    }
}

/// FR-187 Behavior: an enabled scheduled identity with no post-state for
/// any draw makes `weighted_steps` return `NotMarkov` with 0 post-states
/// for the first draw (`x = false`), not the stutter step of a terminal
/// state; `actions` leaves the identity out.
#[trace("TC-622")]
#[test]
fn an_identity_with_no_post_state_for_any_draw_is_not_markov() {
    let world = World::new(&world::gate("", "self.up = x and self.up = (not x)", ""));
    let system = world.system();
    let start = initial(&system);
    let state_key = system.canonical_state_key(&start).expect("a key");
    let not_markov = not_markov_of(
        system
            .weighted_steps(&start, world.workload("Unit"))
            .expect("the state expands"),
        "no post-state for any draw",
    );
    assert_eq!(not_markov.state, state_key);
    assert_eq!(
        not_markov.transition.operation,
        operation_text(&system, "step")
    );
    assert!(super::same_values(
        &not_markov.random,
        &[Value::Boolean(false)]
    ));
    assert_eq!(not_markov.post_states, 0);
    assert!(system
        .actions(&start)
        .expect("expands")
        .expect("decided")
        .is_empty());
}

/// FR-187 Behavior: the stutter step of a terminal state carries every
/// reward of the model at 0, here `cost` on a state where `step` is not
/// enabled.
#[trace("TC-622")]
#[test]
fn the_stutter_step_carries_every_reward_at_zero() {
    let world = World::new(&world::gate(
        "self.up",
        "self.up = x",
        "reward cost on Gate::step = 1;\n",
    ));
    let system = world.system();
    let start = initial(&system);
    let [stutter] = <[_; 1]>::try_from(steps(&system, &start, &world, "Unit")).expect("one");
    assert_eq!(stutter.transition, TransitionId::Stutter);
    assert_eq!(stutter.probability, ratio(1, 1));
    let [(name, value)] = &stutter.rewards[..] else {
        panic!("one reward: {:?}", stutter.rewards);
    };
    assert_eq!(name.as_str(), "cost");
    assert_eq!(integer_of(value), 0);
}

/// FR-187-AC-5 (TC-2064): zero-weight operations are omitted when positive
/// alternatives are enabled, a zero-weight operation chosen alone has total
/// probability 1, and an all-zero workload behaves like equal weights.
/// Over `Link` under `Lean` (`send_b` at 0), the initial state's steps are
/// `send_a`'s alone, with draw probabilities `9/10` and `1/10`. Over `Service`
/// under `Lean`, `reset` has weight 0 and is enabled only on its own; its
/// steps sum to 1. Under `Zero`, every state gives the steps it gives under
/// `Steady`.
#[trace("TC-2064", "FR-187-AC-5")]
#[trace("FSpec-TC-1561", "FSpec-FR-406-AC-7")]
#[test]
fn zero_weight_workloads_follow_enabled_operation_weights() {
    let mut link = world::link();
    link.body
        .push_str("workload Lean on M { weight Msg::send_a = 1; weight Msg::send_b = 0; }\n");
    let world = World::new(&link);
    let system = world.system();
    let start = initial(&system);
    let lean = steps(&system, &start, &world, "Lean");
    assert_eq!(sum(&lean), ratio(1, 1));
    let send_a = operation_text(&system, "send_a");
    let mut probabilities: Vec<Rational> = lean
        .iter()
        .map(|step| {
            let TransitionId::Operation(transition) = &step.transition else {
                panic!("an operation step");
            };
            assert_eq!(transition.operation(), send_a.as_str(), "only send_a");
            step.probability.clone()
        })
        .collect();
    probabilities.sort();
    assert_eq!(probabilities, [ratio(1, 10), ratio(9, 10)]);

    let world = World::new(&world::service(
        "workload Lean on M { weight Server::request = 1; weight Server::attempt = 1; \
         weight Server::reset = 0; }\n",
    ));
    let system = world.system();
    let reset = operation_text(&system, "reset");
    let mut queue = VecDeque::from([initial(&system)]);
    let mut seen = BTreeSet::new();
    let mut reset_enabled_alone = false;
    while let Some(state) = queue.pop_front() {
        if !seen.insert(format!("{state:?}")) {
            continue;
        }
        let actions = system.actions(&state).expect("expands").expect("decided");
        let reset_enabled = actions
            .iter()
            .any(|action| action.identity.operation == reset);
        let other_enabled = actions
            .iter()
            .any(|action| action.identity.operation != reset);
        let lean = steps(&system, &state, &world, "Lean");
        assert_eq!(sum(&lean), ratio(1, 1), "{state:?}");
        if reset_enabled && other_enabled {
            assert!(
                lean.iter().all(|step| !matches!(
                    &step.transition,
                    TransitionId::Operation(transition)
                        if transition.operation() == reset
                )),
                "zero-weight reset is omitted while another operation is enabled"
            );
        }
        if reset_enabled && !other_enabled {
            reset_enabled_alone = true;
            let reset_steps: Vec<_> = lean
                .iter()
                .filter(|step| {
                    matches!(
                        &step.transition,
                        TransitionId::Operation(transition)
                            if transition.operation() == reset
                    )
                })
                .collect();
            assert!(
                !reset_steps.is_empty(),
                "reset is stepped when enabled alone"
            );
            assert_eq!(
                reset_steps
                    .iter()
                    .fold(ratio(0, 1), |total, step| total.add(&step.probability)),
                ratio(1, 1)
            );
        }
        queue.extend(lean.into_iter().map(|step| step.post));
    }
    assert!(reset_enabled_alone, "Service reaches its reset-only state");

    let world = World::new(&world::service(
        "workload Zero on M { weight Server::request = 0; weight Server::attempt = 0; \
         weight Server::reset = 0; }\n",
    ));
    let system = world.system();
    let mut queue = VecDeque::from([initial(&system)]);
    let mut seen = BTreeSet::new();
    while let Some(state) = queue.pop_front() {
        if !seen.insert(format!("{state:?}")) {
            continue;
        }
        let zero = steps(&system, &state, &world, "Zero");
        let steady = steps(&system, &state, &world, "Steady");
        let view = |steps: &[WeightedStep]| -> Vec<(String, Rational)> {
            steps
                .iter()
                .map(|step| (format!("{:?}", step.post), step.probability.clone()))
                .collect()
        };
        assert_eq!(view(&zero), view(&steady), "{state:?}");
        queue.extend(steady.into_iter().map(|step| step.post));
    }
}

/// FR-740-AC-2 (TC-2062), FSpec FR-640-AC-1 and AC-5 (TC-1563): S4 emits
/// `Service` with two `random_parameter` nodes, one `reward` node and one
/// `workload` node, and the step probabilities FR-187-AC-1 states hold for
/// the model that package carries.
#[trace("TC-2062", "FR-740-AC-2")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-5")]
#[test]
fn service_is_emitted_and_its_steps_hold_for_the_emitted_model() {
    let world = World::new(&world::service(""));
    let emission = fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
        .unwrap_or_else(|refusal| panic!("S4 emits Service: {refusal:?}"));
    let wire: serde_json::Value =
        serde_json::from_slice(emission.package().bytes()).expect("JSON bytes");
    let count = |form: &str| {
        wire["semantic_graph"]["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .filter(|node| node["node_tag"] == "probabilistic" && node["semantic_form"] == form)
            .count()
    };
    assert_eq!(count("random_parameter"), 2);
    assert_eq!(count("reward"), 1);
    assert_eq!(count("workload"), 1);

    let system = world.system();
    let idle = initial(&system);
    let [request] = <[_; 1]>::try_from(steps(&system, &idle, &world, "Steady")).expect("one step");
    let busy = steps(&system, &request.post, &world, "Steady");
    assert_eq!(busy.len(), 6);
    assert_eq!(sum(&busy), ratio(1, 1));
}

/// `Service` without its reward, and without the Boolean `ok` random
/// parameter when `boolean` is false.
fn service_without_reward(boolean: bool) -> crate::simulation::sampled::world::Model {
    let mut model = world::service("");
    model.body = model
        .body
        .replace("reward duration on Server::attempt = d;\n", "");
    if !boolean {
        model.body = model
            .body
            .replace("random Server::attempt(ok) ~ { true: 98, false: 2 };\n", "");
    }
    model
}

/// FR-740-AC-1 (TC-2062), FSpec FR-640-AC-1: `Service` without its Boolean
/// parameter emits its `duration` reward, typed `Millis = Int[1, 12]`, as a
/// `reward` node beside the `d` random parameter and the `Steady` workload.
#[trace("TC-2062", "FR-740-AC-1")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-1")]
#[test]
fn service_reward_is_emitted_beside_its_random_parameter_and_workload() {
    let mut model = world::service("");
    model.body = model
        .body
        .replace("random Server::attempt(ok) ~ { true: 98, false: 2 };\n", "");
    let world = World::new(&model);
    let emission = fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
        .unwrap_or_else(|refusal| panic!("S4 emits the reward: {refusal:?}"));
    let wire: serde_json::Value =
        serde_json::from_slice(emission.package().bytes()).expect("JSON bytes");
    let rewards: Vec<&serde_json::Value> = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|node| node["node_tag"] == "probabilistic" && node["semantic_form"] == "reward")
        .collect();
    let [reward] = &rewards[..] else {
        panic!("one reward node: {rewards:?}");
    };
    let member = |term: &serde_json::Value, name: &str| -> serde_json::Value {
        term["members"]
            .as_array()
            .and_then(|members| members.iter().find(|member| member["name"] == name))
            .map(|member| member["value"].clone())
            .unwrap_or_else(|| panic!("binding `{name}` in {term}"))
    };
    assert_eq!(member(&reward["body"], "name")["value"], "duration");
    let node_of = |id: &serde_json::Value| -> serde_json::Value {
        wire["semantic_graph"]["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .find(|node| &node["node_id"] == id)
            .cloned()
            .unwrap_or_else(|| panic!("a node with id {id}"))
    };
    let function = node_of(&member(&reward["body"], "expression")["target"]);
    let returned = node_of(&member(&function["body"], "body")["target"]);
    assert_eq!(
        member(&returned["body"], "name")["value"],
        "d",
        "the function returns its parameter `d`"
    );
    fsl_package::read_evaluation_package(
        &world.package,
        fsl_semantics::library::LibraryName::new("fixture").expect("a library name"),
        world.packages(),
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
    )
    .expect("the reader admits the package with its reward");
}

/// FR-740-AC-1 (TC-2062): `Service` without its reward and its Boolean
/// parameter emits the `Millis`-typed random parameter `d` and the `Steady`
/// workload node.
#[trace("TC-2062", "FR-740-AC-1")]
#[test]
fn service_random_parameter_and_workload_are_emitted() {
    let world = World::new(&service_without_reward(false));
    let emission = fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
        .unwrap_or_else(|refusal| panic!("S4 emits the random parameter: {refusal:?}"));
    let wire: serde_json::Value =
        serde_json::from_slice(emission.package().bytes()).expect("JSON bytes");
    let count = |form: &str| {
        wire["semantic_graph"]["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .filter(|node| node["node_tag"] == "probabilistic" && node["semantic_form"] == form)
            .count()
    };
    assert_eq!((count("random_parameter"), count("workload")), (1, 1));
}

/// FR-185-AC-12 (TC-2062): the Boolean random parameter `ok` of `Service` is
/// refused at emission with its locus, because IR's reader admits no Boolean
/// support value (its join refuses every non-string literal payload).
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
fn service_refuses_emission_naming_its_boolean_random_parameter() {
    let world = World::new(&service_without_reward(true));
    let refusal = fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
        .expect_err("emission refuses the Boolean support");
    assert!(matches!(
        &refusal,
        fsl_package::EmitRefusal::ProbabilisticModelNotEmitted {
            declaration,
            feature: fsl_semantics::check::ProbabilisticFeature::RandomParameter,
            region,
            ..
        } if declaration == "attempt(ok)" && region.start() < region.end()
    ));
}

/// FSL FR-740 (TC-2062): the package route's input over the emitted
/// `Service` package carries the random parameters and the workload its nodes
/// hold, rebuilt as S3 recorded them: the same operations, positions, types,
/// support values, weights and probabilities.
#[trace("TC-2062", "FR-740-AC-1")]
#[test]
fn the_package_route_rebuilds_the_probabilistic_model_of_the_emitted_package() {
    // The package route reads no state frame yet, so the unit holds only the
    // probabilistic declarations.
    let mut model = world::service("");
    model.body = "random Server::attempt(d) ~ { 1: 90, 3: 7, 12: 3 };\n\
                  workload Steady on M { weight Server::request = 1; \
                  weight Server::attempt = 1; weight Server::reset = 1; }\n"
        .to_owned();
    let world = World::new(&model);
    let source = world.package.graph().probabilistic_model();
    let read = fsl_package::read_evaluation_package(
        &world.package,
        fsl_semantics::library::LibraryName::new("fixture").expect("a library name"),
        world.packages(),
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
    )
    .expect("the package emits and its bytes are admitted");
    let admitted = read.admitted();
    let mut rows_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let rows = read
        .emission()
        .identity_rows(admitted, &mut rows_budget)
        .expect("the identity rows join the package");
    let mut join_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let bare = fsl_package::BarePackage::new(admitted)
        .with_supply(
            fsl_package::BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut join_budget,
        )
        .expect("the bytes view admits its supply");
    let input = crate::simulation::model::PackageModelInput::of(
        &bare,
        world.packages(),
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .unwrap_or_else(|pending| panic!("the package route builds its input: {pending}"));
    let rebuilt = input.input().probabilistic;
    assert_eq!(rebuilt.random_parameters().len(), 1);
    for expected in source.random_parameters() {
        let found = rebuilt
            .random_parameter(expected.operation().key(), expected.parameter())
            .expect("the random parameter is rebuilt");
        assert_eq!(
            found.operation().declaring(),
            expected.operation().declaring()
        );
        assert_eq!(found.operation().name(), expected.operation().name());
        assert_eq!(found.position(), expected.position());
        assert_eq!(found.value_type(), expected.value_type());
        assert_eq!(found.support(), expected.support());
    }
    let steady = rebuilt.workload("Steady").expect("the workload is rebuilt");
    let expected = source.workload("Steady").expect("the source workload");
    assert_eq!(steady.weights().len(), expected.weights().len());
    for (expected, expected_weight) in expected.weights() {
        let (_, found_weight) = steady
            .weights()
            .iter()
            .find(|(found, _)| found.key() == expected.key())
            .expect("the operation is weighted");
        assert_eq!(found_weight, expected_weight);
    }
}

/// `use_bare` over `world`'s package emitted and read back from the wire
/// bytes.
fn with_bare<T>(world: &World, use_bare: impl FnOnce(&fsl_package::BarePackage<'_>) -> T) -> T {
    let read = fsl_package::read_evaluation_package(
        &world.package,
        fsl_semantics::library::LibraryName::new("fixture").expect("a library name"),
        world.packages(),
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
    )
    .expect("the package emits and its bytes are admitted");
    let admitted = read.admitted();
    let mut rows_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let rows = read
        .emission()
        .identity_rows(admitted, &mut rows_budget)
        .expect("the identity rows join the package");
    let mut join_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let bare = fsl_package::BarePackage::new(admitted)
        .with_supply(
            fsl_package::BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut join_budget,
        )
        .expect("the bytes view admits its supply");
    use_bare(&bare)
}

/// `use_input` over the package route's input for `world`'s package: emitted,
/// read back from the wire bytes, and rebuilt from the nodes.
fn through_the_wire<T>(
    world: &World,
    use_input: impl FnOnce(crate::simulation::model::ModelCheckInput<'_>) -> T,
) -> T {
    with_bare(world, |bare| {
        let input = crate::simulation::model::PackageModelInput::of(
            bare,
            world.packages(),
            fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        )
        .unwrap_or_else(|pending| panic!("the package route builds its input: {pending}"));
        use_input(input.input())
    })
}

/// Admission of `world`'s model through the package route.
fn wire_admission(world: &World) -> Result<(), AdmissionFailure> {
    through_the_wire(world, |input| {
        world.admit_input(input, 1_000_000).map(|_| ())
    })
}

/// The refusal record of `result`, which must be a refused admission.
fn refusal(
    result: Result<(), AdmissionFailure>,
) -> ((String, String), std::collections::BTreeMap<String, String>) {
    let Err(AdmissionFailure::Refused(record)) = result else {
        panic!("admission refuses");
    };
    (
        (
            record.catalog.code().to_owned(),
            record.catalog.cause().to_owned(),
        ),
        record
            .fields
            .iter()
            .map(|(name, value)| ((*name).to_owned(), value.clone()))
            .collect(),
    )
}

/// FR-187-AC-4 (TC-622 step 4): the package route reads a negative reward
/// from the wire, so a package whose `reward` node computes `n - 3` over
/// `n: Int[0, 5]` refuses admission
/// `invalid_probabilistic_model`/`negative-reward`, naming the operation, the
/// reward and `n = 0`, as the source route does.
#[trace("TC-622", "FR-187-AC-4")]
#[test]
fn a_negative_reward_read_from_the_wire_refuses_admission() {
    let model = world::refund();
    let world = World::with_s3_reward_signs_unchecked(&model);
    let (cause, fields) = refusal(wire_admission(&world));
    assert_eq!(
        cause,
        (
            Code::InvalidProbabilisticModel.as_str().to_owned(),
            "negative-reward".to_owned()
        )
    );
    assert_eq!(fields.get("reward").map(String::as_str), Some("refund"));
    assert_eq!(fields.get("operation"), Some(&model.ident("Account/pay")));
    assert_eq!(fields.get("arguments").map(String::as_str), Some("n = 0"));
}

/// FR-187-AC-5 (TC-2064): a package S3 never checked (`gap = m - n` over
/// `n: Int[0, 5]`, `m: Int[0, 2]`), read from the wire, is refused at
/// admission `invalid_probabilistic_model`/`negative-reward` at the first
/// vector in canonical order (the first parameter most significant) that is
/// negative, past the vectors that are not, naming the operation, the reward
/// and both arguments.
#[trace("TC-2064", "FR-187-AC-5")]
#[test]
fn a_hand_built_wire_package_with_a_negative_reward_is_refused_naming_its_arguments() {
    let mut model = world::scored();
    model.body = "reward gap on Account::pay = m - n;\n\
                  workload Unit on M { weight Account::pay = 1; }\n"
        .to_owned();
    let world = World::with_s3_reward_signs_unchecked(&model);
    let (cause, fields) = refusal(wire_admission(&world));
    assert_eq!(
        cause,
        (
            Code::InvalidProbabilisticModel.as_str().to_owned(),
            "negative-reward".to_owned()
        )
    );
    assert_eq!(fields.get("reward").map(String::as_str), Some("gap"));
    assert_eq!(fields.get("operation"), Some(&model.ident("Account/pay")));
    assert_eq!(
        fields.get("arguments").map(String::as_str),
        Some("n = 1, m = 0")
    );
}

/// FR-187-AC-4: with several negative rewards the package route names the
/// same first reward as the source route, whatever order they are written in:
/// rewards are checked by operation, then name.
#[trace("TC-622", "FR-187-AC-4")]
#[test]
fn both_routes_refuse_the_same_first_of_several_negative_rewards() {
    let mut model = world::refund();
    model.body = "reward zeta on Account::pay = n - 3;\n\
                  reward alpha on Account::pay = n - 4;\n\
                  workload Unit on M { weight Account::pay = 1; }\n"
        .to_owned();
    let world = World::with_s3_reward_signs_unchecked(&model);
    let source = refusal(world.admit().map(|_| ()));
    let wire = refusal(wire_admission(&world));
    assert_eq!(source, wire);
    assert_eq!(wire.1.get("reward").map(String::as_str), Some("alpha"));
}

/// FR-187-AC-4: the order across operations is by operation key before
/// reward name, on both routes: the reward on `cash` is named before the one
/// on `pay`, though `pay`'s is written first and sorts first by name.
#[trace("TC-622", "FR-187-AC-4")]
#[test]
fn both_routes_order_negative_rewards_by_operation_before_name() {
    let mut model = world::refund();
    let amount = model.ident("Amount");
    model.operations.push(world::Operation {
        name: "cash",
        parameters: vec![("n", amount)],
        modifies: vec![],
    });
    model.body = "reward alpha on Account::pay = n - 3;\n\
                  reward zeta on Account::cash = n - 4;\n\
                  workload Unit on M { weight Account::pay = 1; weight Account::cash = 1; }\n"
        .to_owned();
    let world = World::with_s3_reward_signs_unchecked(&model);
    let source = refusal(world.admit().map(|_| ()));
    let wire = refusal(wire_admission(&world));
    assert_eq!(source, wire);
    assert_eq!(wire.1.get("reward").map(String::as_str), Some("zeta"));
    assert_eq!(wire.1.get("operation"), Some(&model.ident("Account/cash")));
}

/// A reward whose `expression` names no function does not project: the
/// package route refuses the node with the typed selection failure, which
/// `PackageModelInput::of` gives as `PendingPackageForm::Reward`.
#[test]
fn a_reward_whose_function_is_missing_does_not_project() {
    let world = World::with_s3_reward_signs_unchecked(&world::refund());
    with_bare(&world, |bare| {
        let input = crate::simulation::model::PackageModelInput::of(
            bare,
            world.packages(),
            fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        )
        .unwrap_or_else(|pending| panic!("the package route builds its input: {pending}"));
        let mut reward = input.input().probabilistic.read_rewards()[0].clone();
        reward.function = reward.value_type.clone();
        assert!(matches!(
            crate::value::ProjectedReward::of(bare, &reward),
            Err(fsl_package::BareSelectionFailure::Slot(
                fsl_package::SlotSelectionFailure::MissingNode(_)
            ))
        ));
    });
}

/// The steps of the `scored` model's one state under `Unit`, as (non-random
/// arguments, reward) pairs, in ascending order, read through `system`.
fn scored_steps(system: &super::ModelSystem<'_>, world: &World) -> Vec<(Vec<i64>, i64)> {
    let start = initial(system);
    let steps = system
        .weighted_steps(&start, world.workload("Unit"))
        .expect("expands")
        .expect("decided");
    let mut seen: Vec<(Vec<i64>, i64)> = steps
        .iter()
        .map(|step| {
            let identity = step.identity.as_ref().expect("an operation step");
            let [(name, reward)] = &step.rewards[..] else {
                panic!("one reward: {:?}", step.rewards);
            };
            assert_eq!(name.as_str(), "score");
            (
                identity.arguments.iter().map(integer_of).collect(),
                integer_of(reward),
            )
        })
        .collect();
    seen.sort();
    seen
}

/// FR-187: the rewards of the steps the package route gives a positive
/// two-parameter reward are the ones the source route gives, and each is
/// `10 * n + m` of the step's own arguments.
#[trace("TC-622", "FR-187-AC-4")]
#[test]
fn the_package_route_gives_the_rewards_of_a_two_parameter_reward_as_the_source_route_does() {
    let world = World::new(&world::scored());
    let source = scored_steps(&world.system(), &world);
    assert_eq!(source.len(), 18);
    for (arguments, reward) in &source {
        let [n, m] = arguments[..] else {
            panic!("two arguments: {arguments:?}");
        };
        assert_eq!(*reward, 10 * n + m);
    }
    let wire = through_the_wire(&world, |input| {
        scored_steps(
            &world
                .admit_input(input, 1_000_000)
                .expect("the wire system admits"),
            &world,
        )
    });
    assert_eq!(source, wire);
}

/// FSpec FR-406-AC-6 (TC-351): under a workload that gives `send_b` weight
/// 0, every step of positive probability from every reachable state of
/// `Link` is one of FR-120's successors, so every behaviour of positive
/// probability is a behaviour of the model.
#[trace("FSpec-TC-351", "FSpec-FR-406-AC-6")]
#[test]
fn a_weight_zero_workload_keeps_every_step_inside_the_state_graph() {
    let mut link = world::link();
    link.body
        .push_str("workload Lean on M { weight Msg::send_a = 1; weight Msg::send_b = 0; }\n");
    let world = World::new(&link);
    let system = world.system();
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([initial(&system)]);
    while let Some(state) = queue.pop_front() {
        let (key, _) = system.state_key(&state).expect("a key");
        if !seen.insert(key) {
            continue;
        }
        let successors: BTreeSet<(Vec<u8>, Vec<u8>)> = system
            .successors(&state)
            .expect("expands")
            .successors
            .iter()
            .map(|(transition, post)| {
                (
                    system.transition_key(transition).expect("bytes"),
                    system.state_key(post).expect("key").0,
                )
            })
            .collect();
        let weighted = steps(&system, &state, &world, "Lean");
        assert_eq!(sum(&weighted), ratio(1, 1));
        for step in &weighted {
            if let TransitionId::Operation(transition) = &step.transition {
                let edge = (
                    system.transition_key(transition).expect("bytes"),
                    system.state_key(&step.post).expect("key").0,
                );
                assert!(successors.contains(&edge), "{state:?}");
            }
        }
        queue.extend(weighted.into_iter().map(|step| step.post));
    }
}

#[trace("TC-3152", "FR-1043-AC-9")]
#[test]
fn restored_model_keys_preserve_successors_and_refuse_initial_provenance() {
    use super::super::{ModelKeyBudget, ModelRestoreBudget, ModelStateProvenance};
    use filament_exact::{Cancel, IntegerDecimalBudget, Reservation, ReservationDenied};
    struct Budget {
        meter: filament_exact::Meter,
        used: usize,
    }
    impl Reservation for Budget {
        fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
            let actual = self.used.checked_add(bytes).ok_or(ReservationDenied)?;
            if actual > 4_000_000 {
                return Err(ReservationDenied);
            }
            self.used = actual;
            Ok(())
        }
    }
    impl IntegerDecimalBudget for Budget {
        fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
            self.meter
                .charge(
                    filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge)
                        .work(Integer::from(units)),
                )
                .map_err(|_| ReservationDenied)
        }
    }
    impl ModelKeyBudget for Budget {
        fn integer(&mut self, value: &Integer) -> Result<(), ReservationDenied> {
            self.meter
                .charge(
                    filament_exact::Charge::new(filament_exact::ChargePoint::GraphResultRetain)
                        .size(
                            filament_exact::LimitKind::IntegerBits,
                            value.magnitude_bits(),
                        ),
                )
                .map_err(|_| ReservationDenied)
        }
    }
    impl ModelRestoreBudget for Budget {
        fn meter(&mut self) -> &mut filament_exact::Meter {
            &mut self.meter
        }
    }
    let model = world::link();
    let world = World::new(&model);
    let system = world.system();
    let actual = initial(&system);
    let bytes = system.state_key(&actual).unwrap().0;
    let mut budget = Budget {
        meter: filament_exact::Meter::new(system.meter_limits()),
        used: 0,
    };
    let restored = system
        .restore_state_key_with_budget(
            &bytes,
            fsl_foundation::IdentityLimits::default(),
            &mut budget,
            &Cancel::new(),
        )
        .unwrap();
    assert_eq!(system.state_key(&restored).unwrap().0, bytes);
    assert!(matches!(
        restored.provenance(),
        ModelStateProvenance::RestoredFromKey { effect: None }
    ));
    assert!(restored.anchor().is_none() && restored.reached_by().is_none());
    let refused = restored.required_anchor();
    assert!(
        refused.is_err(),
        "key restoration cannot supply an initial anchor: {refused:?}"
    );
    let original = system.actions(&actual).unwrap().unwrap();
    let rebuilt = system.actions(&restored).unwrap().unwrap();
    assert_eq!(original.len(), rebuilt.len());
    for (original, rebuilt) in original.iter().zip(&rebuilt) {
        assert_eq!(original.identity, rebuilt.identity);
        assert_eq!(original.draws.len(), rebuilt.draws.len());
        for (original, rebuilt) in original.draws.iter().zip(&rebuilt.draws) {
            assert_eq!(original.probability, rebuilt.probability);
            assert_eq!(original.successors.len(), rebuilt.successors.len());
            for ((transition, post), (restored_transition, restored_post)) in
                original.successors.iter().zip(&rebuilt.successors)
            {
                assert_eq!(transition, restored_transition);
                assert_eq!(
                    system.state_key(post).unwrap().0,
                    system.state_key(restored_post).unwrap().0
                );
                assert!(restored_post.anchor().is_none());
                assert!(
                    restored_post.reached_by().is_some(),
                    "the new actual effect remains available"
                );
            }
        }
    }
    let cancel = Cancel::new();
    cancel.cancel(filament_exact::CancelCause::Deadline);
    let stopped = system.restore_state_key_with_budget(
        &bytes,
        fsl_foundation::IdentityLimits::default(),
        &mut budget,
        &cancel,
    );
    assert!(
        matches!(
            stopped,
            Err(super::super::ModelRestoreError::Key(
                super::super::ModelKeyError::Cancelled(filament_exact::CancelCause::Deadline)
            ))
        ),
        "{stopped:?}"
    );
}
