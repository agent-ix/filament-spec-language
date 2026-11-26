// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 mutation registration for value bounds.

use super::*;
use fsl_certificate_mutation::{Fixture, Verdict, check_document};
use ix_trace_rs::trace;

/// Trace: FR-1051-AC-11, FR-1051-AC-13.
#[trace("FR-1051-AC-11", "FR-1051-AC-13")]
#[test]
fn value_bounds_certificate_rejects_each_single_mutation() {
    let exact = proof(&link(">=", "rational(2,5)"));
    let Keys { s, .. } = keys(&exact);
    let request = bound(&exact, "lower", &[(&s, rational(7, 16))], None, true);
    let document = document(&request);
    let result = check_document(
        Fixture {
            route: "value-bounds/v1",
            name: "cyclic-chain-lower-bound",
            document,
            allowed_to_survive: Vec::new(),
        },
        |bytes| {
            let changed: Value = serde_json::from_slice(bytes).expect("canonical certificate JSON");
            let mut offered = request.clone();
            offered["certificate"] = serde_json::to_string(&changed)
                .expect("certificate encoding")
                .into();
            if run(&offered).0 == 0 {
                Verdict::Accepted
            } else {
                Verdict::Rejected
            }
        },
    )
    .expect("all unlisted mutations reject");
    assert!(result > 0, "the fixture has generated single mutations");
}
