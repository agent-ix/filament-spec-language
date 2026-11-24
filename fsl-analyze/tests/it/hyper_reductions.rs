// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174-AC-7 (TC-3011): a key symmetry and a state constraint selected on an
//! HP-2 item that FR-174's hyper reduction eligibility rule admits are
//! executed, keep the verdict and report `HoldsReduced`; a "yes" selection on
//! an ineligible form settles V-8 before any expansion.

use filament_exact::{Cancel, CancelCause, LimitKind, ScalarLimits};
use fsl_analyze::model_check::ModelCheckRefusal;
use fsl_foundation::ReductionKind;
use fsl_replay::{
    AppliedReduction, HyperOutcome, HyperReductionCause, ModelCheckLimits, ModelTraceSettlement,
    RunStats, SymmetryBrokenCause, SymmetryRefusal,
};
use fsl_route::reductions::{Reductions, SymmetryDeclaration};
use fsl_semantics::check::{HyperForm, TemporalProfile};
use ix_trace_rs::trace;

use crate::hyper_support::refuted;
use crate::hyper_world::{key, non_interference, opaque, Vault, World, EQUAL_INPUTS, STARTS};

/// The two vaults `v` and `w` of one initial state.
const TWO_VAULTS: [(&str, (i64, i64)); 2] = [("v", (0, 0)), ("w", (1, 0))];

/// The initial states of `keys`, each holding every vault with one secret:
/// closed under every renaming of the keys.
fn uniform_snapshots<'k>(keys: &[&'k str]) -> Vec<Vec<(&'k str, (i64, i64))>> {
    [0, 1]
        .into_iter()
        .map(|secret| keys.iter().map(|key| (*key, (secret, 0))).collect())
        .collect()
}

/// `NonInterference`'s `match`, pairing steps of one vault with equal inputs.
const SAME_VAULT: &str = "match { a.step.op = b.step.op and a.step.receiver = b.step.receiver \
     and a.step.Vault::step.i = b.step.Vault::step.i }";

/// The key symmetry of `vaults` over one class of `keys`.
fn class_of(keys: &[&str]) -> Reductions {
    Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: key("vaults"),
            classes: vec![keys.iter().map(|key| (*key).to_owned()).collect()],
        }],
        ..Reductions::default()
    }
}

/// The key symmetry of `vaults` over `v` and `w`.
fn symmetry() -> Reductions {
    class_of(&["v", "w"])
}

/// [`symmetry`] with the state constraint `constraint` also selected.
fn with_constraint(reductions: Reductions, constraint: &str) -> Reductions {
    Reductions {
        constraint: Some(constraint.to_owned()),
        ..reductions
    }
}

/// The secure vault with the state constraints `Low` (met by every reachable
/// state) and `Zero` (cut on the first step that sets `l`) declared as
/// `at current` clauses of its model.
const SECURE_CONSTRAINED: Vault = Vault::Custom(
    "post Step using v on V::Vault::step { self.l = i }\n\
     invariant Low using v on V::Vault at current { self.l <= 1 }\n\
     invariant Zero using v on V::Vault at current { self.l = 0 }\n",
);

/// `NonInterference` over the vaults `v` and `w` of `vault`.
fn interference(vault: Vault) -> World {
    interference_over(vault, &["v", "w"])
}

/// `NonInterference` over the vaults `keys` of `vault`, whose initial states
/// are closed under every renaming of them.
fn interference_over(vault: Vault, keys: &[&str]) -> World {
    World::with_snapshots(
        vault,
        uniform_snapshots(keys),
        keys,
        &non_interference(SAME_VAULT),
    )
}

fn product_states(outcome: &HyperOutcome) -> u64 {
    match outcome {
        HyperOutcome::Holds { stats, .. } | HyperOutcome::HoldsReduced { stats, .. } => {
            stats.product_states
        }
        other => panic!("expected a holding outcome, got {other:?}"),
    }
}

