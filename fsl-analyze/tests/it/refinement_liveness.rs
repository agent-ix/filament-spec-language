// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143 (TC-548): the liveness half of a refinement, checked under the
//! abstract fairness `F_A` on the product the safety half retains.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use filament_exact::{Cancel, CancelCause};
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{operation_identity_text, ModelSimulationRequest};
use fsl_eval::simulation::Limits;
use fsl_package::CheckedPackage;
use fsl_replay::{
    open_systems, replay_refinement_trace, CounterexampleKind, IncompleteCause, LivenessHalf,
    ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, ReachedLimit, RefinementFailure,
    RefinementOutcome, RefinementTraceSettlement, SafetyOutcome, Taken, TemporalCounterexample,
    UndecidedCause,
};
use fsl_semantics::check::CheckedRefinement;
use fsl_semantics::model::observation::DocumentRef;
use ix_trace_rs::trace;

use crate::refinement_aux::{
    halting_refinement, hidden_register_history, pair_refinement, probe_contracts,
    probe_refinement, Shape as AuxShape, Unit, World as AuxWorld, NEVER,
    UNLIMITED as AUX_UNLIMITED,
};
use crate::refinement_fixture::{
    cas_refines_counter, divergence_refinement, key, mutated, World, CAS, UNLIMITED,
};

fn check(world: &World) -> RefinementOutcome {
    run(
        world.refinement(),
        world.concrete_request(UNLIMITED),
        world.package(),
        world.abstract_initial(),
    )
}

fn check_aux(world: &AuxWorld<'_>) -> RefinementOutcome {
    run(
        world.refinement(),
        world.concrete_request(AUX_UNLIMITED),
        world.package(),
        world.abstract_initial(),
    )
}

fn hidden_tick_refinement(rows: &str) -> String {
    format!(
        "refinement HiddenTick using inf abstract Spec concrete Impl {{\n\
         population Spec::ms from Impl::ms;\n\
         object Spec::M from Impl::M {{ v = self.v; }}\n\
         step Impl::M::tick -> any;\n\
         {rows}}}"
    )
}

fn run(
    refinement: &CheckedRefinement,
    concrete: ModelSimulationRequest<'_>,
    package: &CheckedPackage,
    abstract_initial: DocumentRef,
) -> RefinementOutcome {
    check_refinement(
        RefinementCheckRequest {
            refinement,
            concrete: ModelSubject {
                simulation: concrete,
                proof_bounds: Vec::new(),
            },
            abstract_package: package,
            abstract_initial: vec![abstract_initial].into(),
            abstract_protocol_binding: None,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked")
}

fn without_ensure(refinement: &str) -> String {
    mutated(refinement, "ensure fair weak each Spec::Counter::inc;", "")
}

fn holds(outcome: &RefinementOutcome) {
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    let Some(LivenessHalf::Checked(ModelCheckOutcome::HoldsReduced { reductions, .. })) =
        &outcome.liveness
    else {
        panic!("the liveness half holds: {:?}", outcome.liveness);
    };
    assert!(reductions.is_empty());
}

/// The lasso `outcome`'s liveness half refutes with.
fn lasso(outcome: RefinementOutcome) -> TemporalCounterexample {
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    let Some(LivenessHalf::Checked(ModelCheckOutcome::Violated { counterexample, .. })) =
        outcome.liveness
    else {
        panic!("the liveness half is violated: {:?}", outcome.liveness);
    };
    *counterexample
}

/// The operations of `counterexample`'s prefix, by name.
fn operations(counterexample: &TemporalCounterexample) -> Vec<&'static str> {
    let names = [
        "beginA", "beginB", "commitA", "commitB", "retryA", "retryB", "peek",
    ];
    counterexample
        .prefix
        .iter()
        .map(|step| {
            *names
                .iter()
                .find(|name| {
                    operation_identity_text(&key(CAS, &format!("Counter/{name}")))
                        .is_ok_and(|text| text == step.transition.operation())
                })
                .expect("a step of the concrete model")
        })
        .collect()
}

/// FR-143-AC-1 (TC-548 step 1): `CasRefinesCounter` holds both halves; its
/// divergence variant is refuted by an empty stem and a loop of one `peek`
/// step, failure `Divergence{inc}`; without the `ensure` row the same model
/// has no liveness half and a holding safety half.
#[trace("TC-548", "FR-143-AC-1")]
#[test]
fn the_compare_and_set_counter_holds_and_its_divergence_is_a_peek_loop() {
    holds(&check(&World::new(&cas_refines_counter())));

    let counterexample = lasso(check(&World::new(&divergence_refinement())));
    assert_eq!(counterexample.initial_state, 0);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        counterexample
            .lasso
            .as_ref()
            .map(|lasso| (lasso.entry, lasso.terminal_stutter)),
        Some((0, false))
    );
    assert_eq!(operations(&counterexample), ["peek"]);
    let Some(RefinementFailure::Divergence { constraint, .. }) = &counterexample.refinement else {
        panic!(
            "the failure is a divergence: {:?}",
            counterexample.refinement
        );
    };
    assert_eq!(constraint.as_ref().unwrap().operation().name(), "inc");

    let safety_only = check(&World::new(&without_ensure(&divergence_refinement())));
    assert!(matches!(safety_only.safety, SafetyOutcome::Holds { .. }));
    assert_eq!(safety_only.liveness, None);
}

