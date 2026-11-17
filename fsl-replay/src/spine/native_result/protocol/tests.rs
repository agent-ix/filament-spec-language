// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-352 "Protocol-analysis result" (FSpec TC-255): the record of
//! each result a protocol analysis settles, written and read back, and the
//! strict reader's refusals.

use std::collections::BTreeSet;

use filament_contract_model::CheckedNodePathSegment;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use super::*;
use crate::protocol_analysis::{
    AnalysisOutcome, AnalyzerIdentity, AssessedRoot, AssessedRootDraft, BoundsDraft, MissingMember,
    ProjectionBoundsDraft, ProjectionKeyDraft, ProtocolAnalysisResult, RealizabilityKeyDraft,
    RunLimit, UnsupportedCause,
};
use crate::spine::{read_native_run_result, NativeRunResult};
use crate::{ModelCheckLimit, ObservationIdentity};

fn id(name: &str) -> CheckedNodeId {
    CheckedNodeId {
        package: "pkg".into(),
        path: vec![CheckedNodePathSegment::Name(name.into())],
        ordinal: 0,
    }
}

fn projection() -> ProtocolAnalysisKey {
    ProtocolAnalysisKey::projection(ProjectionKeyDraft {
        definition_key: Some("pkg/Flow".to_owned()),
        role: Some("Warehouse".to_owned()),
        profile: Some("bounded-static".to_owned()),
        channels: Some(Vec::new()),
        choices: Some(Vec::new()),
        bounds: Some(ProjectionBoundsDraft { max_depth: Some(8) }),
    })
    .unwrap()
}

fn realizability() -> ProtocolAnalysisKey {
    ProtocolAnalysisKey::realizability(RealizabilityKeyDraft {
        definition_key: Some("pkg/Flow".to_owned()),
        strategy_profile: Some("bounded-static".to_owned()),
        role_knowledge: Some(Some(BTreeSet::new())),
        channels: Some(Vec::new()),
        choices: Some(Vec::new()),
        environment: Some(Some(BTreeSet::new())),
        bounds: Some(BoundsDraft {
            max_depth: Some(8),
            max_history: Some(3),
        }),
        analyzer: Some(AnalyzerIdentity {
            identity: "fsl-analyze::protocol_analysis".to_owned(),
            version: "0.1.0".to_owned(),
        }),
    })
    .unwrap()
}

fn witness() -> SeparatingWitnessRecord {
    AssessedRoot::new(Some(AssessedRootDraft {
        observation: Some(ObservationIdentity {
            authority: "test".to_owned(),
            identity: "protocol-claim".to_owned(),
        }),
        object: Some(filament_exact::ObjectReference::new(
            crate::fixture_universe(1),
            crate::fixture_effective(2),
            filament_exact::ObjectId::new("flow-1").unwrap(),
        )),
    }))
    .unwrap()
    .record(&id("PickedUp"), 2)
    .unwrap()
}

type Outcome = AnalysisOutcome<(), ()>;

fn record(key: &ProtocolAnalysisKey, outcome: Outcome) -> ProtocolAnalysisRecord {
    let witness = matches!(outcome, AnalysisOutcome::Refuted(())).then(witness);
    let settlement = ProtocolAnalysisSettlement::of(&ProtocolAnalysisResult {
        key: key.clone(),
        outcome,
    });
    ProtocolAnalysisRecord::new(key.clone(), &settlement, witness.as_ref()).unwrap()
}

fn read(bytes: &[u8]) -> Result<ProtocolAnalysisRecord, NativeRunReadError> {
    match read_native_run_result(bytes)? {
        NativeRunResult::ProtocolAnalysis(record) => Ok(*record),
        NativeRunResult::Clause(_) | NativeRunResult::CommandError(_) => {
            panic!("a protocol-analysis record reads as one")
        }
    }
}

fn document(record: &ProtocolAnalysisRecord) -> Value {
    serde_json::from_slice(&record.to_bytes().unwrap()).unwrap()
}

fn read_value(value: &Value) -> Result<ProtocolAnalysisRecord, NativeRunReadError> {
    read(&serde_json::to_vec(value).unwrap())
}