#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_key_symmetry_keeps_the_verdict_with_fewer_states() {
    let secure = interference(Vault::Secure);
    let unreduced = secure
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("not refused");
    let reduced = secure.run_selecting(symmetry()).expect("not refused");
    let HyperOutcome::HoldsReduced { reductions, .. } = &reduced else {
        panic!("expected HoldsReduced, got {reduced:?}");
    };
    assert!(
        matches!(&reductions[..], [AppliedReduction::Symmetry { orbit, .. }, AppliedReduction::CopySwap { .. }] if orbit == &["v".to_owned(), "w".to_owned()]),
        "{reductions:?}"
    );
    assert!(product_states(&reduced) < product_states(&unreduced));
    let settled = fsl_replay::settle_hyper_check(
        &reduced,
        &ModelCheckLimits::default(),
        &[],
        None,
        &Cancel::new(),
        |_| panic!("a holding symmetry result replays nothing"),
    )
    .expect("FR-174's key-symmetry row needs no core certificate");
    assert_eq!(
        settled.value,
        fsl_replay::TerminalValue::Proved {
            basis: fsl_replay::ProofBasis::Reduced {
                reductions: reductions.clone(),
            },
            certification: fsl_replay::Certification::Uncertified,
        }
    );
    assert!(settled.certificates.is_empty());
    assert!(settled.counterexample.is_none());

    let leaky = interference(Vault::Leaky);
    let counterexample = refuted(leaky.run_selecting(symmetry()).expect("not refused"));
    assert_eq!(counterexample.traces.len(), 2);
    assert!(matches!(
        leaky.replay(&counterexample),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
}

#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_state_constraint_reduces_and_holds_reduced() {
    let secure = interference(SECURE_CONSTRAINED);
    let low = secure
        .run_selecting(with_constraint(Reductions::default(), "Low"))
        .expect("not refused");
    let HyperOutcome::HoldsReduced { reductions, .. } = &low else {
        panic!("expected HoldsReduced under Low, got {low:?}");
    };
    assert!(
        matches!(&reductions[..], [AppliedReduction::CopySwap { .. }]),
        "{reductions:?}"
    );
    let subjects = (0..secure.clause.aliases().len())
        .map(|_| secure.subject())
        .collect::<Vec<_>>();
    let request = fsl_replay::ProductClosureRequest {
        package: &secure.package,
        clause: fsl_replay::ProductClosureClause::Hyper(&secure.clause),
        subjects: &subjects,
        meter: crate::counter::UNLIMITED,
    };
    let settled = fsl_replay::settle_hyper_check(
        &low,
        &ModelCheckLimits::default(),
        &[],
        Some(&request),
        &Cancel::new(),
        |_| panic!("a holding constrained result replays nothing"),
    )
    .expect("a constraint with no cut has a closed copy-swap certificate");
    assert!(matches!(
        settled.value,
        fsl_replay::TerminalValue::Proved {
            certification: fsl_replay::Certification::Certified,
            ..
        }
    ));
    assert!(!settled.certificates.is_empty());
    let zero = secure
        .run_selecting(with_constraint(Reductions::default(), "Zero"))
        .expect("not refused");
    assert!(
        format!("{zero:?}").contains("ConstraintReached"),
        "{zero:?}"
    );
    let both = secure
        .run_selecting(with_constraint(symmetry(), "Low"))
        .expect("not refused");
    let HyperOutcome::HoldsReduced { reductions, .. } = &both else {
        panic!("expected HoldsReduced, got {both:?}");
    };
    assert!(
        matches!(
            &reductions[..],
            [
                AppliedReduction::Symmetry { .. },
                AppliedReduction::CopySwap { .. }
            ]
        ),
        "{reductions:?}"
    );
}

/// Finite HP-2 reads the closed tuple quotient, preserves its reduction
/// ordering and retains the refusal precedence before subject expansion.
#[trace("TC-3011", "FR-174-AC-7", "FR-173-AC-6")]
#[test]
fn a_finite_hp2_selection_executes_and_preserves_refusal_precedence() {
    let world = World::with_snapshots(
        SECURE_CONSTRAINED,
        uniform_snapshots(&["v", "w"]),
        &["v", "w"],
        "hyper Bounded using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         { always [0, 2] holds(v.l @ a <= 1 and v.l @ b <= 1) }",
    );
    assert_eq!(world.clause.form(), Some(&HyperForm::Universal));
    assert_eq!(
        world.clause.profile(),
        TemporalProfile::EventPositionFalseExtension
    );
    let ordinary = world
        .run_selecting(Reductions::default())
        .expect("the finite clause runs");
    let HyperOutcome::HoldsClosed { stats } = ordinary else {
        panic!("expected ordinary closed enumeration, got {ordinary:?}");
    };
    assert!(stats.product_states > 0, "{stats:?}");

    for selection in [symmetry(), with_constraint(symmetry(), "Low")] {
        let reduced = world.run_selecting(selection).expect("the selection runs");
        let HyperOutcome::HoldsReduced {
            reductions,
            scope: fsl_replay::HyperReducedScope::ClosedExecutions,
            stats: reduced_stats,
            ..
        } = reduced
        else {
            panic!("expected a reduced closed proof, got {reduced:?}");
        };
        assert!(matches!(
            &reductions[..],
            [
                AppliedReduction::Symmetry { .. },
                AppliedReduction::CopySwap { .. }
            ]
        ));
        assert!(reduced_stats.product_states < stats.product_states);
    }
    for selection in [
        with_constraint(Reductions::default(), "Zero"),
        with_constraint(symmetry(), "Zero"),
    ] {
        let cut = world.run_selecting(selection).expect("the constraint runs");
        assert!(
            matches!(
                cut,
                HyperOutcome::Reduction {
                    cause: HyperReductionCause::ConstraintReached { boundary_states },
                    ..
                } if boundary_states > 0
            ),
            "a cut cannot prove the finite clause: {cut:?}"
        );
    }
    let mut nonpreserving = with_constraint(symmetry(), "Zero");
    nonpreserving.partial_order = true;
    assert_eq!(
        world
            .run_selecting(nonpreserving)
            .expect("the preservation row settles"),
        HyperOutcome::Reduction {
            cause: HyperReductionCause::NotPreserving {
                reduction: ReductionKind::PartialOrder,
                form: HyperForm::Universal,
            },
            stats: RunStats::default(),
        }
    );
}

