// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-634 "Invariant findings" (TC-2976) and FSpec FR-181-AC-33 (FSpec
//! TC-2013): a simulation item reports each invariant violation on its
//! reported traces as a finding naming the trace, the position, the invariant
//! and the object, and settles no claim.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, counter_request, Workspace};

const AT_MOST_TWO: &str =
    "invariant AtMostTwo using v on M::Counter at current { self.value <= 2 }\n";
const AT_MOST_THREE: &str =
    "invariant AtMostThree using v on M::Counter at current { self.value <= 3 }\n";

fn findings(item: &Value) -> &Vec<Value> {
    item["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("a simulation item carries `findings`: {item}"))
}

fn assert_violation(finding: &Value, trace: u64, position: u64) {
    assert_eq!(finding["kind"], "invariant-violated", "{finding}");
    assert!(
        finding["invariant"]
            .as_str()
            .is_some_and(|name| name.ends_with("AtMostTwo")),
        "{finding}"
    );
    assert!(finding["self"].to_string().contains("\"a\""), "{finding}");
    assert_eq!(
        finding["trace"].to_string().trim_matches('"'),
        trace.to_string()
    );
    assert_eq!(
        finding["position"].to_string().trim_matches('"'),
        position.to_string()
    );
}

/// FR-634-AC-6, FSpec FR-181-AC-33: the run that reaches `value = 3` is
/// reported with its finding; no claim is settled.
#[test]
#[trace("TC-2976", "FR-634-AC-6", "FSpec-TC-2013", "FSpec-FR-181-AC-33")]
#[ignore = "QSL-2132: simulation items do not report invariant findings yet"]
fn a_violated_invariant_is_a_finding_on_the_violating_run() {
    let ws = Workspace::new("simulation-findings-violated");
    let request = counter_request(
        &ws,
        AT_MOST_TWO,
        json!([{"kind": "simulation", "simulation": {"mode": "enumerate"}},
            {"kind": "simulation", "simulation": {"mode": "sample", "seed": 5, "samples": 2}}]),
    );
    let run = analyze(&request);
    let enumerated = run.item(0);
    assert_eq!(enumerated["category"], "success", "{enumerated}");
    assert_eq!(enumerated["traces"].as_array().map(Vec::len), Some(1));
    assert_eq!(enumerated["record"], Value::Null, "{enumerated}");
    assert_eq!(enumerated["counterexample"], Value::Null, "{enumerated}");
    let found = findings(&enumerated);
    assert_eq!(found.len(), 1, "{enumerated}");
    assert_violation(&found[0], 0, 3);

    let sampled = run.item(1);
    assert_eq!(sampled["category"], "success", "{sampled}");
    assert_eq!(sampled["record"], Value::Null, "{sampled}");
    let found = findings(&sampled);
    assert_eq!(found.len(), 2, "{sampled}");
    assert_violation(&found[0], 0, 3);
    assert_violation(&found[1], 1, 3);
    assert_eq!(run.document(), analyze(&request).document());
}

/// FR-634-AC-7: a holding invariant gives no finding and changes nothing
/// else; a bound that stops before `value = 3` reports none.
#[test]
#[trace("TC-2976", "FR-634-AC-7")]
#[ignore = "QSL-2132: simulation items do not report invariant findings yet"]
fn a_holding_invariant_or_an_unreached_violation_gives_no_finding() {
    let items = json!([{"kind": "simulation", "simulation": {"mode": "enumerate"}},
        {"kind": "simulation", "simulation": {"mode": "sample", "seed": 5}}]);
    let ws = Workspace::new("simulation-findings-holding");
    let holding = analyze(&counter_request(&ws, AT_MOST_THREE, items.clone()));
    let ws = Workspace::new("simulation-findings-plain");
    let plain = analyze(&counter_request(&ws, "", items));
    for index in 0..2 {
        let item = holding.item(index);
        assert_eq!(findings(&item), &Vec::<Value>::new(), "{item}");
        let other = plain.item(index);
        for member in ["category", "traces", "coverage"] {
            assert_eq!(item[member], other[member], "{member}: {item} vs {other}");
        }
    }

    let ws = Workspace::new("simulation-findings-bounded");
    let bounded = analyze(&counter_request(
        &ws,
        AT_MOST_TWO,
        json!([{"kind": "simulation", "simulation": {"mode": "enumerate", "max_states": 2}}]),
    ));
    let item = bounded.item(0);
    assert_eq!(item["category"], "incomplete", "{item}");
    assert!(item.to_string().contains("max_states"), "{item}");
    assert_eq!(findings(&item), &Vec::<Value>::new(), "{item}");
}
