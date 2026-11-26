// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 mutation registration for package-byte counterexample replay.

use super::*;
use fsl_analyze::analyze::{AnalyzeLimits, PackageClauseSubject};
use fsl_certificate_mutation::{Fixture, Verdict, check_document};
use fsl_replay::{ModelTraceSettlement, ModelTraceValue, TemporalCounterexample};
use ix_trace_rs::trace;
use serde_json::Value;

/// Trace: FR-1051-AC-11, FR-1051-AC-13.
#[trace("FR-1051-AC-11", "FR-1051-AC-13")]
#[test]
fn counterexample_replay_from_package_bytes_rejects_single_mutations() {
    let world = world();
    let record = run(&world, ClauseLimits::default());
    let counterexample = record
        .model_check
        .as_ref()
        .and_then(|model| model.counterexample.clone())
        .expect("the source check produces a counterexample");
    let document = serde_json::to_value(&counterexample).expect("counterexample document");
    let node = &world.package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), node.claim().clone());

    let survivors = check_document(
        Fixture {
            route: "R-50 model-trace replay byte route",
            name: "counter-domain-invariant",
            document,
            allowed_to_survive: Vec::new(),
        },
        |bytes| {
            let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
                return Verdict::Rejected;
            };
            let Ok(payload) = serde_json::from_value::<TemporalCounterexample>(value) else {
                return Verdict::Rejected;
            };
            let mut result = None;
            over_package_route(&world, |input, package_bytes, supply| {
                result = Some(
                    PackageClauseSubject::new(
                        input,
                        package_bytes,
                        supply,
                        world.binding(&["c"]),
                        AnalyzeLimits::unbounded(),
                        &key,
                        ClauseLimits::default(),
                    )
                    .expect("the package holds the clause")
                    .replay_counterexample(&payload),
                );
            });
            match result.expect("the package byte route ran") {
                Ok(replayed)
                    if matches!(
                        replayed.settlement,
                        ModelTraceSettlement::Reproduced {
                            value: ModelTraceValue::Truth(false),
                            ..
                        }
                    ) =>
                {
                    Verdict::Accepted
                }
                _ => Verdict::Rejected,
            }
        },
    )
    .expect("all unlisted replay mutations fail to reproduce");
    assert!(survivors > 0, "the replay document has generated mutations");
}