/// Three keys leave a nontrivial stabiliser after binding the least key.
/// At horizon one there are 12 concrete paths. Copy-swap leaves 78 unordered
/// pairs; the simultaneous `w`/`x` action leaves exactly (78 + 14) / 2 = 46.
/// Ignoring that action, renaming each component independently or only
/// dropping duplicate object bindings produces a different count.
#[trace("TC-3011", "FR-174-AC-7", "FR-173-AC-6")]
#[test]
fn a_finite_diagonal_stabiliser_stores_the_actual_closed_tuple_orbits() {
    let keys = ["v", "w", "x"];
    let world = World::with_snapshots(
        SECURE_CONSTRAINED,
        uniform_snapshots(&keys),
        &keys,
        "hyper Bounded using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         { always [0, 1] holds(v.l @ a <= 1 and v.l @ b <= 1) }",
    );
    let ordinary = world
        .run_selecting(Reductions::default())
        .expect("finite scope");
    let HyperOutcome::HoldsClosed { stats } = ordinary else {
        panic!("expected an ordinary closed proof: {ordinary:?}");
    };
    assert_eq!(stats.product_states, 3 * 12 * 12);
    let reduced = world.run_selecting(class_of(&keys)).expect("closed orbits");
    let HyperOutcome::HoldsReduced {
        reductions, stats, ..
    } = &reduced
    else {
        panic!("expected a reduced proof: {reduced:?}");
    };
    assert_eq!(stats.product_states, 46);
    let [AppliedReduction::Symmetry {
        orbit, stabiliser, ..
    }, AppliedReduction::CopySwap { .. }] = &reductions[..]
    else {
        panic!("the diagonal group precedes copy-swap: {reductions:?}");
    };
    assert_eq!(orbit, &["v", "w", "x"]);
    assert_eq!(stabiliser[0].keys, ["w", "x"]);
    let settled = fsl_replay::settle_hyper_check(
        &reduced,
        &ModelCheckLimits::default(),
        &[],
        None,
        &Cancel::new(),
        |_| panic!("a proof replays no counterexample"),
    )
    .expect("the selected key symmetry keeps its uncertified settlement");
    assert!(matches!(
        settled.value,
        fsl_replay::TerminalValue::Proved {
            certification: fsl_replay::Certification::Uncertified,
            ..
        }
    ));
}

/// A finite constraint proves only when every retained path is closed. Two
/// initial secrets yield three unordered predecessor tuples at depth zero;
/// cutting either or both components still names each predecessor once.
#[trace("TC-3011", "FR-174-AC-7", "FR-173-AC-6")]
#[test]
fn a_finite_constraint_keeps_genuine_boundaries_out_of_its_proof() {
    let world = World::with_snapshots(
        SECURE_CONSTRAINED,
        uniform_snapshots(&["v"]),
        &["v"],
        "hyper Bounded using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         { always [0, 1] holds(v.l @ a <= 1 and v.l @ b <= 1) }",
    );
    let low = world
        .run_selecting(with_constraint(Reductions::default(), "Low"))
        .expect("every state meets Low");
    let HyperOutcome::HoldsReduced {
        scope: fsl_replay::HyperReducedScope::ClosedExecutions,
        ref reductions,
        ..
    } = low
    else {
        panic!("expected a complete reduced finite scope: {low:?}");
    };
    assert!(matches!(
        &reductions[..],
        [AppliedReduction::CopySwap { .. }]
    ));
    let settled = fsl_replay::settle_hyper_check(
        &low,
        &ModelCheckLimits::default(),
        &[],
        None,
        &Cancel::new(),
        |_| panic!("a proof replays no counterexample"),
    )
    .expect("finite closure needs no infinite-product certificate");
    assert!(matches!(
        settled.value,
        fsl_replay::TerminalValue::Proved {
            certification: fsl_replay::Certification::Certified,
            ..
        }
    ));
    let cut = world
        .run_selecting(with_constraint(Reductions::default(), "Zero"))
        .expect("Zero prunes the step setting l to one");
    let HyperOutcome::Reduction {
        cause: HyperReductionCause::ConstraintReached { boundary_states },
        stats,
    } = &cut
    else {
        panic!("a cut is not a proof: {cut:?}");
    };
    assert_eq!(*boundary_states, 3);
    assert!(
        stats.product_states > 0,
        "allowed closed tuples are still read"
    );
    let settled = fsl_replay::settle_hyper_check(
        &cut,
        &ModelCheckLimits::default(),
        &[],
        None,
        &Cancel::new(),
        |_| panic!("a boundary replays no counterexample"),
    )
    .expect("the finite boundary retains the existing V-6 settlement");
    assert!(matches!(
        settled.value,
        fsl_replay::TerminalValue::Inconclusive(fsl_replay::InconclusiveCause::ConstraintReached {
            boundary_states: 3
        })
    ));
}

