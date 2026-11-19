// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174-AC-7 (TC-3011): a key symmetry and a state constraint selected on an
//! HP-2 item that FR-174's hyper reduction eligibility rule admits are
//! executed, keep the verdict and report `HoldsReduced`; a "yes" selection on
//! an ineligible form settles V-8 before any expansion.

use filament_exact::Cancel;
use fsl_foundation::ReductionKind;
use fsl_replay::{AppliedReduction, HyperOutcome, HyperReductionCause, ModelCheckLimits, RunStats};
use fsl_route::reductions::{Reductions, SymmetryDeclaration};
use fsl_semantics::check::HyperForm;
use ix_trace_rs::trace;

use crate::hyper::refuted;
use crate::hyper_world::{key, non_interference, opaque, Vault, World, EQUAL_INPUTS};

/// The two vaults `v` and `w` of one initial state.
const TWO_VAULTS: [(&str, (i64, i64)); 2] = [("v", (0, 0)), ("w", (1, 0))];

/// The key symmetry of `vaults` over `v` and `w`.
fn symmetry() -> Reductions {
    Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: key("vaults"),
            classes: vec![vec!["v".to_owned(), "w".to_owned()]],
        }],
        ..Reductions::default()
    }
}

/// [`symmetry`] with the state constraint `constraint` also selected.
fn with_constraint(reductions: Reductions, constraint: &str) -> Reductions {
    Reductions {
        constraint: Some(constraint.to_owned()),
        ..reductions
    }
}

/// `NonInterference` with the state constraints `Low` (met by every reachable
/// state) and `Zero` (cut on the first step that sets `l`) declared beside it.
fn interference(vault: Vault) -> World {
    let declaration = format!(
        "{}\n\
         constraint Low using v on V::Vault at current {{ self.l <= 1 }}\n\
         constraint Zero using v on V::Vault at current {{ self.l = 0 }}",
        non_interference(EQUAL_INPUTS)
    );
    World::with_vaults(vault, &TWO_VAULTS, &declaration)
}

fn product_states(outcome: &HyperOutcome) -> u64 {
    match outcome {
        HyperOutcome::Holds { stats } | HyperOutcome::HoldsReduced { stats, .. } => {
            stats.product_states as u64
        }
        other => panic!("expected a holding outcome, got {other:?}"),
    }
}

#[trace("TC-3011", "FR-174-AC-7")]
#[test]
#[ignore = "QSL-2248: key symmetry with the diagonal action on an HP-2 product not implemented yet"]
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

    let leaky = interference(Vault::Leaky);
    let counterexample = refuted(leaky.run_selecting(symmetry()).expect("not refused"));
    assert_eq!(counterexample.traces.len(), 2);
}

#[trace("TC-3011", "FR-174-AC-7")]
#[test]
#[ignore = "QSL-2248: a state constraint on an HP-2 product not implemented yet"]
fn a_state_constraint_reduces_and_holds_reduced() {
    let secure = interference(Vault::Secure);
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

#[trace("TC-3011", "FR-174-AC-7")]
#[test]
#[ignore = "QSL-2248: the hyper reduction eligibility rule not implemented yet"]
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

/// The constraint `Low` declared after `declaration`.
fn with_low(declaration: &str) -> String {
    format!("{declaration}\nconstraint Low using v on V::Vault at current {{ self.l <= 1 }}")
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
#[ignore = "QSL-2248: a state constraint on an HP-2 product over two subjects or under each fairness not implemented yet"]
fn a_state_constraint_is_eligible_on_every_hp2_item() {
    let power = World::with_second_model(
        Vault::Secure,
        &crate::hyper_world::STARTS,
        &with_low(
            "hyper SavesPower using inf over behaviours (d: V::Vault, e: N::Vault) \
             forall trace s of V forall trace n of N \
             match { s.step.Vault::step.i = n.step.Vault::step.i } \
             { always holds(d.l @ s = e.l @ n) }",
        ),
    );
    let each = World::new(
        Vault::Secure,
        false,
        &crate::hyper_world::STARTS,
        &with_low(
            "hyper P using inf over behaviours (v: V::Vault) \
             forall trace a of V fair weak each V::Vault::step; \
             forall trace b of V fair weak each V::Vault::step; \
             match { a.step.op = b.step.op } { always holds(v.l @ a = v.l @ b) }",
        ),
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
            .run(ModelCheckLimits::default(), &Cancel::new())
            .expect("not refused");
        assert!(
            same_verdict(&constrained, &unconstrained),
            "{constrained:?} against {unconstrained:?}"
        );
    }
}

/// FR-174-AC-7: a request that selects a row-preserving key symmetry is
/// admitted by the selector, then settles `unsupported` before any expansion.
#[trace("TC-599", "FR-174-AC-7")]
#[test]
fn a_selected_preserving_reduction_is_reported_before_expansion() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let selected = Reductions {
        symmetry: vec![fsl_route::reductions::SymmetryDeclaration {
            population: crate::hyper_world::key("vaults"),
            classes: vec![vec!["v".to_owned(), "w".to_owned()]],
        }],
        ..Reductions::default()
    };
    let outcome = world
        .run_selecting(selected)
        .expect("the reduction selection parses");
    assert_eq!(
        outcome,
        HyperOutcome::Reduction {
            cause: HyperReductionCause::Unsupported {
                reduction: ReductionKind::Symmetry,
            },
            stats: RunStats::default(),
        }
    );
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
