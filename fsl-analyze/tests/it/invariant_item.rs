// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-951 (TC-2661): an invariant state clause over a model subject is an
//! EN-1 safety item, certified under the invariant key-context root and
//! refuted by a finite-prefix counterexample that replays.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, counter_request, label, Workspace};

const AT_MOST_THREE: &str =
    "invariant AtMostThree using v on M::Counter at current { self.value <= 3 }\n";
const AT_MOST_TWO: &str =
    "invariant AtMostTwo using v on M::Counter at current { self.value <= 2 }\n";

/// FR-951-AC-1: the holding invariant is proved exhaustively on EN-1 and its
/// certificate is checked.
#[trace("TC-2661", "FR-951-AC-1")]
#[test]
#[ignore = "QSL-2301: EN-1 has no invariant item form, root or closure certificate yet"]
fn a_holding_invariant_is_proved_and_certified_on_en_1() {
    let ws = Workspace::new("invariant-certified");
    let run = analyze(&counter_request(
        &ws,
        AT_MOST_THREE,
        json!([{"claim": "AtMostThree"}]),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["basis"]["type"], "exhaustive", "{item}");
    assert_eq!(item["record"]["backend"], "en-1", "{item}");
    assert_eq!(item["record"]["certification"], "certified", "{item}");
}

/// FR-951-AC-2: the violated invariant is refuted by a finite prefix ending
/// at `value = 3`, with an empty loop, and the replay reproduces it.
#[trace("TC-2661", "FR-951-AC-2")]
#[test]
#[ignore = "QSL-2301: EN-1 has no invariant item form or replayed state-clause counterexample yet"]
fn a_violated_invariant_is_refuted_by_a_replayed_finite_prefix() {
    let ws = Workspace::new("invariant-refuted");
    let run = analyze(&counter_request(
        &ws,
        AT_MOST_TWO,
        json!([{"claim": "AtMostTwo"}]),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["record"]["backend"], "en-1", "{item}");
    let counterexample = &item["counterexample"];
    assert_eq!(counterexample["loop"], json!([]), "{item}");
    assert_eq!(counterexample["over_binding"], Value::Null, "{item}");
    assert_eq!(counterexample["fairness"], json!([]), "{item}");
    let last = counterexample["prefix"]
        .as_array()
        .and_then(|prefix| prefix.last())
        .expect("a non-empty prefix");
    assert!(
        last.to_string().contains("\"3\""),
        "the prefix ends at value = 3: {last}"
    );
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// FR-951-AC-4: a declared invariant whose divisor may be zero refuses at
/// check, whatever the check mode, and a nonzero literal divisor checks.
#[trace("TC-2661", "FR-951-AC-4")]
#[test]
#[ignore = "QSL-2301: EN-1 has no invariant item form yet"]
fn a_declared_invariant_keeps_the_linked_nonzero_refusal() {
    let ws = Workspace::new("invariant-nonzero");
    let run = analyze(&counter_request(
        &ws,
        "invariant Risky using v on M::Counter at current { self.value <= 6 mod (3 - self.value) }\n",
        json!([{"claim": "Risky"}]),
    ));
    assert_eq!(run.exit, 20);
    assert_eq!(run.document()["last_stage"], "S3");
    assert!(
        run.document().to_string().contains("unproved-nonzero"),
        "{}",
        run.document()
    );
    let ws = Workspace::new("invariant-literal-divisor");
    let run = analyze(&counter_request(
        &ws,
        "invariant Literal using v on M::Counter at current { self.value <= 7 mod 4 }\n",
        json!([{"claim": "Literal"}]),
    ));
    assert_eq!(label(&run.item(0)), "proved", "{}", run.item(0));
}