/// The quotient selects actual, deterministic closed paths. A key renaming
/// never turns a canonical state key into a fabricated concrete transition.
#[trace("TC-3011", "FR-174-AC-7", "FR-173-AC-6")]
#[test]
fn a_finite_reduced_refutation_is_deterministic_and_replays_concretely() {
    let keys = ["v", "w", "x"];
    let world = World::with_snapshots(
        Vault::Leaky,
        uniform_snapshots(&keys),
        &keys,
        &format!(
            "hyper Bounded using ep over behaviours (v: V::Vault) \
            forall trace a of V forall trace b of V {SAME_VAULT} \
            {{ always [0, 1] holds(v.l @ a = v.l @ b) }}"
        ),
    );
    let first = refuted(world.run_selecting(class_of(&keys)).expect("reduced run"));
    let second = refuted(world.run_selecting(class_of(&keys)).expect("same request"));
    assert_eq!(first, second);
    assert_eq!(first.kind, fsl_replay::HyperCounterexampleKind::Closed);
    assert_eq!(first.traces.len(), 2);
    assert_eq!(first.traces[0].variable, "a");
    assert_eq!(first.traces[1].variable, "b");
    assert!(first.traces.iter().all(|trace| trace.steps.len() == 1));
    assert!(matches!(
        world.replay(&first),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
}

/// A cut transition whose entering match fails is outside the product. Its
/// predecessor cannot turn an otherwise complete proof into ConstraintReached.
#[trace("TC-3011", "FR-174-AC-7", "FR-173-AC-6")]
#[test]
fn a_finite_unmatched_cut_is_not_a_constraint_boundary() {
    let world = World::with_snapshots(
        SECURE_CONSTRAINED,
        uniform_snapshots(&["v"]),
        &["v"],
        "hyper Bounded using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         match { \
           (if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) = 0 else false) \
           and (if present(b.step.Vault::step.i) then value(b.step.Vault::step.i) = 0 else false) \
         } { always [0, 1] holds(v.l @ a <= 1 and v.l @ b <= 1) }",
    );
    let result = world
        .run_selecting(with_constraint(Reductions::default(), "Zero"))
        .expect("the matched zero-input paths are closed");
    assert!(
        matches!(
            result,
            HyperOutcome::HoldsReduced {
                scope: fsl_replay::HyperReducedScope::ClosedExecutions,
                ..
            }
        ),
        "unmatched i=1 steps create no product boundary: {result:?}"
    );
}

/// The state constraint is applied to the second subject as well as the
/// first, including an initial state that violates it.
#[trace("TC-3011", "FR-174-AC-7", "FR-173-AC-6")]
#[test]
fn a_finite_constraint_checks_every_subject_and_its_initial_state() {
    let world = World::with_second_vaults(
        SECURE_CONSTRAINED,
        &[("v", (0, 0))],
        &[("v", (1, 1))],
        "hyper Bounded using ep over behaviours (d: V::Vault, e: N::Vault) \
         forall trace a of V forall trace b of N \
         { always [0, 1] holds(d.l @ a <= 1 and e.l @ b <= 1) }",
    );
    let low = world
        .run_selecting(with_constraint(Reductions::default(), "Low"))
        .expect("Low holds in both subjects");
    assert!(matches!(low, HyperOutcome::HoldsReduced { .. }), "{low:?}");
    let zero = world
        .run_selecting(with_constraint(Reductions::default(), "Zero"))
        .expect("Zero cuts the second subject's initial state");
    assert!(
        matches!(
            zero,
            HyperOutcome::Reduction {
                cause: HyperReductionCause::ConstraintReached { boundary_states: 1 },
                ..
            }
        ),
        "the second subject cannot be skipped: {zero:?}"
    );
}

/// Short terminal components keep their own closed scope under reduction;
/// they are never lengthened to satisfy the remaining bounded positions.
#[trace("TC-3011", "FR-174-AC-7", "FR-173-AC-6")]
#[test]
fn a_finite_constraint_preserves_false_extension_and_terminal_replay() {
    let world = World::with_snapshots(
        Vault::Custom(
            "pre Fresh using v on V::Vault::step { self.l = 0 }\n\
            post Step using v on V::Vault::step { self.l = i }\n\
            invariant Low using v on V::Vault at current { self.l <= 1 }\n",
        ),
        uniform_snapshots(&["v"]),
        &["v"],
        "hyper Bounded using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         { always [0, 2] holds(v.l @ a <= 1 and v.l @ b <= 1) }",
    );
    let result = world
        .run_selecting(with_constraint(Reductions::default(), "Low"))
        .expect("the terminal finite paths are decided");
    let counterexample = refuted(result);
    assert!(counterexample
        .traces
        .iter()
        .any(|trace| trace.steps.len() < 2));
    assert_eq!(
        counterexample.kind,
        fsl_replay::HyperCounterexampleKind::Closed
    );
    assert!(matches!(
        world.replay(&counterexample),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
}

/// A zero joint-move allowance prevents the first real finite product step;
/// traversal and canonicalization own a separate graph-work meter.
#[trace("TC-3011", "FR-174-AC-7", "FR-184")]
#[test]
fn a_finite_orbit_walk_stops_at_the_actual_transition_allowance() {
    let world = World::with_snapshots(
        SECURE_CONSTRAINED,
        uniform_snapshots(&["v"]),
        &["v"],
        "hyper Bounded using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         { always [0, 1] holds(v.l @ a <= 1 and v.l @ b <= 1) }",
    );
    let seams = world.seams();
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_transitions = 0;
    let result = fsl_analyze::model_check::check_hyper(
        fsl_analyze::model_check::HyperCheckRequest {
            clause: &world.clause,
            subjects: vec![world.subject()],
            letters: &seams,
            matcher: &seams,
            limits,
            meter: crate::counter::UNLIMITED,
            reductions: with_constraint(Reductions::default(), "Low"),
            max_depth: None,
        },
        &Cancel::new(),
    )
    .expect("the caller's zero allowance is a typed stop");
    assert!(
        matches!(
            result,
            HyperOutcome::Stopped {
                limit: Some(fsl_replay::ReachedLimit {
                    limit: fsl_replay::ModelCheckLimit::MaxTransitions,
                    value: 0
                }),
                ..
            }
        ),
        "{result:?}"
    );
    assert_eq!(result.stats().product_edges, 0);
}

const FINITE_PAIR: &str = "hyper Bounded using ep over behaviours (v: V::Vault) \
    forall trace a of V forall trace b of V \
    { always [0, 1] holds(v.l @ a <= 1 and v.l @ b <= 1) }";

fn finite_with_limits(
    world: &World,
    reductions: Reductions,
    limits: ModelCheckLimits,
    meter: ScalarLimits,
    cancel: &Cancel,
) -> HyperOutcome {
    let seams = world.seams();
    fsl_analyze::model_check::check_hyper(
        fsl_analyze::model_check::HyperCheckRequest {
            clause: &world.clause,
            subjects: (0..world.clause.aliases().len())
                .map(|_| world.subject())
                .collect(),
            letters: &seams,
            matcher: &seams,
            limits,
            meter,
            reductions,
            max_depth: None,
        },
        cancel,
    )
    .expect("the finite request has a typed outcome")
}

/// A later sibling cannot consume the allowance before the second complete
/// canonical tuple is judged. Neither returned trace is a partial prefix.
#[trace("TC-3011", "FR-173-AC-6", "FR-184")]
#[test]
fn a_finite_first_closed_refutation_precedes_later_sibling_budget_work() {
    let world = World::new(
        SECURE_CONSTRAINED,
        false,
        &[(0, 0)],
        "hyper Bounded using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         { always [0, 1] holds(v.l @ a = 0 and v.l @ b = 0) }",
    );
    let run = |bound| {
        let mut limits = ModelCheckLimits::default();
        limits.limits.max_transitions = bound;
        finite_with_limits(
            &world,
            Reductions::default(),
            limits,
            crate::counter::UNLIMITED,
            &Cancel::new(),
        )
    };
    let ample = refuted(run(100));
    assert_eq!(ample.kind, fsl_replay::HyperCounterexampleKind::Closed);
    assert_eq!(ample.traces.len(), 2);
    assert!(ample.traces.iter().all(|trace| trace.steps.len() == 1));
    assert_eq!(crate::hyper_support::inputs(&ample, 0), vec![0]);
    assert_eq!(crate::hyper_support::inputs(&ample, 1), vec![1]);
    assert!(matches!(
        world.replay(&ample),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
    let bounded = run(2);
    assert!(
        matches!(bounded, HyperOutcome::Violated { .. }),
        "the second canonical complete tuple refutes within two moves: {bounded:?}"
    );
    let bounded = refuted(bounded);
    assert_eq!(bounded, ample, "the first canonical tuple is stable");
    assert!(matches!(
        world.replay(&bounded),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
    let short = run(1);
    assert!(
        matches!(
            short,
            HyperOutcome::Stopped {
                limit: Some(fsl_replay::ReachedLimit {
                    limit: fsl_replay::ModelCheckLimit::MaxTransitions,
                    value: 1
                }),
                ..
            }
        ),
        "the first tuple holds and a second move is still required: {short:?}"
    );
    assert_eq!(short.stats().product_edges, 1);
}

/// The selected route streams the least actual copy-orbit member, rather
/// than waiting for every concrete image to finish or relabeling a prefix.
#[trace("TC-3011", "FR-174-AC-7", "FR-173-AC-6", "FR-184")]
#[test]
fn a_finite_selected_closed_refutation_precedes_later_orbit_budget_work() {
    let world = World::new(
        SECURE_CONSTRAINED,
        false,
        &[(0, 0)],
        "hyper Bounded using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         { always [0, 1] holds(v.l @ a = 0 and v.l @ b = 0) }",
    );
    let run = |bound| {
        let mut limits = ModelCheckLimits::default();
        limits.limits.max_transitions = bound;
        finite_with_limits(
            &world,
            with_constraint(Reductions::default(), "Low"),
            limits,
            crate::counter::UNLIMITED,
            &Cancel::new(),
        )
    };
    let ample = refuted(run(100));
    let bounded = refuted(run(2));
    assert_eq!(bounded, ample);
    assert_eq!(crate::hyper_support::inputs(&bounded, 0), vec![0]);
    assert_eq!(crate::hyper_support::inputs(&bounded, 1), vec![1]);
    assert!(matches!(
        world.replay(&bounded),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
    let short = run(1);
    assert!(
        matches!(
            short,
            HyperOutcome::Stopped {
                limit: Some(fsl_replay::ReachedLimit {
                    limit: fsl_replay::ModelCheckLimit::MaxTransitions,
                    value: 1
                }),
                ..
            }
        ),
        "{short:?}"
    );
}

/// Joint-depth DFS would visit a's second step(1) before b's first step(1).
/// The ordinary order instead finishes a's whole first path before varying b.
#[trace("TC-3011", "FR-173-AC-6")]
#[test]
fn a_finite_stream_keeps_component_cartesian_order_across_joint_depths() {
    let world = World::new(
        SECURE_CONSTRAINED,
        false,
        &[(0, 0)],
        "hyper Bounded using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         { always [0, 2] holds(v.l @ a = 0) and always [0, 1] holds(v.l @ b = 0) }",
    );
    let counterexample = refuted(finite_with_limits(
        &world,
        Reductions::default(),
        ModelCheckLimits::default(),
        crate::counter::UNLIMITED,
        &Cancel::new(),
    ));
    assert_eq!(crate::hyper_support::inputs(&counterexample, 0), vec![0, 0]);
    assert_eq!(crate::hyper_support::inputs(&counterexample, 1), vec![1, 0]);
    assert!(matches!(
        world.replay(&counterexample),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
}

/// An excluded initial state contributes a boundary, without expanding its
/// two successors or constructing their unconstrained complete-path orbits.
#[trace("TC-3011", "FR-174-AC-7", "FR-184")]
#[test]
fn a_finite_initial_cut_precedes_completion_and_expansion_budgets() {
    let mut world = World::new(SECURE_CONSTRAINED, false, &[(1, 1)], FINITE_PAIR);
    world.max_candidates = 0;
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 1;
    limits.limits.max_transitions = 0;
    let cut = finite_with_limits(
        &world,
        with_constraint(Reductions::default(), "Zero"),
        limits,
        crate::counter::UNLIMITED,
        &Cancel::new(),
    );
    assert!(
        matches!(
            cut,
            HyperOutcome::Reduction {
                cause: HyperReductionCause::ConstraintReached { boundary_states: 1 },
                ..
            }
        ),
        "no excluded expansion may override the initial cut: {cut:?}"
    );
    assert_eq!(cut.stats().product_edges, 0);
    assert_eq!(cut.stats().product_states, 0);
}

/// The real model expansion fits three candidates at l=0 but requires four
/// at l=1. The latter state is reached by a matched step and cut by Zero;
/// expanding that excluded descendant would incorrectly exhaust its budget.
#[trace("TC-3011", "FR-174-AC-7", "FR-184")]
#[test]
fn a_finite_matched_cut_prevents_a_real_descendant_expansion_failure() {
    let mut world = World::new(
        Vault::Custom(
            "pre Guard using v on V::Vault::step { self.l = 0 implies i = 0 }\n\
            post Step using v on V::Vault::step { self.l = 1 }\n\
            invariant Zero using v on V::Vault at current { self.l = 0 }\n",
        ),
        false,
        &[(0, 0)],
        &format!(
            "hyper Bounded using ep over behaviours (v: V::Vault) \
            forall trace a of V forall trace b of V {SAME_VAULT} \
            {{ always [0, 3] holds(v.l @ a <= 1 and v.l @ b <= 1) }}"
        ),
    );
    world.max_candidates = 3;
    let control = finite_with_limits(
        &world,
        Reductions::default(),
        ModelCheckLimits::default(),
        crate::counter::UNLIMITED,
        &Cancel::new(),
    );
    assert!(
        matches!(
            control,
            HyperOutcome::Stopped {
                limit: Some(fsl_replay::ReachedLimit {
                    limit: fsl_replay::ModelCheckLimit::MaxCandidates,
                    value: 3
                }),
                ..
            }
        ),
        "the excluded descendant genuinely exhausts three candidates: {control:?}"
    );
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 1;
    limits.limits.max_transitions = 0;
    let cut = finite_with_limits(
        &world,
        with_constraint(Reductions::default(), "Zero"),
        limits,
        crate::counter::UNLIMITED,
        &Cancel::new(),
    );
    assert!(
        matches!(
            cut,
            HyperOutcome::Reduction {
                cause: HyperReductionCause::ConstraintReached { boundary_states: 1 },
                ..
            }
        ),
        "entering match and cut must precede the descendant failure: {cut:?}"
    );
    assert_eq!(
        cut.stats().product_edges,
        0,
        "the cut successor is not a stored product move"
    );
}

/// Unconstrained paths have eight completions each. Zero leaves one real
/// completed joint tuple and three predecessor prefixes, all within one
/// stored complete member and three admitted joint steps.
#[trace("TC-3011", "FR-174-AC-7", "FR-184")]
#[test]
fn a_finite_cut_excludes_completions_before_the_stored_state_allowance() {
    let world = World::new(
        SECURE_CONSTRAINED,
        false,
        &[(0, 0)],
        &FINITE_PAIR.replace("[0, 1]", "[0, 3]"),
    );
    let seams = world.seams();
    let systems = seams.per_trace();
    let component = fsl_eval::hyper::closed_executions(
        systems[0],
        3,
        100,
        &std::collections::BTreeSet::new(),
        &Cancel::new(),
    )
    .expect("the unconstrained control has all of its completions");
    assert_eq!(component.len(), 8);
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 1;
    limits.limits.max_transitions = 3;
    let cut = finite_with_limits(
        &world,
        with_constraint(Reductions::default(), "Zero"),
        limits,
        crate::counter::UNLIMITED,
        &Cancel::new(),
    );
    assert!(
        matches!(
            cut,
            HyperOutcome::Reduction {
                cause: HyperReductionCause::ConstraintReached { boundary_states: 3 },
                ..
            }
        ),
        "unconstrained completions must not consume the admitted scope: {cut:?}"
    );
    assert_eq!(cut.stats().product_states, 1);
    assert_eq!(cut.stats().product_edges, 3);
}

/// Four actual horizon-one paths form sixteen joint moves. Group work is a
/// separate scalar-meter pass and cannot consume extra transition units.
#[trace("TC-3011", "FR-174-AC-7", "FR-184")]
#[test]
fn a_finite_nonzero_transition_allowance_has_the_same_joint_move_units_under_reduction() {
    let world = World::with_snapshots(
        SECURE_CONSTRAINED,
        uniform_snapshots(&["v"]),
        &["v"],
        FINITE_PAIR,
    );
    for reduced in [false, true] {
        let selection = || {
            if reduced {
                with_constraint(Reductions::default(), "Low")
            } else {
                Reductions::default()
            }
        };
        let mut limits = ModelCheckLimits::default();
        limits.limits.max_states = 16;
        limits.limits.max_transitions = 16;
        let exact = finite_with_limits(
            &world,
            selection(),
            limits,
            crate::counter::UNLIMITED,
            &Cancel::new(),
        );
        if reduced {
            assert!(
                matches!(exact, HyperOutcome::HoldsReduced { .. }),
                "{exact:?}"
            );
            assert_eq!(exact.stats().product_states, 10);
        } else {
            assert!(
                matches!(exact, HyperOutcome::HoldsClosed { .. }),
                "{exact:?}"
            );
            assert_eq!(exact.stats().product_states, 16);
        }
        assert_eq!(
            exact.stats().product_edges,
            16,
            "actual joint moves, reduced={reduced}"
        );
        limits.limits.max_transitions = 15;
        let short = finite_with_limits(
            &world,
            selection(),
            limits,
            crate::counter::UNLIMITED,
            &Cancel::new(),
        );
        assert!(
            matches!(
                short,
                HyperOutcome::Stopped {
                    limit: Some(fsl_replay::ReachedLimit {
                        limit: fsl_replay::ModelCheckLimit::MaxTransitions,
                        value: 15
                    }),
                    ..
                }
            ),
            "one fewer actual joint move stops: {short:?}"
        );
        assert_eq!(short.stats().product_edges, 15);
    }
}

/// With no joint step to take, canonicalization still spends the genuine
/// graph-work meter and names its existing work_units setting when it stops.
#[trace("TC-3011", "FR-174-AC-7", "FR-184", "NFR-009")]
#[test]
fn a_finite_canonicalization_stop_names_the_work_meter_and_not_a_product_limit() {
    let world = World::new(
        SECURE_CONSTRAINED,
        false,
        &[(0, 0)],
        &FINITE_PAIR.replace("always [0, 1] ", ""),
    );
    let mut meter = crate::counter::UNLIMITED;
    meter.work_units = 8;
    let stopped = finite_with_limits(
        &world,
        with_constraint(Reductions::default(), "Low"),
        ModelCheckLimits::default(),
        meter,
        &Cancel::new(),
    );
    assert!(
        matches!(
            stopped,
            HyperOutcome::Stopped {
                limit: Some(fsl_replay::ReachedLimit {
                    limit: fsl_replay::ModelCheckLimit::EvaluationMeter {
                        counter: LimitKind::WorkUnits
                    },
                    value: 8,
                }),
                ..
            }
        ),
        "canonicalization has real bounded work even with no product move: {stopped:?}"
    );
    assert_eq!(stopped.stats().product_edges, 0);
}

struct CancelAfterMatch<'a> {
    inner: &'a dyn fsl_eval::hyper::StepMatch,
    cancel: &'a Cancel,
    matches: std::cell::Cell<usize>,
}

impl fsl_eval::hyper::StepMatch for CancelAfterMatch<'_> {
    fn universal(
        &self,
        instance: &[fsl_eval::hyper::ParameterBinding],
        steps: &[fsl_eval::hyper::StepLabel<'_>],
        meter: &mut filament_exact::Meter,
    ) -> Result<fsl_eval::hyper::MatchReading, fsl_eval::hyper::MatchIncomplete> {
        let reading = self.inner.universal(instance, steps, meter);
        self.matches.set(self.matches.get() + 1);
        self.cancel.cancel(CancelCause::Deadline);
        reading
    }

    fn existential(
        &self,
        instance: &[fsl_eval::hyper::ParameterBinding],
        steps: &[fsl_eval::hyper::StepLabel<'_>],
        meter: &mut filament_exact::Meter,
    ) -> Result<fsl_eval::hyper::MatchReading, fsl_eval::hyper::MatchIncomplete> {
        self.inner.existential(instance, steps, meter)
    }
}

/// An in-flight cancellation is observed by the graph pass, retains its real
/// owner cause, and prevents another entering match from being evaluated.
#[trace("TC-3011", "FR-174-AC-7", "FR-184")]
#[test]
fn a_finite_joint_walk_retains_inflight_cancellation_as_a_typed_stop() {
    let world = World::new(
        SECURE_CONSTRAINED,
        false,
        &[(0, 0)],
        &format!(
            "hyper Bounded using ep over behaviours (v: V::Vault) \
            forall trace a of V forall trace b of V {SAME_VAULT} \
            {{ always [0, 2] holds(v.l @ a <= 1 and v.l @ b <= 1) }}"
        ),
    );
    let seams = world.seams();
    let cancel = Cancel::new();
    let matcher = CancelAfterMatch {
        inner: &seams,
        cancel: &cancel,
        matches: std::cell::Cell::new(0),
    };
    let result = fsl_analyze::model_check::check_hyper(
        fsl_analyze::model_check::HyperCheckRequest {
            clause: &world.clause,
            subjects: vec![world.subject()],
            letters: &seams,
            matcher: &matcher,
            limits: ModelCheckLimits::default(),
            meter: crate::counter::UNLIMITED,
            reductions: with_constraint(Reductions::default(), "Low"),
            max_depth: None,
        },
        &cancel,
    )
    .expect("cancellation is a typed graph stop");
    assert_eq!(matcher.matches.get(), 1);
    assert!(
        matches!(
            result,
            HyperOutcome::Stopped {
                cause: fsl_replay::IncompleteCause::Cancelled {
                    source: CancelCause::Deadline
                },
                limit: None,
                ..
            }
        ),
        "cancellation is not a work or product limit: {result:?}"
    );
}

#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_key_symmetry_on_an_ineligible_form_settles_unsupported() {
    let world = World::with_vaults(Vault::Secure, &TWO_VAULTS, &opaque(""));
    let outcome = world.run_selecting(symmetry()).expect("not refused");
    let HyperOutcome::Reduction { cause, stats } = outcome else {
        panic!("expected a reduction cause, got {outcome:?}");
    };
    assert_eq!(
        cause,
        HyperReductionCause::Unsupported {
            reduction: ReductionKind::Symmetry
        }
    );
    assert_eq!(stats, fsl_replay::RunStats::default());
    // The eligible HP-2 item is not refused by the same selection.
    let eligible = interference(Vault::Secure)
        .run_selecting(symmetry())
        .expect("not refused");
    assert!(!matches!(eligible, HyperOutcome::Reduction { .. }));
}

/// Whether two outcomes give one verdict: both hold, or both are violated.
fn same_verdict(left: &HyperOutcome, right: &HyperOutcome) -> bool {
    let holds = |outcome: &HyperOutcome| {
        matches!(
            outcome,
            HyperOutcome::Holds { .. } | HyperOutcome::HoldsReduced { .. }
        )
    };
    let violated = |outcome: &HyperOutcome| matches!(outcome, HyperOutcome::Violated { .. });
    (holds(left) && holds(right)) || (violated(left) && violated(right))
}

#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_state_constraint_is_eligible_on_every_hp2_item() {
    let power = World::with_second_vaults(
        SECURE_CONSTRAINED,
        &[("v", (0, 0))],
        &[("v", (1, 0))],
        "hyper SavesPower using inf over behaviours (d: V::Vault, e: N::Vault) \
         forall trace s of V forall trace n of N \
         match { s.step.Vault::step.i = n.step.Vault::step.i } \
         { always holds(d.l @ s = e.l @ n) }",
    );
    let each = World::new(
        SECURE_CONSTRAINED,
        false,
        &crate::hyper_world::STARTS,
        "hyper P using inf over behaviours (v: V::Vault) \
         forall trace a of V fair weak each V::Vault::step; \
         forall trace b of V fair weak each V::Vault::step; \
         match { a.step.op = b.step.op } { always holds(v.l @ a = v.l @ b) }",
    );
    for world in [&power, &each] {
        let constrained = world
            .run_selecting(with_constraint(Reductions::default(), "Low"))
            .expect("not refused");
        assert!(
            !matches!(constrained, HyperOutcome::Reduction { .. }),
            "{constrained:?}"
        );
        let unconstrained = world
            .run_selecting(Reductions::default())
            .expect("not refused");
        assert!(
            same_verdict(&constrained, &unconstrained),
            "{constrained:?} against {unconstrained:?}"
        );
    }
}

/// FR-151 "Form": a symmetry declaration naming a key outside the subject's
/// universe is refused before any expansion.
#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_symmetry_over_a_key_outside_the_universe_is_refused() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let refusal = world
        .run_selecting(symmetry())
        .expect_err("the key `w` is not in the universe");
    assert!(
        matches!(
            refusal,
            ModelCheckRefusal::Symmetry(SymmetryRefusal::UnknownKey { .. })
        ),
        "{refusal:?}"
    );
}

/// FR-151 "Closed initial states": initial states a declared swap of `v` and
/// `w` maps to no initial state settle `SymmetryBroken` before any expansion.
#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn initial_states_not_closed_under_the_symmetry_settle_symmetry_broken() {
    let world = World::with_vaults(Vault::Secure, &TWO_VAULTS, &non_interference(EQUAL_INPUTS));
    let outcome = world.run_selecting(symmetry()).expect("not refused");
    let HyperOutcome::Reduction { cause, stats } = outcome else {
        panic!("expected a reduction cause, got {outcome:?}");
    };
    assert!(
        matches!(
            cause,
            HyperReductionCause::SymmetryBroken {
                cause: SymmetryBrokenCause::InitialStatesNotClosed { .. },
                ..
            }
        ),
        "{cause:?}"
    );
    assert_eq!(stats, RunStats::default());
}

/// The diagonal action over a stabiliser that still has keys to swap: the
/// clause binds `v`, so `w` and `x` stay interchangeable in the one instance
/// the orbit `[v, w, x]` leaves. The secure vault holds with fewer product
/// states than the unreduced check, and the leaky vault's counterexample is
/// concretised through the key renaming and replays.
#[trace("TC-3011", "FR-174-AC-7")]
#[test]
fn a_key_symmetry_with_a_nontrivial_stabiliser_keeps_the_verdict() {
    let keys = ["v", "w", "x"];
    let secure = interference_over(Vault::Secure, &keys);
    let unreduced = secure
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("not refused");
    let reduced = secure.run_selecting(class_of(&keys)).expect("not refused");
    let HyperOutcome::HoldsReduced { reductions, .. } = &reduced else {
        panic!("expected HoldsReduced, got {reduced:?}");
    };
    let [AppliedReduction::Symmetry {
        orbit, stabiliser, ..
    }, AppliedReduction::CopySwap { .. }] = &reductions[..]
    else {
        panic!("expected a symmetry then a copy swap, got {reductions:?}");
    };
    assert_eq!(orbit, &["v", "w", "x"]);
    let [class] = &stabiliser[..] else {
        panic!("expected one stabiliser class, got {stabiliser:?}");
    };
    assert_eq!(class.keys, ["w", "x"]);
    assert!(product_states(&reduced) < product_states(&unreduced));

    let leaky = interference_over(Vault::Leaky, &keys);
    let counterexample = refuted(leaky.run_selecting(class_of(&keys)).expect("not refused"));
    assert!(matches!(
        leaky.replay(&counterexample),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
}

/// FR-174-AC-4: a state constraint that does not preserve HP-3 is rejected
/// before any state is expanded.
#[trace("TC-599", "FR-174-AC-4")]
#[test]
fn a_selected_nonpreserving_reduction_is_refused_before_expansion() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match { a.step.Vault::step.i = b.step.Vault::step.i } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }",
    );
    let selected = Reductions {
        constraint: Some("Bounded".to_owned()),
        ..Reductions::default()
    };
    let outcome = world
        .run_selecting(selected)
        .expect("the reduction selection parses");
    assert_eq!(
        outcome,
        HyperOutcome::Reduction {
            cause: HyperReductionCause::NotPreserving {
                reduction: ReductionKind::StateConstraint,
                form: HyperForm::ForallExistsSafety,
            },
            stats: RunStats::default(),
        }
    );
}
