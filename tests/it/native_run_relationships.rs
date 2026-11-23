// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1045: native-run relationship selection request admission.

use crate::workflow_relationship_navigation::{
    canonical_argument_reference, instance, relationship_row, run_selected, ORDER, PAYMENTS,
    PAYMENT_ATTEMPT, SHIPMENT, SHIPMENTS,
};
use filament_observation::{
    ProducerConfiguration, ProducerDigest, RelationshipDeclaration,
    RelationshipEndpointDeclaration, Revision, StaticProducerBundle, PRODUCER_INTERFACE_VERSION,
};
use fsl_command::command::{self, RunLimits};
use fsl_replay::Operation;
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn refused(request: Value) -> (Value, Value) {
    refused_bytes(serde_json::to_vec(&request).expect("request JSON"))
}

fn refused_bytes(bytes: Vec<u8>) -> (Value, Value) {
    let directory = tempfile::tempdir().expect("temporary request directory");
    let path = directory.path().join("request.json");
    std::fs::write(&path, bytes).expect("request file");
    let error = command::run_with_limits(&path, RunLimits::default(), &crate::negotiation::Fr290)
        .err()
        .expect("malformed relationship selection refuses");
    let document = error
        .outcome_document(Operation::Execute)
        .to_bytes()
        .expect("outcome document encodes");
    let envelope = error.value().expect("command error envelope encodes");
    (
        serde_json::from_slice(&document).expect("outcome JSON"),
        envelope,
    )
}

