// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-627 (TC-1587): liveness items under weak and strong fairness through
//! `fsl analyze`. The fairness set is the checked clause's own, so a
//! request item names the clause and the document carries no fairness
//! (FR-620). The mutex is ADR-019 section 6's, over `Counter`'s ranged
//! `setTo` (`acquire`) and `inc` (`release`).

use ix_trace_rs::trace;
use serde_json::json;

use crate::analyze_cli_support::{
    analyze, clause, counter_request, counter_snapshot, label, request_over, Workspace,
};
use crate::counter::{clause_text, document_with_ranged_parameter, unit_text};

const MUTEX: &str =
    "pre Free using v on M::Counter::setTo { self.value = 0 and n >= 1 and n <= 2 }\n\
    post Takes using v on M::Counter::setTo { self.value = n }\n\
    pre Held using v on M::Counter::inc { self.value != 0 }\n\
    post Frees using v on M::Counter::inc { self.value = 0 }\n";

fn mutex_request(ws: &Workspace, clauses: &[(&str, &str)]) -> std::path::PathBuf {
    let mut body = MUTEX.to_owned();
    let mut items = Vec::new();
    for (name, fairness) in clauses {
        body += &clause_text(name, fairness, "always eventually holds(c.value = 1)");
        items.push(json!({"claim": name}));
    }
    request_over(
        ws,
        &unit_text(&body),
        &document_with_ranged_parameter(),
        &[counter_snapshot(&[("a", 0)])],
        json!({"items": items}),
    )
}

/// FR-627-AC-1: a clause with `fair weak M::Counter::inc` settles `proved`
/// with certification `certified`.
#[trace("TC-1587", "FR-627-AC-1")]
#[test]
fn a_weakly_fair_liveness_clause_proves_certified() {
    let ws = Workspace::new("live-weak-inc");
    let fair = clause(
        "Fair",
        "fair weak M::Counter::inc;",
        "eventually holds(c.value = 3)",
    );
    let run = analyze(&counter_request(&ws, &fair, json!([{"claim": "Fair"}])));
    let item = run.item(0);
    assert_eq!(label(&item), "proved");
    assert_eq!(item["record"]["certification"], "certified");
    assert_eq!(run.exit, 0);
}

/// FR-627-AC-2, AC-3: weak `each` refutes the mutex's liveness claim and
/// strong `each` proves it, in one request; the refuted lasso replays and
/// each of its constraints is checked fair on the cycle.
#[trace("TC-1587", "FR-627-AC-2")]
#[trace("TC-1587", "FR-627-AC-3")]
#[test]
fn weak_each_refutes_where_strong_each_proves() {
    let ws = Workspace::new("live-mutex");
    let run = analyze(&mutex_request(
        &ws,
        &[
            ("WeakEach", "fair weak each M::Counter::setTo;"),
            ("StrongEach", "fair strong each M::Counter::setTo;"),
        ],
    ));
    let weak = run.item(0);
    assert_eq!(label(&weak), "refuted");
    assert_eq!(weak["replay"]["reproduced"], true);
    assert!(weak["counterexample"]["loop"].is_array(), "{weak}");
    assert_eq!(weak["counterexample"]["fairness"][0]["kind"], "weak");
    assert_eq!(weak["counterexample"]["fairness"][0]["granularity"], "each");
    let strong = run.item(1);
    assert_eq!(label(&strong), "proved");
    assert_eq!(strong["record"]["certification"], "certified");
    assert_eq!(run.exit, 10);
}
