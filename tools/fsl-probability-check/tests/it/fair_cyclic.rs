// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1042-AC-22: a fresh checker process accepts both documents of a fair
//! cyclic interval and rejects each one after one value is changed.

use super::bounded::{document, mutate, requests_with_mode, run};
use super::fair_mdp::fair_w_source;
use fsl_analyze::exact_probabilistic::ExactProbLimits;
use ix_trace_rs::trace;
use serde_json::json;

const DOMAIN: &[u8] = include_bytes!("../fixtures/link.semantic-ir.json");
const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");

#[trace("TC-3212", "FR-1042-AC-22")]
#[test]
fn fair_cyclic_interval_documents_pass_in_a_fresh_process() {
    let source = fair_w_source().replace("rational(1,2)", "rational(2,5)");
    let limits = ExactProbLimits {
        max_rational_bits: 1,
        precision_bits: 16,
        max_precision_bits: 16,
        ..ExactProbLimits::default()
    };
    let (requests, settlement) = requests_with_mode(&source, SNAPSHOT, limits, DOMAIN, false);
    let [lower, upper] = requests.as_slice() else {
        panic!("the fair interval must carry both proof roles: {settlement:?}");
    };
    assert_eq!(document(lower)["kind"], json!("lower"));
    assert_eq!(document(upper)["kind"], json!("upper"));
    // `go` expects 1/2 and `wait` is a self-loop. A `lower` value above 1/2
    // exceeds the expectation of `go`, which a minimum's lower bound may not.
    // An `upper` value below 1/2 is met only by `wait`, and a policy that
    // always waits is unfair to `go`.
    for (request, replacement, rule) in [
        (
            lower,
            json!({"numerator":"3","denominator":"4"}),
            "inequality-fails",
        ),
        (
            upper,
            json!({"numerator":"1","denominator":"4"}),
            "rank-broken",
        ),
    ] {
        let (status, output) = run(request);
        assert_eq!(status, 0, "{output}");
        assert_eq!(output["value"], json!({"numerator":"1","denominator":"2"}));

        // The decided states hold 0 or 1, so only the initial state's value
        // has another denominator.
        let offered = document(request);
        let undecided = offered["values"]
            .as_array()
            .expect("certificate value rows")
            .iter()
            .filter(|row| row["value"]["denominator"] != "1")
            .collect::<Vec<_>>();
        let [initial] = undecided.as_slice() else {
            panic!("one undecided product state: {offered}");
        };
        let key = &initial["state"];
        let altered = mutate(request, |doc| {
            for row in doc["values"]
                .as_array_mut()
                .expect("certificate value rows")
            {
                if row["state"] == *key {
                    row["value"] = replacement.clone();
                }
            }
        });
        let (status, output) = run(&altered);
        assert_eq!(status, 1, "{output}");
        assert_eq!(
            output,
            json!({"status":"rejected","rule":rule,"at":{"product_key":key}})
        );
    }
}