fn request_failure(output: &crate::command_harness::Output) -> Value {
    assert!(output.stdout.is_empty(), "request refusal wrote stdout");
    serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
        panic!(
            "request refusal is a command error: {error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn request(call: Value) -> Value {
    json!({
        "format": "native-run/1",
        "request": {
            "models": [],
            "program": {"source": {
                "file": "missing-program.native",
                "authority": "agent-ix",
                "identity": "test:relationship-request",
                "document": "RelationshipRequest",
                "formal_revision": 1
            }},
            "call": call
        }
    })
}

fn producer() -> Value {
    producer_with_relationships(Vec::new())
}

fn producer_with_relationships(relationships: Vec<(&str, &str, &str)>) -> Value {
    let mut candidate = StaticProducerBundle {
        bundle_identity: Some("ix://test/producer".to_owned()),
        bundle_revision: Some(Revision {
            namespace: "test-revision".to_owned(),
            value: "1".to_owned(),
        }),
        interface_version: Some(PRODUCER_INTERFACE_VERSION.to_owned()),
        configuration: Some(ProducerConfiguration {
            configuration_identity: "ix://test/configuration".to_owned(),
            digest: ProducerDigest {
                label: "filament-canonical-json-1/1".to_owned(),
                value: format!("sha256:{}", "00".repeat(32)),
            },
        }),
        relationships: Some(
            relationships
                .into_iter()
                .map(|(identity, source, target)| RelationshipDeclaration {
                    relationship_identity: identity.to_owned(),
                    source: RelationshipEndpointDeclaration {
                        type_identity: source.to_owned(),
                    },
                    target: RelationshipEndpointDeclaration {
                        type_identity: target.to_owned(),
                    },
                })
                .collect(),
        ),
        ..StaticProducerBundle::default()
    };
    let digest = candidate
        .canonical_digest_selection()
        .expect("producer candidate has a canonical digest");
    candidate.digest = Some(digest.clone());
    let mut producer = serde_json::to_value(candidate).expect("producer serializes");
    let wire_digest = |value: &str| {
        json!({
            "algorithm": "sha256",
            "domain": "filament-canonical-json-1",
            "value": value,
            "version": "1"
        })
    };
    producer["digest"] = wire_digest(&digest.value);
    producer["configuration"]["digest"] = wire_digest(&format!("sha256:{}", "00".repeat(32)));
    producer
}

fn producer_with_interface_version(version: &str) -> Value {
    let mut candidate: StaticProducerBundle =
        serde_json::from_value(producer()).expect("valid Producer wire bundle");
    candidate.interface_version = Some(version.to_owned());
    let digest = candidate
        .canonical_digest_selection()
        .expect("modified producer has a canonical digest");
    candidate.digest = Some(digest.clone());
    let mut wire = serde_json::to_value(candidate).expect("producer serializes");
    wire["digest"] = json!({
        "algorithm":"sha256",
        "domain":"filament-canonical-json-1",
        "value":digest.value,
        "version":"1"
    });
    wire["configuration"]["digest"] = json!({
        "algorithm":"sha256",
        "domain":"filament-canonical-json-1",
        "value":format!("sha256:{}", "00".repeat(32)),
        "version":"1"
    });
    wire
}

fn relationship_selection(relationship_identities: Value) -> Value {
    relationship_selection_with_producer(relationship_identities, producer())
}

fn relationship_selection_with_producer(relationship_identities: Value, producer: Value) -> Value {
    let authority = json!({
        "bundle_identity": producer["bundleIdentity"],
        "bundle_revision": producer["bundleRevision"],
        "digest": {
            "label": format!(
                "{}/{}",
                producer["digest"]["domain"].as_str().expect("digest domain"),
                producer["digest"]["version"].as_str().expect("digest version")
            ),
            "value": producer["digest"]["value"]
        }
    });
    json!({
        "producer": producer,
        "binding": {
            "identity": "binding",
            "source_identity": "source",
            "schema_identity": "schema",
            "signal_identity": "signal",
            "trigger_identity": "trigger",
            "unit": "unit",
            "subject_kind": "type",
            "required": true
        },
        "expected_subject": {
            "authority": authority,
            "subject_kind": "type",
            "authority_local_object_identity": "01"
        },
        "required_relationships": [],
        "relationship_identities": relationship_identities,
        "scope": {
            "population_identity": "population",
            "membership_rule_identity": "membership-rule",
            "membership_digest": "00".repeat(32),
            "membership_document": "{}",
            "required_member_identities": [],
            "observation_sources": [],
            "completeness_dependencies": [],
            "progress_dependencies": [],
            "clock_identity": "clock",
            "clock_revision": "1",
            "membership_complete": true,
            "closure_identity": null,
            "closure_digest": null,
            "kind": {"kind": "snapshot", "snapshot_identity": "snapshot"},
            "range": {"clock_family": "event-position", "start": "0", "end_exclusive": "1"},
            "members": []
        },
        "records": [],
        "limits": {
            "max_records": "1",
            "max_members": "1",
            "max_relationships": "1",
            "max_required_relationships": "1"
        }
    })
}

fn request_with_relationships(selection: Value) -> Value {
    request(json!({
        "function": "run",
        "arguments": [],
        "snapshot": "missing-snapshot.json",
        "relationships": selection
    }))
}

fn relationship_record() -> Value {
    json!({
        "identity": "record",
        "binding_identity": "binding",
        "source_identity": "source",
        "schema_identity": "schema",
        "subject": {
            "authority": {
                "bundle_identity": "ix://test/producer",
                "bundle_revision": {"namespace": "test-revision", "value": "1"},
                "digest": {"label": "filament-canonical-json-1/1", "value": format!("sha256:{}", "00".repeat(32))}
            },
            "subject_kind": "type",
            "authority_local_object_identity": "01"
        },
        "signal_identity": "signal",
        "trigger_identity": "trigger",
        "unit": "unit",
        "value": {"state": "present", "value_type": "reading", "canonical_value": "1"},
        "visibility": "external",
        "anchor": {"kind": "event-position", "position": "0"},
        "event_time_nanos": "0",
        "ingestion_time_nanos": null,
        "causal_relationship_identity": null,
        "clock_identity": "clock",
        "clock_revision": "1",
        "clock_uncertainty_nanos": "0"
    })
}

fn assert_before_snapshot_io(selection: Value, path: &str) {
    let (document, envelope) = refused(request_with_relationships(selection));
    assert_request_refusal(&document, &envelope, path);
}

fn assert_request_refusal(document: &Value, envelope: &Value, path: &str) {
    assert_eq!(document["format"], "filament-outcome/1", "{document}");
    let diagnostics = document["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1, "{document}");
    assert_eq!(diagnostics[0]["code"], "invalid-request", "{document}");
    assert_eq!(diagnostics[0]["details"]["path"], path, "{document}");
    assert_eq!(envelope["details"]["path"], path, "{envelope}");
}

#[test]
#[trace("FR-1045-AC-1")]
fn invalid_relationship_selection_fails_before_snapshot_io() {
    let (document, envelope) = refused(request(json!({
        "function": "run",
        "arguments": [],
        "relationships": {}
    })));
    assert_request_refusal(&document, &envelope, "/call/relationships");

    let selection = json!({
        "producer": {},
        "binding": {},
        "expected_subject": {},
        "required_relationships": [],
        "relationship_identities": [],
        "scope": {},
        "records": [],
        "limits": {},
        "unexpected": true
    });
    let (document, envelope) = refused(request(json!({
        "function": "run",
        "arguments": [],
        "snapshot": "missing-snapshot.json",
        "relationships": selection
    })));
    assert_request_refusal(&document, &envelope, "/call/relationships/unexpected");
}

#[test]
#[trace("FR-1045-AC-13")]
fn malformed_relationship_identities_refuse_before_io() {
    let cases = [
        (
            json!([
                {"relationship": "r", "declaration_identity": "d1"},
                {"relationship": "r", "declaration_identity": "d2"}
            ]),
            "/call/relationships/relationship_identities/1/relationship",
        ),
        (
            json!([
                {"relationship": "r1", "declaration_identity": "d"},
                {"relationship": "r2", "declaration_identity": "d"}
            ]),
            "/call/relationships/relationship_identities/1/declaration_identity",
        ),
        (
            json!([{"relationship": "r"}]),
            "/call/relationships/relationship_identities/0/declaration_identity",
        ),
        (
            json!([{"relationship": "r", "declaration_identity": "d", "extra": true}]),
            "/call/relationships/relationship_identities/0/extra",
        ),
    ];
    for (mappings, path) in cases {
        let call = json!({
            "function": "run",
            "arguments": [],
            "snapshot": "missing-snapshot.json",
            "relationships": relationship_selection(mappings)
        });
        let (document, envelope) = refused(request(call));
        assert_request_refusal(&document, &envelope, path);
    }

    let mut selection = relationship_selection(json!([]));
    selection
        .as_object_mut()
        .expect("selection object")
        .remove("relationship_identities");
    let call = json!({
        "function": "run",
        "arguments": [],
        "snapshot": "missing-snapshot.json",
        "relationships": selection
    });
    let (document, envelope) = refused(request(call));
    assert_request_refusal(
        &document,
        &envelope,
        "/call/relationships/relationship_identities",
    );
}

#[test]
#[trace("FR-1045-AC-13")]
fn unknown_package_relationship_in_mapping_refuses_after_package_admission() {
    let selection = relationship_selection(json!([{
        "relationship":"ix://test/workflow/relationship/not-in-package",
        "declaration_identity":"producer:Order-ships-Shipment"
    }]));
    let output = run_selected("shipments", "o1", vec![], Some(selection));
    let document = request_failure(&output);
    assert_eq!(output.status.code(), Some(20), "{document}");
    assert_eq!(document["code"], "invalid-request", "{document}");
    assert_eq!(document["stage"], "request", "{document}");
    assert_eq!(
        document["details"]["path"], "/call/relationships/relationship_identities/0/relationship",
        "{document}"
    );
}

#[test]
#[trace("FR-1045-AC-8")]
fn malformed_record_members_refuse_at_their_pointer_before_io() {
    let mut cases = Vec::new();
    let mut record = relationship_record();
    record["extra"] = json!(true);
    cases.push((record, "/call/relationships/records/0/extra"));

    let mut record = relationship_record();
    record
        .as_object_mut()
        .unwrap()
        .remove("ingestion_time_nanos");
    cases.push((record, "/call/relationships/records/0/ingestion_time_nanos"));

    let mut record = relationship_record();
    record["value"] = json!({"state": "absent"});
    cases.push((record, "/call/relationships/records/0/value/state"));

    let mut record = relationship_record();
    record["visibility"] = json!("Internal");
    cases.push((record, "/call/relationships/records/0/visibility"));

    for invalid in [json!(0), json!("+1"), json!("01")] {
        let mut record = relationship_record();
        record["event_time_nanos"] = invalid;
        cases.push((record, "/call/relationships/records/0/event_time_nanos"));
    }

    let mut record = relationship_record();
    record["clock_uncertainty_nanos"] = json!("18446744073709551616");
    cases.push((
        record,
        "/call/relationships/records/0/clock_uncertainty_nanos",
    ));

    for invalid in [json!("0A"), json!("0"), json!("")] {
        let mut record = relationship_record();
        record["causal_relationship_identity"] = invalid;
        cases.push((
            record,
            "/call/relationships/records/0/causal_relationship_identity",
        ));
    }

    for (record, path) in cases {
        let mut selection = relationship_selection(json!([]));
        selection["records"] = json!([record]);
        assert_before_snapshot_io(selection, path);
    }
}

#[test]
#[trace("FR-1045-AC-8")]
fn repeated_record_member_refuses_at_the_later_member_before_io() {
    let mut selection = relationship_selection(json!([]));
    selection["records"] = json!([relationship_record()]);
    let raw = serde_json::to_string(&request_with_relationships(selection))
        .expect("request JSON")
        .replace(
            "\"identity\":\"record\"",
            "\"identity\":\"record\",\"identity\":\"duplicate\"",
        );
    let (document, envelope) = refused_bytes(raw.into_bytes());
    assert_request_refusal(
        &document,
        &envelope,
        "/call/relationships/records/0/identity",
    );
}

#[test]
#[trace("FR-1045-AC-9")]
fn malformed_scope_members_refuse_at_their_pointer_before_io() {
    let cases = [
        (
            "range",
            json!({"clock_family": "timestamp", "start_nanos": "0", "end_nanos": "1"}),
            "/call/relationships/scope/range/clock_family",
        ),
        (
            "members",
            json!([{"object_identity":"o","record_identity":"r","anchor":{"kind":"timestamped-event","instant_nanos":"1"}}]),
            "/call/relationships/scope/members/0/anchor/kind",
        ),
        (
            "kind",
            json!({"snapshot":{"snapshot_identity":"s"}}),
            "/call/relationships/scope/kind/kind",
        ),
        (
            "membership_digest",
            json!("A0".to_owned() + &"0".repeat(62)),
            "/call/relationships/scope/membership_digest",
        ),
        (
            "membership_digest",
            json!("0".repeat(63)),
            "/call/relationships/scope/membership_digest",
        ),
        (
            "membership_document",
            json!({"members": []}),
            "/call/relationships/scope/membership_document",
        ),
    ];
    for (member, invalid, path) in cases {
        let mut selection = relationship_selection(json!([]));
        selection["scope"][member] = invalid;
        assert_before_snapshot_io(selection, path);
    }

    let mut selection = relationship_selection(json!([]));
    selection["scope"]
        .as_object_mut()
        .unwrap()
        .remove("closure_identity");
    assert_before_snapshot_io(selection, "/call/relationships/scope/closure_identity");

    let mut selection = relationship_selection(json!([]));
    selection["scope"]["members"] = json!([{
        "object_identity": "o",
        "record_identity": "r",
        "anchor": {"kind": "event-position", "position": "0", "extra": true}
    }]);
    assert_before_snapshot_io(
        selection,
        "/call/relationships/scope/members/0/anchor/extra",
    );
}

#[test]
#[trace("FR-1045-AC-10")]
fn malformed_subject_binding_and_limit_members_refuse_before_io() {
    let cases = [
        (
            "expected_subject",
            "bundle_revision",
            json!("1"),
            "/call/relationships/expected_subject/authority/bundle_revision",
        ),
        (
            "expected_subject",
            "authority_local_object_identity",
            json!(""),
            "/call/relationships/expected_subject/authority_local_object_identity",
        ),
        (
            "expected_subject",
            "authority_local_object_identity",
            json!("AB"),
            "/call/relationships/expected_subject/authority_local_object_identity",
        ),
        (
            "expected_subject",
            "authority_local_object_identity",
            json!(false),
            "/call/relationships/expected_subject/authority_local_object_identity",
        ),
        (
            "expected_subject",
            "subject_kind",
            json!(3),
            "/call/relationships/expected_subject/subject_kind",
        ),
        (
            "binding",
            "required",
            json!("true"),
            "/call/relationships/binding/required",
        ),
        (
            "limits",
            "max_records",
            json!(5),
            "/call/relationships/limits/max_records",
        ),
        (
            "limits",
            "max_records",
            json!("-1"),
            "/call/relationships/limits/max_records",
        ),
        (
            "limits",
            "max_records",
            json!("05"),
            "/call/relationships/limits/max_records",
        ),
        (
            "limits",
            "max_records",
            json!("18446744073709551616"),
            "/call/relationships/limits/max_records",
        ),
    ];
    for (group, member, invalid, path) in cases {
        let mut selection = relationship_selection(json!([]));
        if group == "expected_subject" {
            if member == "bundle_revision" {
                selection[group]["authority"][member] = invalid;
            } else {
                selection[group][member] = invalid;
            }
        } else {
            selection[group][member] = invalid;
        }
        assert_before_snapshot_io(selection, path);
    }
    for member in [
        "max_records",
        "max_members",
        "max_relationships",
        "max_required_relationships",
    ] {
        let mut selection = relationship_selection(json!([]));
        selection["limits"][member] = json!(5);
        assert_before_snapshot_io(selection, &format!("/call/relationships/limits/{member}"));
    }
}

#[test]
#[trace("FR-1045-AC-11")]
fn invalid_producer_bundle_refuses_before_io() {
    let unsupported = producer_with_interface_version("1.1.0");
    let mut selection = relationship_selection_with_producer(json!([]), unsupported);
    let (document, envelope) = refused(request_with_relationships(selection.clone()));
    assert_eq!(
        document["diagnostics"][0]["code"], "invalid-request",
        "{document}"
    );
    assert_eq!(
        envelope["details"]["path"], "/call/relationships/producer",
        "{envelope}"
    );
    assert!(envelope["details"]["cause"].is_string(), "{envelope}");

    let mut tampered = producer();
    tampered["bundleIdentity"] = json!("ix://test/tampered");
    selection = relationship_selection_with_producer(json!([]), tampered);
    let (document, envelope) = refused(request_with_relationships(selection));
    assert_eq!(
        document["diagnostics"][0]["code"], "invalid-request",
        "{document}"
    );
    assert_eq!(
        envelope["details"]["path"], "/call/relationships/producer",
        "{envelope}"
    );
    assert!(envelope["details"]["cause"].is_string(), "{envelope}");

    selection = relationship_selection(json!([]));
    selection["producer"] = json!("not-a-bundle");
    let (document, envelope) = refused(request_with_relationships(selection));
    assert_eq!(
        document["diagnostics"][0]["code"], "invalid-request",
        "{document}"
    );
    assert_eq!(
        envelope["details"]["path"], "/call/relationships/producer",
        "{envelope}"
    );
    assert!(envelope["details"]["cause"].is_string(), "{envelope}");
}

#[test]
#[trace("FR-1045-AC-10", "FR-1045-AC-11")]
fn producer_bundle_admits_through_the_producer_reader() {
    let output = run_selected(
        "shipments",
        "o1",
        vec![],
        Some(relationship_selection(json!([]))),
    );
    let document: Value = serde_json::from_slice(&output.stdout)
        .expect("admitted producer reaches a native-run outcome");
    assert_eq!(output.status.code(), Some(22), "{document}");
    assert_eq!(document["outcome"]["kind"], "incomplete", "{document}");
    assert_eq!(
        document["outcome"]["reasons"][0]["code"], "missing-closure",
        "{document}"
    );
    assert!(
        document["outcome"]["reasons"]
            .as_array()
            .expect("typed incomplete reasons")
            .iter()
            .all(|reason| reason["code"] != "missing-relationship"),
        "both mapped declarations satisfy their required slots: {document}"
    );
}

#[test]
#[trace("FR-1045-AC-5", "FR-100-AC-24")]
fn resolver_resource_limit_keeps_the_exact_incomplete_bound() {
    let mut selection = relationship_selection_with_producer(
        json!([]),
        producer_with_relationships(vec![("producer:Order-ships-Shipment", ORDER, SHIPMENT)]),
    );
    let authority = selection["expected_subject"]["authority"].clone();
    selection["required_relationships"] = json!([{
        "declaration_identity": "producer:Order-ships-Shipment",
        "source": {
            "authority": authority,
            "subject_kind": ORDER,
            "authority_local_object_identity": "6f31"
        },
        "target": {
            "authority": authority,
            "subject_kind": SHIPMENT,
            "authority_local_object_identity": "7331"
        }
    }]);
    selection["limits"]["max_required_relationships"] = json!("0");

    let output = run_selected("shipments", "o1", vec![], Some(selection));
    let document: Value = serde_json::from_slice(&output.stdout)
        .expect("resource limit is a native-run result document");
    assert_eq!(output.status.code(), Some(22), "{document}");
    assert_eq!(document["outcome"]["kind"], "incomplete", "{document}");
    assert_eq!(
        document["outcome"]["limit"], "max_required_relationships",
        "{document}"
    );
    assert!(document["outcome"].get("reasons").is_none(), "{document}");
    assert!(document["outcome"].get("stage").is_none(), "{document}");
}

#[test]
#[trace("FR-1045-AC-5", "FR-1045-AC-10")]
fn resolver_reference_and_selection_refusals_keep_typed_causes() {
    let complete = |mut selection: Value| {
        selection["scope"]["closure_identity"] = json!("closure");
        selection["scope"]["closure_digest"] = json!("22".repeat(32));
        selection
    };
    let mut foreign = complete(relationship_selection(json!([])));
    foreign["expected_subject"]["authority"]["bundle_identity"] = json!("ix://foreign/producer");
    let output = run_selected("shipments", "o1", vec![], Some(foreign));
    let document: Value = serde_json::from_slice(&output.stdout).expect("typed resolver result");
    assert_eq!(document["outcome"]["kind"], "refused", "{document}");
    assert_eq!(
        document["outcome"]["code"], "invalid_runtime_input",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["cause"], "invalid-reference-foreign-authority",
        "{document}"
    );
    assert!(document["outcome"].get("stage").is_none(), "{document}");

    for (member, value) in [
        ("label", json!("different-producer")),
        ("value", json!("00".repeat(32))),
    ] {
        let mut foreign_digest = complete(relationship_selection(json!([])));
        foreign_digest["expected_subject"]["authority"]["digest"][member] = value;
        let output = run_selected("shipments", "o1", vec![], Some(foreign_digest));
        let document: Value =
            serde_json::from_slice(&output.stdout).expect("typed resolver result");
        assert_eq!(
            document["outcome"]["code"], "invalid_runtime_input",
            "{document}"
        );
        assert_eq!(
            document["outcome"]["cause"], "invalid-reference-foreign-authority",
            "{document}"
        );
    }

    let mut empty_subject_kind = complete(relationship_selection(json!([])));
    empty_subject_kind["expected_subject"]["subject_kind"] = json!("");
    let output = run_selected("shipments", "o1", vec![], Some(empty_subject_kind));
    let document: Value = serde_json::from_slice(&output.stdout).expect("typed resolver result");
    assert_eq!(
        document["outcome"]["code"], "invalid_runtime_input",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["cause"], "invalid-reference-empty",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["fields"]["component"], "subject-kind",
        "{document}"
    );

    let mut empty_binding = complete(relationship_selection(json!([])));
    empty_binding["binding"]["identity"] = json!("");
    let output = run_selected("shipments", "o1", vec![], Some(empty_binding));
    let document: Value = serde_json::from_slice(&output.stdout).expect("typed resolver result");
    assert_eq!(
        document["outcome"]["code"], "invalid_runtime_input",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["cause"], "invalid-selection",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["fields"]["selection"],
        "binding-identity"
    );

    let mut closure = relationship_selection(json!([]));
    closure["scope"]["closure_identity"] = json!("closure");
    let output = run_selected("shipments", "o1", vec![], Some(closure));
    let document: Value = serde_json::from_slice(&output.stdout).expect("typed resolver result");
    assert_eq!(
        document["outcome"]["code"], "invalid_runtime_input",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["cause"], "invalid-selection",
        "{document}"
    );
    assert_eq!(document["outcome"]["fields"]["selection"], "closure");
}

#[test]
#[trace("FR-1045-AC-5", "FR-1045-AC-10")]
fn required_slot_with_unknown_producer_declaration_keeps_resolver_refusal() {
    let mut selection = relationship_selection_with_producer(
        json!([]),
        producer_with_relationships(vec![("producer:known", ORDER, SHIPMENT)]),
    );
    let authority = selection["expected_subject"]["authority"].clone();
    selection["required_relationships"] = json!([{
        "declaration_identity": "producer:missing",
        "source": {"authority": authority, "subject_kind": ORDER, "authority_local_object_identity": "01"},
        "target": {"authority": authority, "subject_kind": SHIPMENT, "authority_local_object_identity": "02"}
    }]);
    let output = run_selected("shipments", "o1", vec![], Some(selection));
    let document: Value =
        serde_json::from_slice(&output.stdout).expect("reference refusal is a native-run outcome");
    assert_eq!(document["outcome"]["kind"], "refused", "{document}");
    assert_eq!(
        document["outcome"]["code"], "invalid_runtime_input",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["cause"], "invalid-reference-unknown-relationship-declaration",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["fields"]["identity"], "producer:missing",
        "{document}"
    );
}

#[test]
#[trace("FR-1045-AC-2", "FR-106-AC-74")]
fn invalid_relationship_identity_refuses_after_snapshot_read() {
    for invalid_id in [
        json!(""),
        json!(false),
        json!("00FF"),
        json!("0"),
        json!("0g"),
    ] {
        let mut row = relationship_row(SHIPMENTS, true, vec![instance("o1", "s1")]);
        row["instances"][0]["identity"] = invalid_id;
        assert_identity_refusal(vec![row]);
    }
    let mut row = relationship_row(SHIPMENTS, true, vec![instance("o1", "s1")]);
    row["instances"][0]
        .as_object_mut()
        .expect("instance object")
        .remove("identity");
    assert_identity_refusal(vec![row]);
}

fn assert_identity_refusal(rows: Vec<Value>) {
    let selection = relationship_selection_with_producer(
        json!([{
            "relationship": SHIPMENTS,
            "declaration_identity": "producer:Order-ships-Shipment"
        }]),
        producer_with_relationships(vec![("producer:Order-ships-Shipment", ORDER, SHIPMENT)]),
    );
    let output = run_selected("shipments", "o1", rows, Some(selection));
    let failure = request_failure(&output);
    assert_eq!(output.status.code(), Some(20), "{failure}");
    assert_eq!(failure["code"], "invalid-request", "{failure}");
    assert_eq!(failure["stage"], "request", "{failure}");
    assert_eq!(
        failure["details"]["path"], "/call/snapshot/relationships/0/instances/0/identity",
        "{failure}"
    );
}

#[test]
#[trace("FR-1045-AC-5", "FR-1045-AC-14")]
fn mapping_to_undeclared_producer_relationship_is_resolver_refusal() {
    let row = relationship_row(
        SHIPMENTS,
        true,
        vec![{
            let mut instance = instance("o1", "s1");
            instance["identity"] = json!("01");
            instance
        }],
    );
    let selection = relationship_selection_with_producer(
        json!([{
            "relationship": SHIPMENTS,
            "declaration_identity": "producer:missing"
        }]),
        producer_with_relationships(vec![("producer:Order-ships-Shipment", ORDER, SHIPMENT)]),
    );
    let output = run_selected("shipments", "o1", vec![row], Some(selection));
    let document: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "resolver refusal is a result document: {error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_ne!(output.status.code(), Some(0), "{document}");
    assert_eq!(document["outcome"]["kind"], "refused", "{document}");
    assert_eq!(
        document["outcome"]["code"], "invalid_runtime_input",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["cause"], "invalid-reference-unknown-relationship-declaration",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["fields"]["identity"], "producer:missing",
        "{document}"
    );
}

#[test]
#[trace("FR-1045-AC-5", "FR-1045-AC-14")]
fn mapping_to_wrong_declaration_is_wrong_endpoint_kind() {
    let row = relationship_row(
        SHIPMENTS,
        true,
        vec![{
            let mut instance = instance("o1", "s1");
            instance["identity"] = json!("01");
            instance
        }],
    );
    let selection = relationship_selection_with_producer(
        json!([{
            "relationship": SHIPMENTS,
            "declaration_identity": "producer:Order-ships-Shipment"
        }]),
        producer_with_relationships(vec![(
            "producer:Order-ships-Shipment",
            ORDER,
            "ix://test/workflow-relationships/PaymentAttempt",
        )]),
    );
    let output = run_selected("shipments", "o1", vec![row], Some(selection));
    let document: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "resolver refusal is a result document: {error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_ne!(output.status.code(), Some(0), "{document}");
    assert_eq!(document["outcome"]["kind"], "refused", "{document}");
    assert_eq!(
        document["outcome"]["code"], "invalid_runtime_input",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["cause"], "invalid-reference-wrong-endpoint-kind",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["fields"]["endpoint"], "target",
        "{document}"
    );
}

#[test]
#[trace("FR-1045-AC-2")]
fn invalid_snapshot_relationship_references_keep_exact_refusals() {
    let mut instance = instance("o1", "s1");
    instance["identity"] = json!("01");
    let row = relationship_row(SHIPMENTS, true, vec![instance]);
    let selection = relationship_selection_with_producer(
        json!([{
            "relationship": SHIPMENTS,
            "declaration_identity": "producer:Order-ships-Shipment"
        }]),
        producer_with_relationships(vec![("producer:Order-ships-Shipment", SHIPMENT, ORDER)]),
    );
    let output = run_selected("shipments", "o1", vec![row], Some(selection));
    let document: Value = serde_json::from_slice(&output.stdout)
        .expect("reversed relationship returns a typed resolver result");
    assert_eq!(document["outcome"]["kind"], "refused", "{document}");
    assert_eq!(
        document["outcome"]["code"], "invalid_runtime_input",
        "{document}"
    );
    assert_eq!(
        document["outcome"]["cause"], "invalid-reference-reversed-endpoints",
        "{document}"
    );
}

#[test]
#[trace("FR-1045-AC-7", "FR-106-AC-74")]
fn relationship_identity_is_optional_without_correlation() {
    let output = run_selected(
        "paymentAttempts",
        "o1",
        vec![relationship_row(PAYMENTS, true, vec![instance("o1", "p1")])],
        None,
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "ordinary native-run outcome is a result document: {error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(output.status.code(), Some(0), "{document}");
    assert_eq!(document["outcome"]["kind"], "completed", "{document}");
    assert_eq!(
        document["outcome"]["value"],
        canonical_argument_reference(PAYMENT_ATTEMPT, "p1")
    );
}

#[test]
#[trace("FR-1045-AC-12", "FR-106-AC-74")]
fn mapped_relationships_correlate_and_unmapped_relationships_are_ignored() {
    let selection = relationship_selection_with_producer(
        json!([{
            "relationship": SHIPMENTS,
            "declaration_identity": "producer:Order-ships-Shipment"
        }]),
        producer_with_relationships(vec![("producer:Order-ships-Shipment", ORDER, SHIPMENT)]),
    );
    let mapped = relationship_row(
        SHIPMENTS,
        true,
        vec![{
            let mut instance = instance("o1", "s1");
            instance["identity"] = json!("01");
            instance
        }],
    );
    let mut unmapped = relationship_row(
        PAYMENTS,
        false,
        vec![{
            let mut instance = instance("o1", "p1");
            instance["identity"] = json!("NOT-HEX");
            instance
        }],
    );
    unmapped["complete"] = json!(false);
    let with_unmapped = run_selected(
        "shipments",
        "o1",
        vec![mapped.clone(), unmapped],
        Some(selection.clone()),
    );
    let without_unmapped = run_selected("shipments", "o1", vec![mapped], Some(selection));
    let with_unmapped: Value =
        serde_json::from_slice(&with_unmapped.stdout).expect("mapped result document");
    let without_unmapped: Value = serde_json::from_slice(&without_unmapped.stdout)
        .expect("mapped result document without the unmapped entry");
    assert_eq!(with_unmapped, without_unmapped);
}

#[test]
#[trace("FR-1045-AC-6", "FR-100-AC-6")]
fn unknown_relationship_type_in_correlation_is_invalid_request_after_snapshot_read() {
    let unknown = relationship_row(
        "ix://test/workflow/relationship/unknown",
        true,
        vec![instance("o1", "s1")],
    );
    let correlated = run_selected(
        "shipments",
        "o1",
        vec![unknown.clone()],
        Some(relationship_selection(json!([]))),
    );
    let failure = request_failure(&correlated);
    assert_eq!(correlated.status.code(), Some(20), "{failure}");
    assert_eq!(failure["code"], "invalid-request", "{failure}");
    assert_eq!(failure["stage"], "request", "{failure}");
    assert_eq!(
        failure["details"]["path"], "/call/snapshot/relationships/0/relationship",
        "{failure}"
    );

    let ordinary = run_selected("shipments", "o1", vec![unknown], None);
    let failure = request_failure(&ordinary);
    assert_eq!(ordinary.status.code(), Some(20), "{failure}");
    assert_eq!(failure["code"], "invalid_runtime_input", "{failure}");
    assert_eq!(failure["stage"], "call", "{failure}");
}