/// Each result of `kind`'s rows, with its `settlement`, top-level `basis`
/// and the `label`, `execution` and `truth` it is written with.
fn rows(realizable: bool) -> Vec<(Outcome, Value, &'static str, [&'static str; 3])> {
    use crate::protocol_analysis::InconclusiveCause as Cause;
    let inconclusive = |cause: Cause, wire: Value| {
        (
            AnalysisOutcome::Inconclusive(cause),
            json!({"result": "inconclusive", "cause": wire}),
            "unsettled",
            ["inconclusive", "completed", "pending"],
        )
    };
    let mut rows = vec![
        (
            AnalysisOutcome::Proved(()),
            json!({"result": "proved", "basis": {"type": "exhaustive"},
                   "certification": "uncertified"}),
            "closed-scope",
            ["proved", "completed", "satisfied"],
        ),
        (
            AnalysisOutcome::Refuted(()),
            json!({"result": "refuted"}),
            "decisive-counterexample",
            ["refuted", "completed", "violated"],
        ),
        inconclusive(Cause::Horizon, json!({"type": "bound-reached", "depth": 8})),
        inconclusive(Cause::ReplayDisagreed, json!({"type": "replay-disagreed"})),
        (
            AnalysisOutcome::Unsupported(UnsupportedCause {
                slug: UnsupportedSlug::UnsupportedJoin,
                node: id("Join"),
                detail: Some("quorum".to_owned()),
            }),
            json!({"result": "unsupported", "diagnostic": {
                "code": "unsupported_projection", "cause": "unsupported-join",
                "detail": {"node": node_text(&id("Join")).unwrap(), "reason": "quorum"}}}),
            "unavailable",
            ["unsupported", "unsupported", "unavailable"],
        ),
        (
            AnalysisOutcome::Failed(RunLimit {
                limit: ModelCheckLimit::MaxStates,
                value: 5,
            }),
            json!({"result": "incomplete", "cause": {"type": "limit-reached",
                   "limit": "max_states", "value": 5, "setting": "model_check.max_states"}}),
            "unavailable",
            ["failed", "resource-incomplete", "unavailable"],
        ),
    ];
    if realizable {
        rows.extend([
            inconclusive(
                Cause::PremiseNotStated {
                    member: "role_knowledge",
                },
                json!({"type": "premise-not-stated"}),
            ),
            inconclusive(Cause::Unforced, json!({"type": "unforced-refutation"})),
            inconclusive(
                Cause::StrategyReplayFailed,
                json!({"type": "strategy-replay-failed"}),
            ),
            inconclusive(Cause::CutCount, json!({"type": "cut-count"})),
            inconclusive(Cause::Undecided, json!({"type": "undecided-evaluation"})),
            inconclusive(
                Cause::InstanceBoundReached {
                    max_live_instances: 2,
                },
                json!({"type": "instance-bound-reached", "max_live_instances": 2}),
            ),
        ]);
    }
    rows
}

/// FSpec FR-352-AC-14: each result of each kind is written with exactly
/// `format`, `analysis`, `basis` and, with a decisive basis, `witness`, and
/// reads back to an equal record.
#[trace("FSpec-TC-255", "FSpec-FR-352-AC-14", "TC-1414", "FR-574-AC-8")]
#[test]
fn each_protocol_analysis_result_round_trips_as_its_record() {
    for (key, kind, realizable) in [
        (projection(), "local-projection", false),
        (realizability(), "realizability", true),
    ] {
        for (outcome, settlement, basis, [label, execution, truth]) in rows(realizable) {
            let decisive = matches!(outcome, AnalysisOutcome::Refuted(()));
            let written = record(&key, outcome);
            let bytes = written.to_bytes().unwrap();
            let value: Value = serde_json::from_slice(&bytes).unwrap();
            let members: Vec<&str> = value
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            let expected: &[&str] = if decisive {
                &["analysis", "basis", "format", "witness"]
            } else {
                &["analysis", "basis", "format"]
            };
            assert_eq!(members, expected, "{kind} {label}");
            assert_eq!(value["format"], "native-run-result/2");
            assert_eq!(value["basis"], basis, "{kind} {label}");
            let analysis = &value["analysis"];
            assert_eq!(analysis["kind"], kind);
            assert_eq!(
                [
                    &analysis["label"],
                    &analysis["execution"],
                    &analysis["truth"]
                ],
                [label, execution, truth],
                "{kind} {label}"
            );
            assert_eq!(analysis["settlement"], settlement, "{kind} {label}");
            assert_eq!(
                ProtocolAnalysisKey::from_json(&analysis["key"].to_string()).unwrap(),
                key
            );
            if decisive {
                let witness = &value["witness"];
                assert!(witness.get("quantifier").is_none() && witness.get("index").is_none());
                assert_eq!(
                    witness["deciding_element"],
                    node_text(&id("PickedUp")).unwrap()
                );
                assert_eq!(witness["value_path"]["kind"], "runtime_value");
                assert_eq!(
                    witness["value_path"]["root"]["observation"],
                    json!({"authority": "test", "identity": "protocol-claim"})
                );
                assert_eq!(witness["value_path"]["root"]["subject"]["tag"], "object");
                assert_eq!(witness["value_path"]["steps"], json!([]));
                assert_eq!(witness["trace_position"], "2");
            }
            let back = read(&bytes).unwrap();
            assert_eq!(back, written, "{kind} {label}");
            assert_eq!(back.to_bytes().unwrap(), bytes);
            assert_eq!(back.has_witness(), decisive);
            if decisive {
                assert!(back.witness_is(&witness()));
                assert_eq!(back.deciding_node(), Some(id("PickedUp")));
                assert_eq!(back.trace_position(), Some(2));
            }
        }
    }
}

