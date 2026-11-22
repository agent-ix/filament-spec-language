// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850 "Equality with the source route", FR-940-AC-13 (QSL-2767): a
//! `value-validity` item of a `native-analyze/1` request that selects a v2
//! checked package settles over the package's own rows exactly as the item
//! of the request over the source settles, and never settles `unsupported`
//! for a fact the source route reads.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, bounded, counter_request, label, package_selection, request_members, Workspace,
};

/// A Boolean value clause over a bounded integer that holds everywhere.
const SMALL: &str = "function Small using v(n: Int[0, 9]): Boolean pure { n * 2 - n = n }\n";
/// A Boolean value clause the check refutes at `n = 5`.
const LOW: &str = "function Low using v(n: Int[0, 9]): Boolean pure { n < 5 }\n";
/// A scalar operation whose result bound `x + 1` leaves at `x = 8`.
const NARROW: &str = "function Narrow using v(x: Int[0, 9]): Int[0, 8] pure { x + 1 }\n";
/// A scalar operation that stays inside its result bound everywhere.
const WIDE: &str = "function Wide using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }\n";

/// The source route's document for `items` over `declarations`, the
/// package route's document for the same request, and the package bytes.
fn both(name: &str, declarations: &str, items: &Value) -> (Value, i32, Value, i32) {
    let body = format!("{declarations}{}", bounded());
    let ws = Workspace::new(name);
    let source = analyze(&counter_request(&ws, &body, items.clone()));
    let (members, _) = package_selection(&ws, &request_members(Some(items.clone())));
    let document = json!({"format": "native-analyze/1", "request": members});
    let package = analyze(&ws.write("analyze-package.json", document.to_string().as_bytes()));
    (
        source.document(),
        source.exit,
        package.document(),
        package.exit,
    )
}

/// The package route's items and exit equal the source route's.
fn assert_parity(name: &str, declarations: &str, items: Value) -> Value {
    let (source, source_exit, package, package_exit) = both(name, declarations, &items);
    assert_eq!(package["items"], source["items"], "{package}");
    assert_eq!(package["registry"], source["registry"], "{package}");
    assert_eq!(package_exit, source_exit, "{package}");
    package["items"][0].clone()
}

/// FR-940-AC-13: a Boolean value clause that holds settles `proved` over
/// the package route as it does over the source.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn a_package_value_clause_that_holds_equals_the_source() {
    let item = assert_parity(
        "package-value-holds",
        SMALL,
        json!([{"claim": "Small", "kind": "value-validity"}]),
    );
    assert_eq!(label(&item), "proved", "{item}");
}

/// FR-940-AC-13: a Boolean value clause the check refutes carries the
/// source route's counterexample and replay.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn a_package_value_clause_refutation_equals_the_source() {
    let item = assert_parity(
        "package-value-refutes",
        LOW,
        json!([{"claim": "Low", "kind": "value-validity"}]),
    );
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// FR-940-AC-13: a scalar operation whose result leaves its bound is
/// refuted, and one that stays inside it is proved, as over the source.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn a_package_scalar_operation_equals_the_source() {
    let item = assert_parity(
        "package-value-scalar-refutes",
        NARROW,
        json!([{"claim": "Narrow", "kind": "value-validity"}]),
    );
    assert_eq!(label(&item), "refuted", "{item}");
    let item = assert_parity(
        "package-value-scalar-holds",
        WIDE,
        json!([{"claim": "Wide", "kind": "value-validity"}]),
    );
    assert_eq!(label(&item), "proved", "{item}");
}

/// FR-940-AC-13: value items mixed with a temporal clause keep the request's
/// item order and every record.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn a_package_value_item_runs_beside_a_clause_item() {
    let declarations = format!("{SMALL}{LOW}");
    assert_parity(
        "package-value-mixed",
        &declarations,
        json!([{"claim": "Bounded"},
            {"claim": "Low", "kind": "value-validity"},
            {"claim": "Small", "kind": "value-validity"}]),
    );
}

/// FR-940 "never settles `unsupported` for a fact the source route reads": a
/// value clause over the mathematical `Integer` has no bounded candidate; the
/// package route settles the record the source route settles.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn a_package_value_item_with_no_candidate_equals_the_source() {
    let declarations = "function Even using v(n: Integer): Boolean pure { n * 2 - n = n }\n";
    let items = json!([{"claim": "Even", "kind": "value-validity"}]);
    let (source, source_exit, package, package_exit) =
        both("package-value-unbounded", declarations, &items);
    assert_eq!(package["items"], source["items"], "{package}");
    assert_eq!(package_exit, source_exit, "{package}");
}
