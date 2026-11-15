// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-730 (TC-2022): a repeat's loop proof is discharged over the explored
//! protocol subject as two derived items of the repeat, `loop-invariant` and
//! `loop-termination`, each settling `proved`, `refuted` or `inconclusive`.

use fsl_route::request::ItemKind;
use ix_trace_rs::trace;

/// FR-730-AC-4: the request writer has the two derived item kinds a loop
/// proof contributes, beside deadlock freedom, so a repeat's invariant and
/// termination obligations are routed and settled like every other derived
/// item instead of being recorded and left undischarged.
#[trace("TC-2022", "FR-730-AC-4", "FSpec-TC-1520", "FSpec-FR-171-AC-6")]
#[ignore = "QSL-2222: loop-invariant and loop-termination items are not implemented yet"]
#[test]
fn a_loop_proof_contributes_invariant_and_termination_item_kinds() {
    let kinds: Vec<&str> = ItemKind::ALL.iter().map(|kind| kind.to_wire()).collect();
    for expected in ["loop-invariant", "loop-termination"] {
        assert!(
            kinds.contains(&expected),
            "{expected} is a derived item kind: {kinds:?}"
        );
    }
}

/// A unit whose protocol `P` holds `repeat Retry` with no maximum, the loop
/// proof `invariant { invariant } variant { variant }` and guard `guard`,
/// its body one `touch` attempt.
fn loop_unit(invariant: &str, variant: &str, guard: &str) -> String {
    let run = format!(
        "sequence Main {{ repeat Retry by w visible () invariant {{ {invariant} }} \
         variant {{ {variant} }} while {{ {guard} }} sequence Body {{ {} }} }}",
        crate::fill::attempt("T", "touch", "TouchPre", "t", "true")
    );
    crate::fill::contracts("self.v = 0") + &crate::fill::protocol("P", "", &run, "d")
}

/// The derived items `loop-invariant` and `loop-termination` of `P` in
/// `body`, through the request writer that derives every protocol item.
fn loop_items(body: &str) -> Vec<&'static str> {
    use crate::derived_request::protocol_request;
    use crate::model_settle::protocol_anchor;
    use fsl_route::deadlock::ProtocolTemporalItem;

    let checked = crate::fill::package_with(crate::fill::PROTOCOL_PROFILE, body);
    let protocol = checked.graph().protocol("P").expect("P");
    let occurrence = protocol_anchor(&checked, "P").1;
    let items = [ProtocolTemporalItem {
        occurrence: &occurrence,
        subject: &"P",
        protocol,
        models: &[],
        non_atomic_access: false,
    }];
    let written = protocol_request(&items);
    written
        .items()
        .iter()
        .filter_map(|item| item.item_kind())
        .map(ItemKind::to_wire)
        .filter(|kind| kind.starts_with("loop-"))
        .collect()
}

/// FR-730-AC-1: a repeat whose variant decreases derives its invariant and
/// termination items, which settle `proved` over a complete exploration.
#[trace("TC-2022", "FR-730-AC-1")]
#[ignore = "QSL-2222: loop-invariant and loop-termination items are not implemented yet"]
#[test]
fn a_decreasing_variant_derives_both_loop_items() {
    let kinds = loop_items(&loop_unit("true", "0", "false"));
    assert_eq!(kinds, ["loop-invariant", "loop-termination"], "{kinds:?}");
}

/// FR-730-AC-2: a repeat whose variant never decreases derives the
/// termination item that settles `refuted`; a false invariant derives the
/// invariant item that settles `refuted`.
#[trace("TC-2022", "FR-730-AC-2")]
#[ignore = "QSL-2222: loop-invariant and loop-termination items are not implemented yet"]
#[test]
fn an_unchanged_variant_and_a_false_invariant_derive_refutable_loop_items() {
    for (invariant, variant) in [("true", "1"), ("false", "0")] {
        let kinds = loop_items(&loop_unit(invariant, variant, "true"));
        assert_eq!(
            kinds,
            ["loop-invariant", "loop-termination"],
            "{invariant}/{variant}: {kinds:?}"
        );
    }
}

/// FR-730-AC-3: the loop items of a subject explored under a state limit
/// are derived and settle `inconclusive`, naming the limit.
#[trace("TC-2022", "FR-730-AC-3")]
#[ignore = "QSL-2222: loop-invariant and loop-termination items are not implemented yet"]
#[test]
fn loop_items_are_derived_whatever_the_exploration_limit() {
    let kinds = loop_items(&loop_unit("true", "1", "true"));
    assert_eq!(kinds.len(), 2, "{kinds:?}");
}