/// The `terminal` declarations of FR-143-AC-2's combinations: each model
/// takes none, `terminal when self.value = 2`, or `terminal any`.
fn terminal_combinations() -> Vec<String> {
    let forms = |model: &str| {
        [
            String::new(),
            format!("terminal {model}Done using v on {model} when {{ self.value = 2 }};"),
            format!("terminal {model}Done using v on {model} any;"),
        ]
    };
    forms("Spec")
        .iter()
        .flat_map(|spec| forms("Impl").map(|concrete| format!("{spec}\n{concrete}")))
        .collect()
}

/// FR-143-AC-2 (TC-548 step 2): `Halting` is refuted by the stem `inc` and
/// the terminal stutter at `value` 1 with `Divergence{inc}`; without its
/// `ensure` row it has no liveness half; with the precondition
/// `self.value < 2` it holds. Each verdict is the same under every
/// combination of `terminal` declarations.
#[trace("TC-548", "FR-143-AC-2")]
#[test]
fn a_concrete_halt_is_a_divergence_whatever_the_terminal_declarations() {
    for terminals in terminal_combinations() {
        let unit = Unit::halting(1, &terminals);
        let counterexample = lasso(check_aux(&unit.world(&halting_refinement(true))));
        let inc = operation_identity_text(&unit.concrete_key("Counter/inc")).expect("a text");
        let stem: Vec<&str> = counterexample
            .prefix
            .iter()
            .map(|step| step.transition.operation())
            .collect();
        assert_eq!(stem, [inc.as_str()], "{terminals}");
        assert_eq!(
            counterexample
                .lasso
                .map(|lasso| (lasso.entry, lasso.terminal_stutter)),
            Some((1, true)),
            "{terminals}"
        );
        let Some(RefinementFailure::Divergence { constraint, .. }) = &counterexample.refinement
        else {
            panic!("{terminals}: a divergence: {:?}", counterexample.refinement);
        };
        assert_eq!(
            constraint.as_ref().unwrap().operation().name(),
            "inc",
            "{terminals}"
        );

        let safety_only = check_aux(&unit.world(&halting_refinement(false)));
        assert!(matches!(safety_only.safety, SafetyOutcome::Holds { .. }));
        assert_eq!(safety_only.liveness, None, "{terminals}");

        holds(&check_aux(
            &Unit::halting(2, &terminals).world(&halting_refinement(true)),
        ));
    }
}

/// FR-143-AC-3 (TC-548 step 3): with `flipA` fair and `flipB` not, a loop of
/// two `flipA` steps never takes the fair abstract `flipB`, and the failure
/// is `AbstractUnfair{flipB}`; making `flipB` fair as well holds.
#[trace("TC-548", "FR-143-AC-3")]
#[test]
fn a_loop_that_takes_abstract_steps_but_never_flip_b_is_abstract_unfair() {
    let unit = Unit::pair();
    let counterexample = lasso(check_aux(&unit.world(&pair_refinement(&["flipA"]))));
    assert_eq!(counterexample.prefix.len(), 2);
    assert_eq!(
        counterexample
            .lasso
            .map(|lasso| (lasso.entry, lasso.terminal_stutter)),
        Some((0, false))
    );
    let Some(RefinementFailure::AbstractUnfair { constraint }) = &counterexample.refinement else {
        panic!("an abstract unfairness: {:?}", counterexample.refinement);
    };
    assert_eq!(constraint.operation().name(), "flipB");
    let flipped: Vec<&str> = counterexample
        .prefix
        .iter()
        .map(|step| step.transition.operation())
        .collect();
    let flip_a = operation_identity_text(&unit.concrete_key("Pair/flipA")).expect("a text");
    assert_eq!(flipped, [flip_a.as_str(), flip_a.as_str()]);

    holds(&check_aux(
        &unit.world(&pair_refinement(&["flipA", "flipB"])),
    ));
}

