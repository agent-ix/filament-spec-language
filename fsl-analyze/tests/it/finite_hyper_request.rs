// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-898 finite trace populations through the native analyze request.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, counter_request_with, label, request_members, Workspace,
};

const POPULATION: &str = "population";

fn run_items(
    ws: &Workspace,
    source: &str,
    items: Value,
    traces: Value,
) -> crate::analyze_cli_support::Run {
    let mut request = request_members(Some(items));
    request["documents"] = json!([
        {"file": "start.json"},
        {"file": "population.json", "identity": {"authority": "test", "identity": POPULATION}}
    ]);
    let path = counter_request_with(ws, &source, request);
    ws.write("population.json", traces.to_string().as_bytes());
    let run = analyze(&path);
    assert!(
        run.item(0).is_object(),
        "a finite hyper request returns an item: {}",
        run.document()
    );
    run
}

fn request(ws: &Workspace, clause: &str, traces: Value) -> crate::analyze_cli_support::Run {
    let source = format!(
        "record Obs {{ inp: Int[0, 3]; out: Int[0, 3]; }}\n\
         profile ep = \"filament.temporal.event-position.false-extension/v1\";\n\
         {clause}\n"
    );
    run_items(
        ws,
        &source,
        json!([{
            "claim": "Det",
            "kind": "temporal-satisfaction",
            "traces": {"authority": "test", "identity": POPULATION}
        }]),
        traces,
    )
}

fn population(closure: &str, traces: Value) -> Value {
    json!({
        "format": "filament.trace-population/1",
        "record_type": "Obs",
        "closure": closure,
        "traces": traces,
    })
}

fn trace(identity: &str, states: &[(u64, u64)]) -> Value {
    json!({
        "identity": identity,
        "states": states.iter().map(|(inp, out)| json!({"inp": inp, "out": out})).collect::<Vec<_>>(),
    })
}

const DET: &str = "hyper Det using ep over t in Obs bounded 3 \
    forall trace a forall trace b { (a.inp = b.inp) implies (a.out = b.out) }";

/// FR-898-AC-3: the checked clause settles closed trace populations.
#[trace("TC-2535", "FR-898-AC-3")]
#[test]
fn closed_population_proves_and_replays_a_joint_position_refutation() {
    let ws = Workspace::new("finite-hyper-closed");
    let held = request(
        &ws,
        DET,
        population(
            "closed",
            json!([
                trace("a", &[(0, 0), (1, 1)]),
                trace("b", &[(0, 0), (1, 1)]),
                trace("c", &[(2, 2), (3, 3)]),
            ]),
        ),
    );
    assert_eq!(label(&held.item(0)), "proved");

    let ws = Workspace::new("finite-hyper-refuted");
    let refuted = request(
        &ws,
        DET,
        population(
            "closed",
            json!([
                trace("a", &[(0, 0), (1, 0)]),
                trace("b", &[(0, 0), (1, 1)]),
                trace("c", &[(2, 2), (3, 3)]),
            ]),
        ),
    );
    let item = refuted.item(0);
    assert_eq!(label(&item), "refuted");
    assert_eq!(item["replay"]["reproduced"], true);
    assert_eq!(item["replay"]["target"]["hyperproperty"], "Det");
    assert!(item["counterexample"]["false_tuples"]
        .as_array()
        .is_some_and(|tuples| {
            tuples
                .iter()
                .any(|tuple| tuple["traces"] == json!(["a", "b"]) && tuple["position"] == 1)
        }));
}

/// FR-898-AC-4: quantifier order matters; an open population never proves,
/// and exceeding the declared population bound is inconclusive.
#[trace("TC-2535", "FR-898-AC-4")]
#[test]
fn quantifier_order_open_prefix_and_population_bound_settle() {
    let clauses = "hyper ForallExists using ep over t in Obs bounded 3 \
         forall trace a exists trace b { a.out = b.inp }\n\
         hyper ExistsForall using ep over t in Obs bounded 3 \
         exists trace b forall trace a { a.out = b.inp }";
    let ws = Workspace::new("finite-hyper-prefix-order");
    let source = format!(
        "record Obs {{ inp: Int[0, 3]; out: Int[0, 3]; }}\n\
         profile ep = \"filament.temporal.event-position.false-extension/v1\";\n\
         {clauses}\n"
    );
    let items = json!([
        {"claim": "ForallExists", "kind": "temporal-satisfaction", "traces": {"authority": "test", "identity": POPULATION}},
        {"claim": "ExistsForall", "kind": "temporal-satisfaction", "traces": {"authority": "test", "identity": POPULATION}}
    ]);
    let ordered = run_items(
        &ws,
        &source,
        items,
        population(
            "closed",
            json!([trace("a", &[(0, 1)]), trace("b", &[(1, 0)]),]),
        ),
    );
    assert_eq!(label(&ordered.item(0)), "proved");
    assert_eq!(label(&ordered.item(1)), "refuted");

    let ws = Workspace::new("finite-hyper-open");
    let open = request(
        &ws,
        DET,
        population(
            "open",
            json!([trace("a", &[(0, 0)]), trace("b", &[(0, 0)])]),
        ),
    );
    assert_eq!(label(&open.item(0)), "inconclusive");

    let ws = Workspace::new("finite-hyper-bound");
    let over_bound = request(
        &ws,
        DET,
        population(
            "closed",
            json!([
                trace("a", &[(0, 0)]),
                trace("b", &[(0, 0)]),
                trace("c", &[(1, 1)]),
                trace("d", &[(1, 1)])
            ]),
        ),
    );
    assert_eq!(label(&over_bound.item(0)), "inconclusive");
}

/// FR-898-AC-5: a tuple missing an aligned state remains inconclusive.
#[trace("TC-2535", "FR-898-AC-5")]
#[test]
fn missing_aligned_record_is_inconclusive() {
    let ws = Workspace::new("finite-hyper-incomplete");
    let clause = "hyper Det using ep over t in Obs bounded 3 \
        forall trace a forall trace b { a.out = b.out }";
    let run = request(
        &ws,
        clause,
        population(
            "closed",
            json!([trace("a", &[(0, 0)]), trace("b", &[(0, 0), (1, 1)]),]),
        ),
    );
    assert_eq!(label(&run.item(0)), "inconclusive");
}
