// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-249-AC-7 to AC-9 (TC-3008): a task automaton lowers to a timed subject
//! that carries the declared periods and deadlines, EN-6's verdict on it
//! equals EN-7's response-time verdict on Buttazzo's task set, and an
//! unschedulable set refutes with a replayed counterexample whose first
//! deadline miss is exact. The source is ADR-026 section 11's `Rpc` with one
//! always-enabled `fire_*` operation per task, each releasing its task.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, request_over, Run, Workspace};
use crate::analyze_family_cli::{rpc_snapshot, rpc_unit};

/// One always-enabled operation per task, each releasing that task of `set`.
fn sources(set: &str, tasks: &[&str]) -> String {
    tasks
        .iter()
        .map(|task| {
            format!(
                "post Fire_{task} using v on M::Call::fire_{task} {{ self.phase = pre(self.phase) }}\n\
                 release {set}.{task} on M::Call::fire_{task};\n"
            )
        })
        .collect()
}

fn run(name: &str, tasks: &[&str], declarations: &str, items: Value) -> Run {
    let ws = Workspace::new(name);
    let operations: Vec<(String, &[&str])> = tasks
        .iter()
        .map(|task| (format!("fire_{task}"), &["phase"][..]))
        .collect();
    let extra: Vec<(&str, &[&str])> = operations
        .iter()
        .map(|(name, fields)| (name.as_str(), *fields))
        .collect();
    analyze(&request_over(
        &ws,
        &rpc_unit(4, declarations),
        &crate::zone_search::rpc_document_with(&extra),
        &[rpc_snapshot()],
        json!({"items": items}),
    ))
}

/// `Pair`: `a` periodic 4 ms, deadline 3 ms; `b` sporadic 6 ms, deadline
/// 5 ms; both fixed-work.
const PAIR: &str = "taskset Pair {\n\
     task a periodic 4 ms deadline 3 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
     task b sporadic 6 ms deadline 5 ms bcet 2 ms wcet 2 ms priority 2 lo;\n\
     }\n\
     schedulable PairFp on Pair under fixed_priority;\n";

/// Buttazzo's response-time example, `(C, T = D)` of `(1, 4)`, `(2, 6)` and
/// `(third, 10)` ms, priorities by period, every task fixed-work.
fn buttazzo(third: u32) -> String {
    format!(
        "taskset B {{\n\
         task t1 periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
         task t2 periodic 6 ms deadline 6 ms bcet 2 ms wcet 2 ms priority 2 lo;\n\
         task t3 periodic 10 ms deadline 10 ms bcet {third} ms wcet {third} ms priority 3 lo;\n\
         }}\n\
         schedulable BFp on B under fixed_priority;\n\
         schedulable BEdf on B under edf;\n{}",
        sources("B", &["t1", "t2", "t3"])
    )
}

fn rational(value: &Value) -> (String, String) {
    (
        value["numerator"].as_str().unwrap_or("-").to_owned(),
        value["denominator"].as_str().unwrap_or("-").to_owned(),
    )
}

fn whole(value: i64) -> (String, String) {
    (value.to_string(), "1".to_owned())
}

#[trace("TC-3008", "FR-249-AC-7")]
#[test]
#[ignore = "QSL-2240: lowering a task automaton to an EN-6 subject carrying its task timing not implemented yet"]
fn a_two_task_set_lowers_with_its_declared_periods_and_deadlines() {
    let declarations = format!("{PAIR}{}", sources("Pair", &["a", "b"]));
    let run = run(
        "task-product-timing",
        &["a", "b"],
        &declarations,
        json!([{"claim": "PairFp", "backend": "en-6"}]),
    );
    let item = run.item(0);
    assert_eq!(
        item["request"]["candidates"]["backends"],
        json!(["en-7", "en-6"])
    );
    assert_eq!(label(&item), "proved", "{item}");
    let rows = item["certificate"]["task_product"]["queue_bounds"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let read: Vec<_> = rows
        .iter()
        .map(|row| {
            (
                row["task"].as_str().unwrap_or("-").to_owned(),
                row["arrival"].as_str().unwrap_or("-").to_owned(),
                rational(&row["interarrival"]),
                rational(&row["relative_deadline"]),
                rational(&row["work_lower"]),
                rational(&row["work_upper"]),
                row["max_jobs"].clone(),
            )
        })
        .collect();
    assert_eq!(
        read,
        vec![
            (
                "a".to_owned(),
                "periodic".to_owned(),
                whole(4),
                whole(3),
                whole(1),
                whole(1),
                json!(2)
            ),
            (
                "b".to_owned(),
                "sporadic".to_owned(),
                whole(6),
                whole(5),
                whole(2),
                whole(2),
                json!(2)
            ),
        ],
        "{item}"
    );
    // Without a release the claim keeps EN-7 alone.
    let bare = run_bare();
    assert_eq!(bare["request"]["candidates"]["backends"], json!(["en-7"]));
}

fn run_bare() -> Value {
    run("task-product-bare", &[], PAIR, json!([{"claim": "PairFp"}])).item(0)
}

#[trace("TC-3008", "FR-249-AC-8")]
#[test]
#[ignore = "QSL-2240: EN-6 deciding a task automaton, agreeing with EN-7's response-time verdict, not implemented yet"]
fn the_zone_verdict_equals_the_response_time_verdict_on_buttazzo() {
    let items = json!([
        {"claim": "BFp"}, {"claim": "BFp", "backend": "en-6"},
        {"claim": "BEdf"}, {"claim": "BEdf", "backend": "en-6"},
    ]);
    for (third, expected) in [(3, ["proved", "proved"]), (4, ["refuted", "proved"])] {
        let run = run(
            &format!("buttazzo-{third}"),
            &["t1", "t2", "t3"],
            &buttazzo(third),
            items.clone(),
        );
        for (claim, want) in expected.iter().enumerate() {
            let closed = run.item(2 * claim);
            let zone = run.item(2 * claim + 1);
            assert_eq!(label(&closed), *want, "EN-7, t3 work {third}: {closed}");
            assert_eq!(
                label(&zone),
                label(&closed),
                "EN-6 = EN-7, t3 work {third}: {zone}"
            );
        }
    }
}

#[trace("TC-3008", "FR-249-AC-9")]
#[test]
#[ignore = "QSL-2240: a replayed task-derived counterexample naming its first deadline miss not implemented yet"]
fn an_unschedulable_set_refutes_with_its_first_missed_deadline() {
    let run = run(
        "buttazzo-miss",
        &["t1", "t2", "t3"],
        &buttazzo(4),
        json!([{"claim": "BFp", "backend": "en-6"}]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true);
    let prefix = item["counterexample"]["prefix"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let misses: Vec<&Value> = prefix
        .iter()
        .map(|step| &step["transition"])
        .filter(|identity| {
            identity["type"] == "task-transition" && identity["action"] == "deadline-miss"
        })
        .collect();
    let first = misses.first().expect("a deadline-miss step");
    assert_eq!(first["task"], "t3");
    assert_eq!(first["job"], 0);
    assert_eq!(rational(&first["demand"]), whole(4));
    // Least-delay concretisation: every task activates at 0, so t3's
    // absolute deadline is 10.
    assert_eq!(
        rational(&first["deadline"]),
        whole(10),
        "{}",
        item["counterexample"]
    );
}