/// FR-143-AC-4 (TC-548 step 4), FR-899-AC-1 (TC-2536): `RegisterHistory`
/// without its `prev` row holds its safety half and decides its liveness
/// half with an `ensure` row, by inclusion over the candidate-set product,
/// and has none without it.
#[trace("TC-548", "FR-143-AC-4")]
#[trace("TC-2536", "FR-899-AC-1")]
#[test]
fn liveness_through_a_hidden_abstract_field_is_decided() {
    let unit = Unit::register();
    let ensured = hidden_register_history();
    let outcome = check_aux(&unit.world(&ensured));
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    assert!(
        matches!(outcome.liveness, Some(LivenessHalf::Checked(_))),
        "{:?}",
        outcome.liveness
    );

    let outcome =
        check_aux(&unit.world(&ensured.replace("ensure fair weak Spec::Register::write;\n", "")));
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    assert_eq!(outcome.liveness, None);
}

/// FR-899-AC-1 (TC-2536): a recurrent concrete `peek` loop has no candidate
/// run that can take the abstract `write`, so hidden-field liveness refutes it.
#[trace("TC-2536", "FR-899-AC-1")]
#[test]
fn a_hidden_field_with_an_always_enabled_abstract_write_refutes_a_peek_loop() {
    let unit = Unit::registers(&["r"], &["r"], AuxShape::Peeker);
    let refinement = hidden_register_history().replace(
        "step Impl::Register::write",
        "step Impl::Register::peek -> stutter;\nstep Impl::Register::write",
    );
    let outcome = check_aux(&unit.world(&refinement));
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    let counterexample = match outcome.liveness {
        Some(LivenessHalf::Checked(ModelCheckOutcome::Violated { counterexample, .. })) => {
            counterexample
        }
        other => panic!("a hidden liveness counterexample: {other:?}"),
    };
    assert!(matches!(
        counterexample.refinement,
        Some(RefinementFailure::HiddenUnfair { .. })
    ));
}

/// FR-899-AC-2 (TC-2536): the same concrete write loop has both a stuttering
/// candidate run and one that takes the abstract write; the fair candidate
/// suffices for inclusion.
#[trace("TC-2536", "FR-899-AC-2")]
#[test]
fn the_non_taking_candidate_is_hidden_and_a_fair_write_candidate_holds() {
    let unit = Unit::register();
    let refinement = hidden_register_history().replace(
        "step Impl::Register::write -> Spec::Register::write(self, x);",
        "step Impl::Register::write -> any;",
    );
    // One concrete write(0) loop has both hidden-state candidate paths: an
    // `any` candidate can keep the abstract `prev` value or take write while
    // leaving the visible image unchanged. The non-taking path is weakly
    // unfair, while the candidate taking write supplies a fair run.
    let outcome = check_aux(&unit.world(&refinement));
    holds(&outcome);
    let SafetyOutcome::Holds { certificate, .. } = &outcome.safety else {
        panic!("the safety phase retains the candidate-set certificate");
    };
    assert!(
        certificate
            .positions
            .iter()
            .any(|position| position.abstract_states.len() > 1),
        "the same concrete loop retains both hidden candidate valuations"
    );
    let abstract_write = operation_identity_text(&unit.abstract_key("Register/write"))
        .expect("the abstract write identity");
    assert!(
        certificate.successors.iter().flatten().any(|edge| {
            edge.transition.is_some()
                && matches!(&edge.taken, Taken::CandidateSteps(steps)
                    if steps.iter().filter_map(|step| step.transition.as_ref())
                        .any(|transition| transition.operation() == abstract_write))
        }),
        "the candidate set includes a run taking the abstract write"
    );
}

/// The settlement FR-145 replay gives `payload` over `world`.
fn replayed(
    world: &AuxWorld<'_>,
    payload: &TemporalCounterexample,
) -> RefinementTraceSettlement {
    let systems = open_systems(
        world.refinement(),
        &world.concrete_request(AUX_UNLIMITED),
        world.package(),
        vec![world.abstract_initial()],
    )
    .expect("the subjects open");
    replay_refinement_trace(world.refinement(), &systems, payload).expect("the trace replays")
}

