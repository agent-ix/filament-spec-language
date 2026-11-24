// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1042-AC-22: a fair cyclic interval is accepted by a fresh checker process.

use super::bounded::{requests_with_mode, run};
use super::fair_mdp::fair_w_source;
use fsl_analyze::exact_probabilistic::ExactProbLimits;
use ix_trace_rs::trace;
use serde_json::json;

const DOMAIN: &[u8] = include_bytes!("../fixtures/link.semantic-ir.json");
const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");

#[trace("FR-1042-AC-22")]
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
    assert_eq!(super::bounded::document(lower)["kind"], json!("lower"));
    assert_eq!(super::bounded::document(upper)["kind"], json!("upper"));
    for request in [lower, upper] {
        let (status, output) = run(request);
        assert_eq!(status, 0, "{output}");
        assert_eq!(output["value"], json!({"numerator":"1","denominator":"2"}));
    }
}
