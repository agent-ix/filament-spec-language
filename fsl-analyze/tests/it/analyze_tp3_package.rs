// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338-AC-30 and FR-338-AC-31 through `analyze`: a TP-3 clause under an
//! `on each` activation, and a TP-3 clause over a protocol subject, settle
//! from the checked package the compile writes as they settle from source,
//! with the same value and certification, because the core checks the
//! certificate on each route.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, counter_request, label, package_selection, request_over, Run, Workspace,
};
use crate::counter::clause_text_on;

/// `request`'s members run from source and, after the compile writes its
/// handoff and the source is removed, from the package it selects.
fn both_routes(ws: &Workspace, request: &std::path::Path) -> (Run, Run) {
    let source = analyze(request);
    let document: Value =
        serde_json::from_slice(&std::fs::read(request).expect("the request")).expect("JSON");
    let (members, _) = package_selection(ws, &document["request"]);
    let package_request = json!({"format": "native-analyze/1", "request": members});
    let path = ws.write("analyze-package.json", package_request.to_string().as_bytes());
    std::fs::remove_file(ws.path("program.fsl")).expect("the source is removed before the package run");
    let package = analyze(&path);
    (source, package)
}

fn assert_same_settlement(source: &Run, package: &Run) {
    for member in ["items", "aggregate", "last_stage", "category"] {
        assert_eq!(
            package.document()[member],
            source.document()[member],
            "{member}: {}",
            package.document()
        );
    }
    assert_eq!(package.exit, source.exit, "{}", package.document());
}

/// FR-338-AC-30 over the CLI: the unguarded `on each` clause whose activation
/// at value 1 reads value 0 one position back settles `proved`, `certified`
/// from source and from the package; the clause whose activation at value 2
/// reads a previous value 1 is refuted on both routes.
#[trace("TC-525", "FR-338-AC-30")]
#[trace("TC-2920", "FR-1030-AC-11")]
#[test]
fn an_on_each_tp3_claim_settles_from_the_package_as_from_source() {
    let each = "on each (q: M::Counter)";
    let body = clause_text_on(
        "PrevSafeEach",
        each,
        "holds(c.value = 1) implies once[1,1] holds(c.value = 0)",
    ) + &clause_text_on(
        "PrevJumpEach",
        each,
        "holds(c.value = 2) implies once[1,1] holds(c.value = 0)",
    );
    let ws = Workspace::new("tp3-each-package");
    let path = counter_request(
        &ws,
        &body,
        json!([{"claim": "PrevSafeEach"}, {"claim": "PrevJumpEach"}]),
    );
    let (source, package) = both_routes(&ws, &path);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_eq!(
        source.item(0)["record"]["certification"],
        "certified",
        "{}",
        source.document()
    );
    assert_eq!(label(&source.item(1)), "refuted", "{}", source.document());
    assert_eq!(label(&package.item(0)), "proved", "{}", package.document());
    assert_eq!(
        package.item(0)["record"]["certification"],
        "certified",
        "{}",
        package.document()
    );
    assert_eq!(label(&package.item(1)), "refuted", "{}", package.document());
    assert_same_settlement(&source, &package);
}

fn fill_subject() -> Value {
    json!({"protocol": {"protocol": "Fill",
        "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
        "roles": {"w": {"type": "model-binding", "selection": "test/fill"}}}})
}

/// FR-338-AC-31 over the CLI: the repaired `Fill`'s past clause `not
/// once[1,2]` and its future clause `eventually[0,2]` settle `proved`,
/// `certified` from source and from the package, and the clause whose
/// `eventually[0,1]` window no behaviour meets is refuted on both routes.
#[trace("TC-525", "FR-338-AC-31")]
#[test]
fn a_protocol_tp3_claim_settles_from_the_package_as_from_source() {
    let clause = |name: &str, formula: &str| {
        format!(
            "temporal {name} using t0 over (k: M::Cell) clock \"model-steps\" on origin \
             {{ {formula} }}\n"
        )
    };
    let body = crate::fill::fill_unit("self.v <= 1", "")
        + &clause(
            "Past",
            "always (holds(k.v = 0) implies not once[1,2] holds(k.v >= 1))",
        )
        + &clause(
            "Soon",
            "always (holds(k.v = 0) implies eventually[0,2] holds(k.v >= 2))",
        )
        + &clause(
            "TooSoon",
            "always (holds(k.v = 0) implies eventually[0,1] holds(k.v = 3))",
        );
    let ws = Workspace::new("tp3-protocol-package");
    let snapshots: Vec<Vec<u8>> = {
        let mut documents = crate::fill::Documents::new();
        documents.start("s0", &[("c", 0)]);
        documents.snapshots.into_values().collect()
    };
    let path = request_over(
        &ws,
        &crate::fill::unit_text(&body),
        &crate::fill::document(),
        &snapshots,
        json!({"subject": fill_subject(),
            "items": [{"claim": "Past"}, {"claim": "Soon"}, {"claim": "TooSoon"}]}),
    );
    let (source, package) = both_routes(&ws, &path);
    for (index, expected) in [(0, "proved"), (1, "proved"), (2, "refuted")] {
        assert_eq!(
            label(&source.item(index)),
            expected,
            "{}",
            source.document()
        );
        assert_eq!(
            label(&package.item(index)),
            expected,
            "{}",
            package.document()
        );
    }
    for index in [0, 1] {
        assert_eq!(
            source.item(index)["record"]["certification"],
            "certified",
            "{}",
            source.document()
        );
        assert_eq!(
            package.item(index)["record"]["certification"],
            "certified",
            "{}",
            package.document()
        );
    }
    assert_same_settlement(&source, &package);
}