/// Replay reproduces exactly the failure `payload` records.
fn assert_reproduced(world: &AuxWorld<'_>, payload: &TemporalCounterexample) {
    match replayed(world, payload) {
        RefinementTraceSettlement::Reproduced { failure, .. } => {
            assert_eq!(Some(failure), payload.refinement);
        }
        other => panic!("the failure reproduces: {other:?}"),
    }
}

/// The operation identities of `payload`'s stem and of its loop.
fn stem_and_loop(payload: &TemporalCounterexample) -> (Vec<String>, Vec<String>) {
    let entry = payload.lasso.as_ref().expect("a lasso").entry;
    let identity = |step: &fsl_replay::ModelStep| step.transition.operation().to_owned();
    (
        payload.prefix[..entry].iter().map(identity).collect(),
        payload.prefix[entry..].iter().map(identity).collect(),
    )
}

/// FR-899-AC-6 / TC-2536 CX-1: an abstract fire taken only along a candidate
/// path that cannot recur does not make the concrete tick loop fair.
#[trace("TC-2536", "FR-899-AC-6")]
#[test]
fn cx1_refutes_when_fire_only_leaves_the_candidate_cycle() {
    let contracts = "pre AdvanceEnabled using v on Spec::M::advance { self.h = 0 }\n\
        post AdvanceStep using v on Spec::M::advance { self.h = 1 and self.v = pre(self.v) }\n\
        post FireStep using v on Spec::M::fire {\n\
          (pre(self.h) = 1 and self.h = 2 and self.v = pre(self.v)) or\n\
          (pre(self.h) != 1 and self.h = pre(self.h) and self.v = 1)\n\
        }\n";
    let unit = Unit::hidden_fairness_model(2, 0, &["advance", "fire"], contracts);
    let refinement = hidden_tick_refinement("ensure fair weak Spec::M::fire;\n");
    let world = unit.world(&refinement);
    let outcome = check_aux(&world);
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    let counterexample = lasso(outcome);
    assert!(matches!(
        counterexample.refinement,
        Some(RefinementFailure::HiddenUnfair { .. })
    ));
    let (stem, cycle) = stem_and_loop(&counterexample);
    let tick = operation_identity_text(&unit.concrete_key("M/tick")).expect("the tick identity");
    assert_eq!(stem, [tick.clone(), tick.clone()]);
    assert_eq!(cycle, [tick]);
    assert_reproduced(&world, &counterexample);

    // With `fire` also mapping `h = 2` to `h = 1` keeping `v`, the cycle
    // `1 -> 2 -> 1` is a fair candidate run: the check holds and the earlier
    // counterexample no longer reproduces.
    let edited_contracts = contracts.replace(
        "(pre(self.h) = 1 and self.h = 2 and self.v = pre(self.v)) or\n",
        "(pre(self.h) = 1 and self.h = 2 and self.v = pre(self.v)) or\n\
          (pre(self.h) = 2 and self.h = 1 and self.v = pre(self.v)) or\n",
    );
    assert_ne!(edited_contracts, contracts, "the fire row is edited");
    let edited_unit =
        Unit::hidden_fairness_model(2, 0, &["advance", "fire"], &edited_contracts);
    let edited_world = edited_unit.world(&refinement);
    holds(&check_aux(&edited_world));
    assert!(
        !matches!(
            replayed(&edited_world, &counterexample),
            RefinementTraceSettlement::Reproduced { .. }
        ),
        "the earlier counterexample does not reproduce once the cycle is fair"
    );
}

