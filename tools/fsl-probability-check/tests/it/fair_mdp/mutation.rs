// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 mutation registration for the cyclic-MDP fair route.

use super::*;
use crate::bounded::{DOMAIN, request_with, run};
use fsl_certificate_mutation::{
    AllowedMutation, Fixture, MutationId, MutationKind, Verdict, check_document,
};
use ix_trace_rs::trace;

/// Trace: FR-1051-AC-11, FR-1051-AC-13.
#[trace("FR-1051-AC-11", "FR-1051-AC-13")]
#[test]
fn cyclic_mdp_fair_certificate_rejects_each_single_mutation() {
    let source = fair_w_source();
    let (request, settlement) = request_with(
        &source,
        include_bytes!("../../fixtures/link.snapshot.json"),
        ExactProbLimits::default(),
        DOMAIN,
    );
    let request = request.unwrap_or_else(|| panic!("fair fixture proof: {settlement:?}"));
    let document: Value =
        serde_json::from_str(request["certificate"].as_str().expect("certificate"))
            .expect("certificate document");
    let result = check_document(
        Fixture {
            route: "cyclic-mdp-fair-eventual/v1",
            name: "fair-w-minimum",
            document,
            allowed_to_survive: vec![AllowedMutation {
                id: MutationId {
                    member: "$/ranking/0/rank".into(),
                    kind: MutationKind::RankPlus,
                },
                reason: "The incremented policy rank still meets this fixture’s strict progress inequalities, so it remains a valid progress witness.".into(),
            }],
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
