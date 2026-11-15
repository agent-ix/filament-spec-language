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
#[ignore = "QSL-2013: reporting the first refuted instance over a shorter later counterexample is not verified yet"]
fn the_first_refuted_instance_is_reported_over_a_shorter_later_one() {
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
    let outcome = check_model(
        ModelCheckRequest {
            temporal_clock_selections: &[],
            subject: subject(&package, &documents, initial, &["a", "b"], UNLIMITED),
            item: ModelCheckItem::Clause {
                clause,
                letters: &letters,
            },
            max_depth: None,
            limits: ModelCheckLimits::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
    .expect("checked");
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("expected Violated, got {outcome:?}");
    };
    assert_eq!(counterexample.over_binding, Some(instance("a")));
    assert_eq!(counterexample.prefix.len(), 2);
}