/// FR-899-AC-7 / TC-2536 CX-2: each constraint has a fair candidate alone,
/// but no one candidate run satisfies both constraints.
#[trace("TC-2536", "FR-899-AC-7")]
#[test]
fn cx2_requires_one_candidate_run_to_satisfy_all_fairness_constraints() {
    let contracts = "post Fire1Step using v on Spec::M::fire1 {\n\
          (pre(self.h) = 0 and self.h = 0 and self.v = pre(self.v)) or\n\
          (pre(self.h) != 0 and self.h = pre(self.h) and self.v = 1)\n\
        }\n\
        post Fire2Step using v on Spec::M::fire2 {\n\
          (pre(self.h) = 1 and self.h = 1 and self.v = pre(self.v)) or\n\
          (pre(self.h) != 1 and self.h = pre(self.h) and self.v = 1)\n\
        }\n";
    let unit = Unit::hidden_fairness_model(2, 0, &["fire1", "fire2"], contracts)
        .with_hidden_initials(&[1]);
    let refinement = hidden_tick_refinement(
        "ensure fair weak Spec::M::fire1;\nensure fair weak Spec::M::fire2;\n",
    );
    let world = unit.world(&refinement);
    let counterexample = lasso(check_aux(&world));
    let Some(RefinementFailure::HiddenUnfair { constraint }) = counterexample.refinement.as_ref()
    else {
        panic!("the joint candidate constraint prefix refutes");
    };
    assert_eq!(constraint.operation().name(), "fire2");
    assert_reproduced(&world, &counterexample);

    // Replay does not reproduce the same lasso under another constraint's name.
    let fire1 = world
        .refinement()
        .abstract_operation_fairness()
        .find(|constraint| constraint.operation().name() == "fire1")
        .cloned()
        .expect("the fire1 constraint");
    let mut renamed = counterexample.clone();
    renamed.refinement = Some(RefinementFailure::HiddenUnfair { constraint: fire1 });
    assert!(
        !matches!(
            replayed(&world, &renamed),
            RefinementTraceSettlement::Reproduced { .. }
        ),
        "the first constraint is satisfiable alone, so it is not the one named"
    );

    // Without `fire2` the one constraint left is satisfiable by a candidate.
    let without_fire2 = hidden_tick_refinement("ensure fair weak Spec::M::fire1;\n");
    holds(&check_aux(&unit.world(&without_fire2)));
}

/// FR-899-AC-8 / TC-2536 CX-3: a candidate that never enables fire supplies
/// a fair run even when another candidate enables it continuously.
#[trace("TC-2536", "FR-899-AC-8")]
#[test]
fn cx3_holds_when_one_candidate_never_enables_fire() {
    let contracts = "pre FireEnabled using v on Spec::M::fire { self.h = 0 }\n\
        post FireStep using v on Spec::M::fire { self.h = 0 and self.v = pre(self.v) }\n";
    let unit = Unit::hidden_fairness_model(2, 0, &["fire"], contracts)
        .with_hidden_initials(&[1]);
    let refinement = hidden_tick_refinement("ensure fair weak Spec::M::fire;\n");
    let world = unit.world(&refinement);
    holds(&check_aux(&world));

    // A hand-built counterexample naming `HiddenUnfair{fire}` for the tick
    // loop does not reproduce: the candidate that never enables `fire` is a
    // fair run. The payload is the tick loop of AC-6's fixture, renamed to
    // this world's `fire` constraint.
    let loop_contracts = "pre AdvanceEnabled using v on Spec::M::advance { self.h = 0 }\n\
        post AdvanceStep using v on Spec::M::advance { self.h = 1 and self.v = pre(self.v) }\n\
        post FireStep using v on Spec::M::fire {\n\
          (pre(self.h) = 1 and self.h = 2 and self.v = pre(self.v)) or\n\
          (pre(self.h) != 1 and self.h = pre(self.h) and self.v = 1)\n\
        }\n";
    let loop_unit = Unit::hidden_fairness_model(2, 0, &["advance", "fire"], loop_contracts);
    let loop_refinement = hidden_tick_refinement("ensure fair weak Spec::M::fire;\n");
    let mut built = lasso(check_aux(&loop_unit.world(&loop_refinement)));
    let fire = world
        .refinement()
        .abstract_operation_fairness()
        .find(|constraint| constraint.operation().name() == "fire")
        .cloned()
        .expect("the fire constraint");
    built.refinement = Some(RefinementFailure::HiddenUnfair { constraint: fire });
    assert!(
        !matches!(
            replayed(&world, &built),
            RefinementTraceSettlement::Reproduced { .. }
        ),
        "a loop with a fair candidate run is not hidden-unfair"
    );
}

/// `Probe` over the concrete guards `concrete` (`flipA`, `flipB`, `setA`,
/// `peek`) and the abstract ones `abstract_` (`flipA`, `flipB`, `setA`),
/// starting at `initials`.
fn probe(concrete: [&str; 4], abstract_: [&str; 3], initials: &[(i64, i64)]) -> Unit {
    Unit::probe_pair(probe_contracts(concrete, abstract_), initials)
}

/// The concrete model that can only `peek`, the abstract one unguarded.
fn peeker() -> Unit {
    probe([NEVER, NEVER, NEVER, ""], ["", "", ""], &[(0, 0)])
}

