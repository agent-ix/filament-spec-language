// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-125 and FSpec FR-361-AC-10: a refuted item with several instances
//! reports the first refuted instance in canonical instance order, with that
//! instance's counterexample, even when a later instance has a shorter one
//! (ADR-018 §6 states the same rule).

use filament_exact::Cancel;
use fsl_analyze::model_check::{check_model, ModelCheckRequest};
use fsl_replay::{ModelCheckItem, ModelCheckLimits, ModelCheckOutcome};
use ix_trace_rs::trace;

use crate::counter::{
    document, instance, package_with_clauses, subject, Documents, Letters, CONTRACT, UNLIMITED,
};

/// Over the Counter subject with `a` at 0 and `b` at 3 and the universe
/// `[a, b]`, `always holds(c.value < 2)` is refuted at `a` after two `inc`
/// steps and at `b` at position 0. The reported instance is `a`, the first
/// refuted in instance order, with its two-step counterexample, not `b`'s
/// shorter one.
#[trace("TC-520", "FR-125-AC-16")]
#[trace("FSpec-TC-290", "FSpec-FR-361-AC-10")]
#[test]
fn the_first_refuted_instance_is_reported_over_a_shorter_later_one() {
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome_under(ModelCheckLimits::default())
    else {
        panic!("expected Violated");
    };
    assert_eq!(counterexample.over_binding, Some(instance("a")));
    assert_eq!(counterexample.prefix.len(), 2);
}

/// FR-125, FR-126: the search over the instances before the one first found
/// refuted is charged to the same budget as the first search. `b` is found at
/// position 0 with one state, so `max_states` 3 leaves two states for the
/// search that must reach `a`'s three-state refutation: the item stops
/// incomplete on `max_states`, and reports no instance.
#[trace("TC-520", "FR-125-AC-16")]
#[test]
fn the_search_for_an_earlier_refuted_instance_shares_the_budget() {
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 3;
    let outcome = outcome_under(limits);
    assert!(
        matches!(outcome, ModelCheckOutcome::Stopped { .. }),
        "expected a stop, got {outcome:?}"
    );
}

/// FR-125: a search over the earlier instances that reaches its horizon without
/// deciding them leaves the item incomplete; the later instance `b`, found at
/// position 0, is not reported as the first refuted one. With `max_depth` 1,
/// `a`'s two-step refutation is out of reach.
#[trace("TC-520", "FR-125-AC-16")]
#[test]
fn an_undecided_search_for_an_earlier_instance_is_not_reported_as_refuted() {
    let outcome = outcome_with(ModelCheckLimits::default(), Some(1), Default::default());
    assert!(
        matches!(outcome, ModelCheckOutcome::BoundReached { .. }),
        "expected BoundReached, got {outcome:?}"
    );
}

/// FR-125 on the reduced route: partial-order reduction reports the first
/// refuted instance, as the unreduced route does.
#[trace("TC-520", "FR-125-AC-16")]
#[test]
fn a_reduced_check_reports_the_first_refuted_instance() {
    let reductions = fsl_route::reductions::Reductions {
        partial_order: true,
        ..Default::default()
    };
    let outcome = outcome_with(ModelCheckLimits::default(), None, reductions);
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("expected Violated, got {outcome:?}");
    };
    assert_eq!(counterexample.over_binding, Some(instance("a")));
}

fn outcome_under(limits: ModelCheckLimits) -> ModelCheckOutcome {
    outcome_with(limits, None, Default::default())
}

fn outcome_with(
    limits: ModelCheckLimits,
    max_depth: Option<usize>,
    reductions: fsl_route::reductions::Reductions,
) -> ModelCheckOutcome {
    let mut documents = Documents::new(document(false));
    let initial = vec![documents.start("s0", &[("a", 0), ("b", 3)])];
    let (package, clauses) = package_with_clauses(
        &documents.document,
        CONTRACT,
        &[("C", "", "always holds(c.value < 2)")],
    );
    let [clause] = clauses.as_slice() else {
        panic!("one declared clause");
    };
    let letters = Letters {
        atom: |value| Some(value < 2),
    };
    check_model(
        ModelCheckRequest {
            temporal_clock_selections: &[],
            subject: subject(&package, &documents, initial, &["a", "b"], UNLIMITED),
            item: ModelCheckItem::Clause {
                clause,
                letters: &letters,
            },
            max_depth,
            limits,
            reductions,
        },
        &Cancel::new(),
    )
    .expect("checked")
}
