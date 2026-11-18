// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174 (TC-599): a hyper check receives and settles the request's
//! reduction selection before expanding a product.

use fsl_foundation::ReductionKind;
use fsl_replay::{HyperOutcome, HyperReductionCause, RunStats};
use fsl_route::reductions::Reductions;
use fsl_semantics::check::HyperForm;
use ix_trace_rs::trace;

use crate::hyper_world::{non_interference, World, EQUAL_INPUTS, STARTS, Vault};

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