fn operation_of(outcome: RefinementOutcome) -> String {
    match lasso(outcome).refinement {
        Some(RefinementFailure::Divergence {
            constraint: Some(constraint),
            ..
        })
        | Some(RefinementFailure::AbstractUnfair { constraint }) => {
            constraint.operation().name().to_owned()
        }
        other => panic!("a liveness failure: {other:?}"),
    }
}

/// FR-143-AC-6, FR-899-AC-3 (TC-548, TC-2537): a strong constraint in `F_A`
/// is read strongly. The loop `flipA, flipA` visits `a = 0` forever and never
/// takes the strong `flipB`, so the liveness half is violated naming `flipB`,
/// with the proved safety half beside it; a weak `F_A` holds as before. With
/// the safety half violated, the liveness half is not run.
#[trace("TC-548", "FR-143-AC-6")]
#[trace("TC-2537", "FR-899-AC-3", "FR-899-AC-4")]
#[test]
fn a_strong_abstract_fairness_constraint_refutes_a_loop_that_never_takes_it() {
    let unit = probe(
        ["", "self.a = 0", "", ""],
        ["", "self.a = 0", ""],
        &[(0, 0)],
    );
    let flip_a = "assume fair weak each Impl::Pair::flipA;";
    let concrete_strong = "assume fair strong each Impl::Pair::flipB;";
    let weak = "ensure fair weak whole Spec::Pair::flipB;";
    let strong = "ensure fair strong whole Spec::Pair::flipB;";
    holds(&check_aux(&unit.world(&probe_refinement(&[flip_a, weak]))));
    holds(&check_aux(&unit.world(&probe_refinement(&[
        flip_a,
        concrete_strong,
        strong,
    ]))));
    for rows in [
        vec![flip_a, strong],
        vec![flip_a, weak, strong],
        vec![flip_a, strong, weak],
    ] {
        let outcome = check_aux(&unit.world(&probe_refinement(&rows)));
        let counterexample = lasso(outcome);
        let Some(RefinementFailure::AbstractUnfair { constraint }) =
            counterexample.refinement.as_ref()
        else {
            panic!(
                "the strong constraint is abstract-unfair: {:?}",
                counterexample.refinement
            );
        };
        assert_eq!(constraint.operation().name(), "flipB", "{rows:?}");
    }
    let violated = probe_refinement(&[flip_a, strong]).replace(
        "step Impl::Pair::flipA -> Spec::Pair::flipA(self);",
        "step Impl::Pair::flipA -> stutter;",
    );
    let outcome = check_aux(&unit.world(&violated));
    assert!(matches!(outcome.safety, SafetyOutcome::Violated { .. }));
    assert_eq!(outcome.liveness, None);
}

/// FR-899-AC-4 (TC-2537): concrete strong fairness removes the otherwise
/// abstract-unfair loop, so the liveness inclusion holds.
#[trace("TC-2537", "FR-899-AC-4")]
#[test]
fn concrete_strong_fairness_removes_the_unfair_refinement_loop() {
    let unit = probe(
        ["", "self.a = 0", "", ""],
        ["", "self.a = 0", ""],
        &[(0, 0)],
    );
    let assume_flip_a = "assume fair weak each Impl::Pair::flipA;";
    let assume_strong_flip_b = "assume fair strong each Impl::Pair::flipB;";
    let ensure_strong_flip_b = "ensure fair strong whole Spec::Pair::flipB;";
    holds(&check_aux(&unit.world(&probe_refinement(&[
        assume_flip_a,
        assume_strong_flip_b,
        ensure_strong_flip_b,
    ]))));
}

/// FR-143 Second phase (TC-548): the classes are taken in `F_A`'s source
/// order, so of two violated constraints the first written is the one named.
#[trace("TC-548")]
#[test]
fn the_first_violated_constraint_in_source_order_is_named() {
    let unit = peeker();
    let flip_a = "ensure fair weak whole Spec::Pair::flipA;";
    let flip_b = "ensure fair weak whole Spec::Pair::flipB;";
    let named = |rows: &[&str]| operation_of(check_aux(&unit.world(&probe_refinement(rows))));
    assert_eq!(named(&[flip_a, flip_b]), "flipA");
    assert_eq!(named(&[flip_b, flip_a]), "flipB");
}

