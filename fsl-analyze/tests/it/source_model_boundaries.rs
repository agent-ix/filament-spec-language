// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authored temporal bodies read real model behaviours through the public command.

use crate::analyze_cli_support::{Workspace, analyze, counter_snapshot, label, request_over};
use crate::counter::{CONTRACT, document, unit_text};
use ix_trace_rs::trace;
use serde_json::{Value, json};

struct OwnedWorkspace(Workspace);

impl OwnedWorkspace {
    fn new(name: &str) -> Self {
        Self(Workspace::new(name))
    }
}

impl Drop for OwnedWorkspace {
    fn drop(&mut self) {
        std::fs::remove_dir_all(self.0.path("")).expect("remove this test's own request files");
    }
}

fn request(
    ws: &Workspace,
    bounded: bool,
    declarations: &str,
    items: &[&str],
) -> std::path::PathBuf {
    let mut source = unit_text(&format!("{CONTRACT}{declarations}"));
    if bounded {
        source = source.replace(
            "filament.temporal.infinite-trace/v1",
            "filament.temporal.event-position.false-extension/v1",
        );
    }
    request_over(
        ws,
        &source,
        &document(false),
        &[counter_snapshot(&[("a", 0)])],
        json!({"items":items.iter().map(|name| json!({"claim":name})).collect::<Vec<_>>()}),
    )
}

fn values(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(members) => {
            if members.get("name").and_then(Value::as_str) == Some("value") {
                let scalar = &value["value"];
                if scalar["type"] == "integer" {
                    found.push(
                        scalar["value"]
                            .as_str()
                            .expect("canonical integer")
                            .to_owned(),
                    );
                }
            }
            for member in members.values() {
                values(member, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                values(item, found);
            }
        }
        _ => {}
    }
}

fn state_value(state: &Value) -> String {
    let mut found = Vec::new();
    values(state, &mut found);
    let [value] = found.as_slice() else {
        panic!("one Counter.value: {state}");
    };
    value.clone()
}

#[test]
#[trace("FR-126-AC-2")]
fn source_atoms_distinguish_sufficient_and_short_bounded_each_horizons() {
    let ws = OwnedWorkspace::new("qsl2090-each-horizons");
    let body = "temporal Enough using t0 over(c: M::Counter) clock \"model-steps\" on each(q: M::Counter) { eventually[0,3] holds(c.value = 3) }\ntemporal Short using t0 over(c: M::Counter) clock \"model-steps\" on each(q: M::Counter) { eventually[0,1] holds(c.value = 3) }";
    let run = analyze(&request(&ws.0, true, body, &["Enough", "Short"]));
    let enough = run.item(0);
    let short = run.item(1);
    assert_eq!(label(&enough), "proved", "{enough}");
    assert_eq!(enough["record"]["certification"], "certified", "{enough}");
    assert_eq!(label(&short), "refuted", "{short}");
    assert_eq!(short["replay"]["reproduced"], true, "{short}");
    let prefix = short["counterexample"]["prefix"]
        .as_array()
        .expect("a finite counterexample");
    assert_eq!(
        prefix.len(),
        1,
        "the independently known path is 0 -> 1: {short}"
    );
    assert_eq!(state_value(&prefix[0]["post_state"]["state"]), "1");
    assert_eq!(run.exit, 10);
}

#[test]
#[trace("FR-125-AC-2")]
fn a_bounded_state_predicate_is_false_past_the_terminal_closure() {
    let bounded_ws = OwnedWorkspace::new("qsl2090-false-extension");
    let body = "temporal ThroughFive using t0 over(c: M::Counter) clock \"model-steps\" on origin { always[0,5] holds(c.value <= 3) }";
    let bounded = analyze(&request(&bounded_ws.0, true, body, &["ThroughFive"]));
    let bounded_item = bounded.item(0);
    assert_eq!(label(&bounded_item), "refuted", "{bounded_item}");
    assert_eq!(bounded_item["replay"]["reproduced"], true, "{bounded_item}");
    assert_eq!(bounded.exit, 10);
}

#[test]
#[trace("FR-125-AC-2")]
fn a_bounded_operator_reads_true_over_the_infinite_terminal_stutter() {
    let infinite_ws = OwnedWorkspace::new("qsl2090-infinite-extension");
    let body = "temporal ThroughFive using t0 over(c: M::Counter) clock \"model-steps\" on origin { always[0,5] holds(c.value <= 3) }";
    let infinite = analyze(&request(&infinite_ws.0, false, body, &["ThroughFive"]));
    let infinite_item = infinite.item(0);
    assert_eq!(label(&infinite_item), "proved", "{infinite_item}");
    assert_eq!(
        infinite_item["record"]["certification"], "certified",
        "{infinite_item}"
    );
    assert_eq!(infinite.exit, 0);
}

#[test]
#[trace("FR-125-AC-1", "FR-125-AC-2", "FR-126-AC-3")]
fn a_real_terminal_lasso_repeats_three_with_a_distinct_stutter_transition() {
    let ws = OwnedWorkspace::new("qsl2090-terminal-stutter");
    let body = "temporal ReturnsToZero using t0 over(c: M::Counter) clock \"model-steps\" on origin { always eventually holds(c.value = 0) }";
    let run = analyze(&request(&ws.0, false, body, &["ReturnsToZero"]));
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
    let prefix = item["counterexample"]["prefix"]
        .as_array()
        .expect("a lasso stem");
    let cycle = item["counterexample"]["loop"]
        .as_array()
        .expect("a lasso cycle");
    assert_eq!(
        prefix
            .iter()
            .map(|step| state_value(&step["post_state"]["state"]))
            .collect::<Vec<_>>(),
        ["1", "2", "3"]
    );
    assert_eq!(cycle.len(), 1, "the terminal repeats once: {item}");
    assert_eq!(state_value(&cycle[0]["post_state"]["state"]), "3");
    assert_ne!(
        cycle[0]["transition"], prefix[2]["transition"],
        "stutter is not inc: {item}"
    );
    assert_eq!(run.exit, 10);
}

