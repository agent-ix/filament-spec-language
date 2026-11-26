// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 mutation registration for finite-horizon measure proofs.

use super::*;
use fsl_certificate_mutation::{Fixture, Verdict, check_document};
use ix_trace_rs::trace;

/// Trace: FR-1051-AC-11, FR-1051-AC-13.
#[trace("FR-1051-AC-11", "FR-1051-AC-13")]
#[test]
fn finite_horizon_measure_certificate_rejects_each_single_mutation() {
    let mut source = model_source()
        .replace("{ true: 1, false: 9 }", "{ true: 1, false: 1 }")
        .replace("{ true: 1, false: 4 }", "{ true: 1, false: 3 }")
        .replace(
            "workload Even on M { weight Msg::send_a = 1; weight Msg::send_b = 1; }",
            "workload Steady on M { weight Msg::send_a = 1; weight Msg::send_b = 0; }",
        );
    source.push_str("\nprobabilistic Likely using inf on M over (m: M::Msg) under Steady on origin { mean of (fraction holds(m.delivered) over [0,2]) >= rational(2,5) }\n");
    let (request, settlement) = request(&source, SNAPSHOT, ExactProbLimits::default());
    let request = request.unwrap_or_else(|| panic!("exact measure fixture: {settlement:?}"));
    let document = document(&request);
    assert_eq!(
        document["objective"]["profile"],
        "filament.probability-objective/finite-horizon-measure/v1"
    );
    let result = check_document(
        Fixture {
            route: "finite-horizon-measure/v1",
            name: "finite-horizon-fraction-mean",
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