/// FR-143 Second phase (TC-548): an `each` constraint is one class for each
/// abstract transition identity and a `whole` constraint one class for the
/// operation. Where only `setA` runs, a loop of `setA(1)` leaves `setA(0)`
/// enabled and never taken, which violates `each` and not `whole`.
#[trace("TC-548")]
#[test]
fn an_each_constraint_is_a_class_for_each_identity_and_a_whole_one_for_the_operation() {
    let unit = probe([NEVER, NEVER, "", NEVER], ["", "", ""], &[(0, 0)]);
    let whole = unit.world(&probe_refinement(&[
        "ensure fair weak whole Spec::Pair::setA;",
    ]));
    holds(&check_aux(&whole));
    let each = unit.world(&probe_refinement(&[
        "ensure fair weak each Spec::Pair::setA;",
    ]));
    let counterexample = lasso(check_aux(&each));
    assert!(matches!(
        counterexample.refinement,
        Some(RefinementFailure::AbstractUnfair { .. })
    ));
    assert_eq!(
        counterexample.prefix.len() - counterexample.lasso.as_ref().expect("a lasso").entry,
        1
    );
}

/// FR-143 Outputs (TC-548): the lasso starts from the initial state the
/// behaviour begins at. Of two initial states only the second can peek
/// forever, and the lasso's `initial_state` is 1.
#[trace("TC-548")]
#[test]
fn the_lasso_names_the_initial_state_it_starts_from() {
    let unit = probe(
        [NEVER, NEVER, NEVER, "self.a = 1"],
        ["", "self.a = 1", ""],
        &[(0, 0), (1, 0)],
    );
    let world = unit.world(&probe_refinement(&[
        "ensure fair weak whole Spec::Pair::flipB;",
    ]));
    let outcome = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(AUX_UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: world.abstract_initials().into(),
            abstract_protocol_binding: None,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked");
    let counterexample = lasso(outcome);
    assert_eq!(counterexample.initial_state, 1);
    assert!(counterexample.prefix.is_empty() || counterexample.lasso.is_some());
}

fn peeker_run(limits: ModelCheckLimits, cancel: &Cancel) -> RefinementOutcome {
    let unit = peeker();
    let world = unit.world(&probe_refinement(&[
        "ensure fair weak whole Spec::Pair::flipA;",
        "ensure fair weak whole Spec::Pair::flipB;",
    ]));
    check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(AUX_UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()].into(),
            abstract_protocol_binding: None,
            max_depth: None,
            limits,
        },
        cancel,
    )
    .expect("the refinement is checked")
}

fn stopped(outcome: &RefinementOutcome) -> Option<(IncompleteCause, Option<ReachedLimit>)> {
    match &outcome.liveness {
        Some(LivenessHalf::Checked(ModelCheckOutcome::Stopped { cause, limit, .. })) => {
            Some((cause.clone(), *limit))
        }
        _ => None,
    }
}

/// FR-143 Behavior (TC-548): the second phase respects `max_transitions`,
/// which the safety half has not used up, and returns `Stopped` naming it.
#[trace("TC-548")]
#[test]
fn the_second_phase_stops_at_the_transition_budget() {
    let full = peeker_run(ModelCheckLimits::default(), &Cancel::new());
    let SafetyOutcome::Holds { stats, .. } = &full.safety else {
        panic!("the safety half holds");
    };
    let used = usize::try_from(stats.product_edges).expect("a count");
    let limits = ModelCheckLimits {
        limits: Limits {
            max_transitions: used,
            ..Limits::default()
        },
        ..ModelCheckLimits::default()
    };
    let outcome = peeker_run(limits, &Cancel::new());
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    assert_eq!(
        stopped(&outcome),
        Some((
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit {
                limit: ModelCheckLimit::MaxTransitions,
                value: u64::try_from(used).expect("a count"),
            })
        ))
    );
}