/// FR-125 and FR-091 against the Counter's own arithmetic: from `c = 0`,
/// `inc` runs to the deadlock at 3, so position `i` reads `min(i, 3)`. Past
/// that terminal state an infinite trace stutters it, and a bounded
/// (false-extension) trace reads every atom false. Each claim's label is
/// the one the closed forms give, never one the engine produced.
#[test]
#[trace("FR-125-AC-2")]
fn bounded_claims_settle_as_the_terminal_reading_of_their_profile_decides() {
    let mut body = String::new();
    let mut expected = Vec::new();
    for upper in [0_u32, 2, 3, 5] {
        for k in [1_u32, 3] {
            let always = format!("A{upper}x{k}");
            let eventually = format!("E{upper}x{k}");
            body.push_str(&format!(
                "temporal {always} using t0 over(c: M::Counter) clock \"model-steps\" on origin {{ always[0,{upper}] holds(c.value <= {k}) }}\n\
                 temporal {eventually} using t0 over(c: M::Counter) clock \"model-steps\" on origin {{ eventually[0,{upper}] holds(c.value = {k}) }}\n"
            ));
            let reach = upper.min(3);
            expected.push((always, reach <= k, upper <= 3 && upper <= k));
            expected.push((eventually, k <= reach, k <= reach));
        }
    }
    for (bounded, infinite) in [(false, true), (true, false)] {
        let ws = OwnedWorkspace::new(if bounded {
            "qsl2714-grid-b"
        } else {
            "qsl2714-grid-i"
        });
        let names: Vec<&str> = expected.iter().map(|(name, _, _)| name.as_str()).collect();
        let run = analyze(&request(&ws.0, bounded, &body, &names));
        for (index, (name, on_infinite, on_bounded)) in expected.iter().enumerate() {
            let item = run.item(index);
            let holds = if infinite { *on_infinite } else { *on_bounded };
            assert_eq!(
                label(&item),
                if holds { "proved" } else { "refuted" },
                "{name} bounded={bounded}: {item}"
            );
            if holds {
                assert_eq!(
                    item["record"]["certification"], "certified",
                    "{name}: {item}"
                );
            }
        }
    }
}

/// FR-338-AC-10 and FR-327 against the Counter's own arithmetic: position
/// `i` of the one run reads `i` for `i` up to 3, the deadlock, and the false
/// extension of the bounded profile reads every atom false past it. An `on
/// each` clause, with and without a `when` condition, holds when every
/// activation position's window is met; the closed form below reads the
/// windows directly and shares nothing with the monitor or its merged
/// window. Overlapping activations, windows part elapsed and past the
/// terminal state, and a discharge and reopening at one position all occur
/// in the grid, and a proof is certified.
#[test]
#[trace("FR-338-AC-28", "FR-126-AC-2")]
fn each_claims_settle_as_the_windows_of_their_activations_decide() {
    type Guard = (&'static str, fn(u32) -> bool);
    let guards: [Guard; 3] = [
        ("", |_| true),
        (" when (q.value >= 2)", |v| v >= 2),
        (" when (q.value = 1)", |v| v == 1),
    ];
    let run_values = [0_u32, 1, 2, 3];
    let window = |from: usize, upper: u32| -> Vec<Option<u32>> {
        (from..=from + usize::try_from(upper).expect("small"))
            .map(|position| run_values.get(position).copied())
            .collect()
    };
    let mut body = String::new();
    let mut expected = Vec::new();
    for (index, (when, guard)) in guards.iter().enumerate() {
        for upper in 0_u32..=3 {
            for k in [0_u32, 1, 3] {
                let name = format!("E{index}u{upper}k{k}");
                body.push_str(&format!(
                    "temporal {name} using t0 over(c: M::Counter) clock \"model-steps\" on each(q: M::Counter){when} {{ eventually[0,{upper}] holds(c.value = {k}) }}\n"
                ));
                let holds = run_values.iter().enumerate().all(|(position, value)| {
                    !guard(*value) || window(position, upper).into_iter().any(|at| at == Some(k))
                });
                expected.push((name, holds));
            }
            for k in [1_u32, 3] {
                let name = format!("A{index}u{upper}k{k}");
                body.push_str(&format!(
                    "temporal {name} using t0 over(c: M::Counter) clock \"model-steps\" on each(q: M::Counter){when} {{ always[0,{upper}] holds(c.value <= {k}) }}\n"
                ));
                let holds = run_values.iter().enumerate().all(|(position, value)| {
                    !guard(*value)
                        || window(position, upper)
                            .into_iter()
                            .all(|at| at.is_some_and(|at| at <= k))
                });
                expected.push((name, holds));
            }
        }
    }
    let ws = OwnedWorkspace::new("qsl2714-each-grid");
    let names: Vec<&str> = expected.iter().map(|(name, _)| name.as_str()).collect();
    let run = analyze(&request(&ws.0, true, &body, &names));
    for (index, (name, holds)) in expected.iter().enumerate() {
        let item = run.item(index);
        assert_eq!(
            label(&item),
            if *holds { "proved" } else { "refuted" },
            "{name}: {item}"
        );
        if *holds {
            assert_eq!(
                item["record"]["certification"], "certified",
                "{name}: {item}"
            );
        }
    }
}
