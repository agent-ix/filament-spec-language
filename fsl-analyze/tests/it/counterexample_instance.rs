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
#[test]
fn the_first_refuted_instance_is_reported_over_a_shorter_later_one() {
    let ModelCheckOutcome::Violated { counterexample, .. } =
        outcome_under(ModelCheckLimits::default())
    else {
        panic!("expected Violated");
    };
    assert_eq!(counterexample.over_binding, Some(instance("a")));
    assert_eq!(counterexample.prefix.len(), 2);
}

/// FR-125, FR-126-AC-28: a limit does not discard a refutation the run
/// found. `b` is found at position 0 with one state; `max_states` 3 leaves
/// two states for the search that must reach `a`'s three-state refutation,
/// which stops on `max_states`. The refutation found within the budget is the
/// item's outcome: bound to `b`, not `Stopped`.
#[trace("TC-520", "FR-125-AC-16")]
#[test]
fn the_search_for_an_earlier_refuted_instance_shares_the_budget() {
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 3;
    let outcome = outcome_under(limits);
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("expected the refutation found within the budget, got {outcome:?}");
    };
    assert_eq!(counterexample.over_binding, Some(instance("b")));
}

/// FR-125, FR-126-AC-27: a search over the earlier instances that reaches its
/// horizon without deciding them does not discard the later instance's
/// refutation. `b` is found at position 0; with `max_depth` 1, `a`'s two-step
/// refutation is out of reach, so the item is bound to `b`.
#[trace("TC-520", "FR-125-AC-16")]
#[test]
fn an_undecided_search_for_an_earlier_instance_does_not_discard_a_later_refutation() {
    let outcome = outcome_with(ModelCheckLimits::default(), Some(1), Default::default());
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("expected the refutation within the horizon, got {outcome:?}");
    };
    assert_eq!(counterexample.over_binding, Some(instance("b")));
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
            workers: 1,
            temporal_clock_selections: &[],
            subject: subject(&package, &documents, initial, &["a", "b"], UNLIMITED),
            item: ModelCheckItem::Clause {
                clause,
                letters: &letters,
            },
            max_depth,
            limits,
            smt_encoding: Default::default(),
            smt_run: Default::default(),
            reductions,
        },
        &Cancel::new(),
    )
    .expect("checked")
}