/// FR-143 Behavior (TC-548): a handle cancelled during the second phase
/// stops it with `Stopped{Cancelled}`, the safety half already holding.
#[trace("TC-548")]
#[test]
fn a_cancellation_during_the_second_phase_stops_it() {
    let charges = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&charges);
    let counting = Cancel::observing(move || {
        counter.fetch_add(1, Ordering::Relaxed);
    });
    peeker_run(ModelCheckLimits::default(), &counting);
    let total = charges.load(Ordering::Relaxed);
    let cancelled_at = |at: u32| {
        let seen = Arc::new(AtomicU32::new(0));
        let handle: Arc<OnceLock<Cancel>> = Arc::default();
        let (count, inner) = (Arc::clone(&seen), Arc::clone(&handle));
        let cancel = Cancel::observing(move || {
            if count.fetch_add(1, Ordering::Relaxed) + 1 == at {
                if let Some(cancel) = inner.get() {
                    cancel.cancel(CancelCause::Requested);
                }
            }
        });
        assert!(handle.set(cancel.clone()).is_ok(), "set once");
        peeker_run(ModelCheckLimits::default(), &cancel)
    };
    let mut live = 0;
    for at in 1..=total {
        let outcome = cancelled_at(at);
        if let Some((cause, limit)) = stopped(&outcome) {
            assert_eq!(
                cause,
                IncompleteCause::Cancelled {
                    source: CancelCause::Requested
                }
            );
            assert_eq!(limit, None);
            assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
            live += 1;
        }
    }
    assert!(live > 0, "a charge of the second phase is cancelled");
}

/// FR-143 Behavior (TC-548): an abstract contract conjunction that does not
/// decide, here a postcondition of `flipB` undefined at `b = 1`, leaves the
/// enabledness of the class unknown, so the half is `Undecided`.
#[trace("TC-548")]
#[test]
fn an_undecided_abstract_contract_leaves_the_liveness_half_undecided() {
    let contracts = format!(
        "function unknown_at_one using v(n: Int[0, 1]): Boolean pure {{ 1 mod (1 - n) >= 0 }}\n{}",
        probe_contracts([NEVER, NEVER, NEVER, ""], ["", "", ""])
    )
    .replace(
        "post SpecFlipB using v on Spec::Pair::flipB { self.b = 1 - pre(self.b) }",
        "post SpecFlipB using v on Spec::Pair::flipB { unknown_at_one(pre(self.b)) and self.b = 1 - pre(self.b) }",
    );
    let unit = Unit::probe_pair(contracts, &[(0, 1)]);
    let outcome = check_aux(&unit.world_with_mode(
        &probe_refinement(&["ensure fair weak whole Spec::Pair::flipB;"]),
        CheckMode::Kernel,
    ));
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    assert!(matches!(
        outcome.liveness,
        Some(LivenessHalf::Checked(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::UndecidedSuccessor,
            ..
        }))
    ));
}

/// FR-143-AC-6, FR-899-AC-1 (TC-548, TC-2536): a refinement with a hidden
/// abstract field and a strong `F_A` constraint decides its liveness half
/// over the candidate-set product with the strong reading.
#[trace("TC-548", "FR-143-AC-6")]
#[trace("TC-2536", "FR-899-AC-1")]
#[test]
fn a_hidden_field_with_a_strong_constraint_is_decided() {
    let unit = Unit::register();
    let strong = hidden_register_history().replace("ensure fair weak", "ensure fair strong");
    let outcome = check_aux(&unit.world(&strong));
    assert!(
        matches!(outcome.liveness, Some(LivenessHalf::Checked(_))),
        "{:?}",
        outcome.liveness
    );
}

/// The charges a run of `Pair`'s refinement with `refinement` makes against
/// its cancellation handle.
fn charges_of(refinement: &str) -> u32 {
    let unit = Unit::pair();
    let world = unit.world(refinement);
    let charges = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&charges);
    let counting = Cancel::observing(move || {
        counter.fetch_add(1, Ordering::Relaxed);
    });
    let outcome = run_cancel(&world, &counting);
    holds(&outcome);
    charges.load(Ordering::Relaxed)
}

fn run_cancel(world: &AuxWorld<'_>, cancel: &Cancel) -> RefinementOutcome {
    check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(AUX_UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()].into(),
            abstract_protocol_binding: None,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        cancel,
    )
    .expect("the refinement is checked")
}

/// FR-143 Behavior (TC-548): the fairness decomposition of each class is
/// made under the run's own cancellation handle. A second class adds the
/// charges of its decomposition to the one the class loop polls, so a handle
/// that the decomposition did not see would add one charge, not more.
#[trace("TC-548")]
#[test]
fn the_decomposition_of_each_class_charges_the_runs_cancellation_handle() {
    let one = pair_refinement(&["flipA", "flipB"]);
    let two = one.replace(
        "ensure fair weak each Spec::Pair::flipB;",
        "ensure fair weak each Spec::Pair::flipB;\nensure fair weak each Spec::Pair::flipA;",
    );
    assert_ne!(one, two);
    assert!(charges_of(&two) >= charges_of(&one) + 2);
}
