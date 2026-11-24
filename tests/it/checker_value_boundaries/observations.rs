// SPDX-License-Identifier: AGPL-3.0-or-later

use super::HEADER;
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{CheckedUnit, DependencyInput};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{package_input, PackageInput};
use fsl_semantics::model::observation::{
    admit_current_snapshot, AdmissionFailure, DocumentRef, ObservationLimits, PathStep,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::collections::BTreeMap;

// The repository's public test constructors provide the carrier structure;
// the declarations, field data and adverse changes below are original.
use crate::support::probabilistic_model::base as carrier;
#[path = "../../../fsl-semantics/tests/support/variant_fields.rs"]
pub(super) mod fields;

fn model() -> Value {
    let native = |name: &str| format!("ix://filament/native/{name}");
    let envelope = carrier::declaration(
        "Envelope",
        "money",
        json!({"fields":[
        fields::field("Envelope", "label", &native("String")),
        fields::field("Envelope", "token", &native("UUID")),
        fields::field("Envelope", "payload", &native("Bytes")),
        fields::field("Envelope", "metadata", &native("JsonObject"))]}),
    );
    let mut flags = fields::many("Parcel", "flags", &native("Boolean"), true, false);
    flags["multiplicity"]["upper"] = json!(1);
    let parcel = carrier::declaration(
        "Parcel",
        "thing",
        json!({"identityFields":[carrier::id("Parcel/id")],"fields":[
        fields::field("Parcel", "id", &native("Integer")),
        fields::field("Parcel", "envelope", &carrier::id("Envelope")),
        fields::field("Parcel", "count", &carrier::id("Count")),
        fields::optional("Parcel", "approved", &native("Boolean")),
        flags
    ],"operations":[fields::operation_with("Parcel", "stamp", &[("receipt",native("Bytes"))], &native("UUID"))]}),
    );
    let count = carrier::declaration(
        "Count",
        "value",
        json!({"scalar":"integer","constraints":[
            {"identity":carrier::id("Count/min"),"keyword":"min","operands":{"value":"0"},"appliesTo":"ix://filament/native/Integer","diagnosticCode":"count.min","origin":carrier::origin("Count/min")},
            {"identity":carrier::id("Count/max"),"keyword":"max","operands":{"value":"7"},"appliesTo":"ix://filament/native/Integer","diagnosticCode":"count.max","origin":carrier::origin("Count/max")}
        ]}),
    );
    let mut document = fields::package(vec![envelope, parcel, count], true);
    let value_construct = document["constructs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["kind"]["name"] == "value")
        .unwrap();
    value_construct["construct"] = json!({"identity":"none","shape":"record","members":{},"meaning":fsl_semantics::model::intake::meaning::VALUE_TYPE});
    document["populations"] = json!([{
        "identity":carrier::id("parcels"),"displayName":"parcels",
        "kind":{"module":carrier::BUSINESS,"name":"population"},
        "origin":carrier::origin("parcels"),"extent":"closed","members":[carrier::id("Parcel")]
    }]);
    document
}

fn checked(body: &str) -> (CheckedUnit, PackageInput) {
    let bytes = serde_json::to_vec(&model()).unwrap();
    let packages = package_input([bytes.as_slice()]).expect("original model carrier admits");
    let source = format!("{HEADER}model M = {:?};\n{body}", fields::PACKAGE);
    let checked = crate::support::front_end::checked(
        SourceIdentity::new("boundary-tests", "original:parcel-model"),
        "parcel.native",
        source.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .expect("model declarations check");
    (checked, packages)
}

fn snapshot() -> Value {
    json!({"format":"filament.state.snapshot/v1",
    "identity":{"authority":"boundary-tests","identity":"original:parcel-state"},
    "observation":"current","model":{"identity":fields::PACKAGE},
    "populations":[{"population":carrier::id("parcels"),"complete":true,"objects":[{
        "key":"parcel-17","type":carrier::id("Parcel"),"fields":{
            "envelope":{"record":{"type":carrier::id("Envelope"),"fields":{"label":{"kind":"string","text":"00112233-4455-6677-8899-aabbccddeeff"},
                "token":{"kind":"uuid","text":"00112233-4455-6677-8899-aabbccddeeff"},
                "payload":{"kind":"bytes","text":"00ff00"},
                "metadata":{"kind":"json_object","text":"[null,7]"}}}},
            "id":{"integer":"17"},"count":{"integer":"6"},"approved":{"present":{"boolean":false}},"flags":{"sequence":[]}
        }}]}]})
}

#[test]
#[trace("TC-985", "TC-992", "TC-1231", "FR-106-AC-61")]
fn record_native_fields_distinguish_string_uuid_and_payload_kinds() {
    let (checked, packages) = checked("function marker using v(): Integer pure { 41 }");
    let graph = checked.package().graph();
    let selected = DocumentRef {
        authority: "boundary-tests".into(),
        identity: "original:parcel-state".into(),
    };
    let admit = |document: &Value| {
        admit_current_snapshot(
            graph.model_inputs(),
            graph.scope().types(),
            &packages,
            ModelNormalizationLimits::default(),
            &BTreeMap::from([(selected.clone(), serde_json::to_vec(document).unwrap())]),
            &selected,
            ObservationLimits::default(),
        )
    };
    let healthy = snapshot();
    admit(&healthy).expect("record-valued model fields admit native payloads");
    for field in ["token", "payload", "metadata"] {
        let mut wrong = healthy.clone();
        wrong["populations"][0]["objects"][0]["fields"]["envelope"]["record"]["fields"][field] =
            json!({"kind":"string","text":"00112233-4455-6677-8899-aabbccddeeff"});
        let AdmissionFailure::Refused(record) =
            admit(&wrong).expect_err("raw String cannot replace another native")
        else {
            panic!("a wrong native kind is a refusal")
        };
        assert_eq!(record.catalog.code(), "invalid_runtime_input");
        assert_eq!(record.catalog.cause(), "wrong-value-kind");
        assert_eq!(
            record.path.last(),
            Some(&PathStep::Field(field.into())),
            "{record:?}"
        );
    }
}

#[test]
#[trace("FR-106-AC-15")]
fn nested_record_shape_defects_do_not_admit_partial_field_values() {
    let (checked, packages) = checked("function marker using v(): Integer pure { 41 }");
    let graph = checked.package().graph();
    let selected = DocumentRef {
        authority: "boundary-tests".into(),
        identity: "original:parcel-state".into(),
    };
    for (change, expected_cause) in [
        ("missing", "missing-member"),
        ("extra", "unknown-member"),
        ("object-type", "wrong-value-kind"),
        ("unselected-type", "wrong-value-kind"),
    ] {
        let mut document = snapshot();
        let record = &mut document["populations"][0]["objects"][0]["fields"]["envelope"]["record"];
        match change {
            "missing" => {
                record["fields"].as_object_mut().unwrap().remove("metadata");
            }
            "extra" => record["fields"]["unexpected"] = json!({"integer":"99"}),
            "object-type" => record["type"] = json!(carrier::id("Parcel")),
            "unselected-type" => record["type"] = json!("ix://original/foreign/Envelope"),
            _ => unreachable!(),
        }
        let provisions =
            BTreeMap::from([(selected.clone(), serde_json::to_vec(&document).unwrap())]);
        let result = admit_current_snapshot(
            graph.model_inputs(),
            graph.scope().types(),
            &packages,
            ModelNormalizationLimits::default(),
            &provisions,
            &selected,
            ObservationLimits::default(),
        );
        let AdmissionFailure::Refused(record) =
            result.expect_err("a malformed nested record has no admitted environment")
        else {
            panic!("record defects are refusals")
        };
        assert_eq!(
            record.catalog.code(),
            "invalid_runtime_input",
            "{change}: {record:?}"
        );
        assert_eq!(
            record.catalog.cause(),
            expected_cause,
            "{change}: {record:?}"
        );
        assert_eq!(
            record.fields.get("field").map(String::as_str),
            Some("envelope"),
            "{change}: {record:?}"
        );
    }
}

#[test]
#[trace("TC-895")]
fn presence_comes_from_optional_field_and_not_sequence_cardinality() {
    checked("invariant Approved using v on M::Parcel at current { present(self.approved) implies (value(self.approved) = false) }");
    let bytes = serde_json::to_vec(&model()).unwrap();
    let packages = package_input([bytes.as_slice()]).unwrap();
    for expression in ["present(self.flags)", "value(self.approved) = false"] {
        let source = format!("{HEADER}model M = {:?}; invariant Bad using v on M::Parcel at current {{ {expression} }}",fields::PACKAGE);
        let error = crate::support::front_end::checked(
            SourceIdentity::new("boundary-tests", "original:presence-boundary"),
            "presence.native",
            source.as_bytes(),
            &packages,
            &DependencyInput::default(),
        )
        .unwrap_err();
        let expected = if expression.starts_with("present") {
            fsl_foundation::Code::IllTyped
        } else {
            fsl_foundation::Code::UndefinedExpression
        };
        assert_eq!(error.code(), expected, "{expression}: {error:?}");
    }
}

#[test]
#[trace("TC-029")]
fn scalar_expressions_cannot_replace_a_boolean_clause() {
    checked("invariant BooleanRoot using v on M::Parcel at current { self.count = self.count }");
    let bytes = serde_json::to_vec(&model()).unwrap();
    let packages = package_input([bytes.as_slice()]).unwrap();
    for expression in ["self.count", "self.envelope"] {
        let source = format!("{HEADER}model M = {:?}; invariant Bad using v on M::Parcel at current {{ {expression} }}",fields::PACKAGE);
        let error = crate::support::front_end::checked(
            SourceIdentity::new("boundary-tests", "original:scalar-roots"),
            "roots.native",
            source.as_bytes(),
            &packages,
            &DependencyInput::default(),
        )
        .unwrap_err();
        assert_eq!(
            error.code(),
            fsl_foundation::Code::IllTyped,
            "{expression}: {error:?}"
        );
    }
}

#[test]
#[trace("FR-106", "TC-985", "TC-992")]
fn invocation_native_parameters_and_result_are_admitted_in_their_declared_roles() {
    use fsl_semantics::model::observation::{
        admit_observations, ClauseFacts, ClauseSelection, ClauseSelectionInput, OperationFacts,
        Provisions,
    };
    let (checked, packages) =
        checked("post Stamped using v on M::Parcel::stamp { result = result }");
    let graph = checked.package().graph();
    let clause = graph
        .state_clause("Stamped")
        .expect("actual checked postcondition");
    let operation = clause.operation().unwrap();
    let facts = ClauseFacts {
        identity: clause.identity(),
        kind: clause.kind(),
        context: clause.context(),
        operation: Some(OperationFacts {
            declaring: operation.declaring.clone(),
            declaration: operation.declaration.clone(),
        }),
        operations: graph.scope().operations().clone(),
        named_populations: vec![],
    };
    let reference = |name: &str| DocumentRef {
        authority: "boundary-tests".into(),
        identity: name.into(),
    };
    let pre = reference("original:parcel-pre");
    let post = reference("original:parcel-post");
    let call = reference("original:parcel-call");
    let label = |document: &DocumentRef| json!({"authority":document.authority,"identity":document.identity});
    let snapshots = [(&pre, "pre"), (&post, "post")]
        .into_iter()
        .map(|(document, role)| {
            let mut value = snapshot();
            value["identity"] = label(document);
            value["observation"] = json!(role);
            (document.clone(), serde_json::to_vec(&value).unwrap())
        })
        .collect();
    let invocation = json!({"format":"filament.state.invocation/v1","identity":label(&call),
        "model":{"identity":fields::PACKAGE},"context":carrier::id("Parcel"),"operation":"stamp",
        "self":{"population":carrier::id("parcels"),"key":"parcel-17"},
        "pre":{"identity":label(&pre)},"post":{"identity":label(&post)},
        "parameters":{"receipt":{"kind":"bytes","text":"01ff"}},
        "result":{"kind":"uuid","text":"10213243-5465-7687-98a9-bacbdcedfe0f"},"created":[],"deleted":[]});
    let selection = ClauseSelection {
        name: "Stamped".into(),
        input: ClauseSelectionInput::Invocation {
            invocation: call.clone(),
        },
    };
    let admit = |value: &Value| {
        admit_observations(
            graph.model_inputs(),
            graph.scope().types(),
            &facts,
            &packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &snapshots,
                invocations: &BTreeMap::from([(call.clone(), serde_json::to_vec(value).unwrap())]),
            },
            &selection,
            ObservationLimits::default(),
        )
    };
    let admitted = admit(&invocation).expect("typed parameter and result admit");
    assert_eq!(admitted.clause, facts.identity);
    for member in ["parameters", "result"] {
        let mut wrong = invocation.clone();
        if member == "parameters" {
            wrong[member]["receipt"] = json!({"kind":"string","text":"01ff"});
        } else {
            wrong[member] = json!({"kind":"string","text":"10213243-5465-7687-98a9-bacbdcedfe0f"});
        }
        let AdmissionFailure::Refused(record) =
            admit(&wrong).expect_err("spelling equality does not erase native type")
        else {
            panic!("wrong native argument/result refuses")
        };
        assert_eq!(record.catalog.code(), "invalid_runtime_input");
        assert_eq!(record.catalog.cause(), "wrong-value-kind");
        let expected_field = if member == "parameters" {
            "receipt"
        } else {
            "result"
        };
        assert_eq!(
            record.fields.get("field").map(String::as_str),
            Some(expected_field),
            "{record:?}"
        );
    }
}