/// FSpec FR-352-AC-15: each defect is refused with a located diagnostic
/// naming the member; a removed key member is
/// `invalid_runtime_input`/`missing-member` naming it.
#[trace("FSpec-TC-255", "FSpec-FR-352-AC-15")]
#[test]
fn the_strict_reader_refuses_each_defect_naming_the_member() {
    use crate::protocol_analysis::InconclusiveCause as Cause;
    let proved = document(&record(&projection(), AnalysisOutcome::Proved(())));
    let refuted = document(&record(&projection(), AnalysisOutcome::Refuted(())));
    let failed = document(&record(
        &projection(),
        AnalysisOutcome::Failed(RunLimit {
            limit: ModelCheckLimit::MaxStates,
            value: 5,
        }),
    ));
    let horizon = document(&record(
        &projection(),
        AnalysisOutcome::Inconclusive(Cause::Horizon),
    ));
    let reordered = {
        let canonical: serde_json::Map<String, Value> =
            serde_json::from_str(&node_text(&id("PickedUp")).unwrap()).unwrap();
        let members: Vec<String> = canonical
            .iter()
            .rev()
            .map(|(name, value)| format!("{}:{value}", Value::from(name.as_str())))
            .collect();
        format!("{{{}}}", members.join(","))
    };
    type Edit = fn(&mut Value, &str);
    let cases: [(&Value, Edit, &str); 14] = [
        (&proved, |d, _| d["disposition"] = json!({}), "disposition"),
        (
            &refuted,
            |d, _| {
                d.as_object_mut().unwrap().remove("analysis");
            },
            "disposition",
        ),
        (&proved, |d, _| d["usage"] = json!({}), "usage"),
        (
            &proved,
            |d, _| d["analysis"]["kind"] = json!("refinement"),
            "analysis.kind",
        ),
        (
            &proved,
            |d, _| {
                d["analysis"]["key"].as_object_mut().unwrap().remove("role");
            },
            "analysis.key",
        ),
        (
            &proved,
            |d, _| d["analysis"]["truth"] = json!("pending"),
            "analysis.label",
        ),
        (
            &refuted,
            |d, _| {
                d["analysis"]["settlement"] =
                    json!({"result": "inconclusive", "cause": {"type": "replay-disagreed"}});
            },
            "analysis.settlement.result",
        ),
        (
            &failed,
            |d, _| d["analysis"]["execution"] = json!("failed"),
            "analysis.execution",
        ),
        (
            &horizon,
            |d, _| d["analysis"]["settlement"]["cause"]["depth"] = json!(9),
            "analysis.settlement.cause.depth",
        ),
        (
            &horizon,
            |d, _| d["analysis"]["settlement"]["cause"] = json!({"type": "cut-count"}),
            "analysis.settlement.cause",
        ),
        (&refuted, |d, _| d["witness"]["index"] = json!(0), "index"),
        (
            &refuted,
            |d, text| d["witness"]["deciding_element"] = json!(text),
            "deciding_element",
        ),
        (
            &refuted,
            |d, _| {
                d["witness"]["value_path"]["steps"] = json!([{"tag": "member", "name": "m"}]);
            },
            "value_path",
        ),
        (
            &refuted,
            |d, _| d["witness"]["trace_position"] = json!("02"),
            "trace_position",
        ),
    ];
    for (base, edit, member) in cases {
        let mut broken = base.clone();
        edit(&mut broken, &reordered);
        let refusal = read_value(&broken).expect_err(member);
        assert!(refusal.to_string().contains(member), "{member}: {refusal}");
    }
    let mut removed = proved.clone();
    removed["analysis"]["key"]
        .as_object_mut()
        .unwrap()
        .remove("role");
    match read_value(&removed) {
        Err(NativeRunReadError::AnalysisKey(refusal)) => {
            assert_eq!(
                (refusal.code(), refusal.cause()),
                ("invalid_runtime_input", "missing-member")
            );
            assert!(matches!(
                refusal,
                crate::protocol_analysis::KeyReadError::Missing(MissingMember { ref member }) if member == "role"
            ));
        }
        other => panic!("a missing key member: {:?}", other.err()),
    }
}
