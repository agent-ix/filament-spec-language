// SPDX-License-Identifier: AGPL-3.0-or-later
//! Every retained intake refusal and assembly error reaches the public outcome.

use filament_exact::Cancel;
use fsl_replay::spine::{self, CompileRefusal, ParseRequest, SpineLimits};
use fsl_replay::{DependencyInput, OutcomeDocument, SourceIdentity};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.model.complete/v1\";\n\
    model M = \"acme/orders\";\n";

fn check(text: &str, packages: &BTreeMap<String, Vec<u8>>) -> spine::FrontEndFailure {
    let source = SourceIdentity::new("test", "refused-model");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("the source parses")
    .into_value();
    let models = match spine::select(&parsed, packages, limits.model, &cancel) {
        Ok(models) => models.into_value(),
        Err(failure) => return failure,
    };
    spine::check(
        &parsed,
        "refused-model".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .expect_err("the unit refuses")
}

fn package(operations: usize, duplicate_identity: bool) -> BTreeMap<String, Vec<u8>> {
    let mut document: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/spine-model.semantic-ir.json"
    ))
    .unwrap();
    let origin = document["types"][0]["origin"].clone();
    document["types"][0]["operations"] = Value::Array((0..operations).map(|index| {
        let identity = format!("ix://acme/orders/Widget/op{index}");
        let parameter = |suffix: &str| json!({
            "identity": format!("{identity}/{suffix}"), "name": "same",
            "typeRef": "ix://filament/native/Integer", "presence": "required",
            "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": origin
        });
        json!({
            "identity": identity, "name": format!("op{index}"),
            "params": [parameter("first"), parameter(if duplicate_identity { "first" } else { "second" })],
            "returns": {"typeRef": "ix://filament/native/Integer", "nullable": false,
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true}},
            "pre": [], "post": [], "origin": origin,
            "frame": {"modifies": [], "creates": [], "deletes": []}
        })
    }).collect());
    BTreeMap::from([(
        "acme/orders".to_owned(),
        serde_json::to_vec(&document).unwrap(),
    )])
}

fn outcome(failure: spine::FrontEndFailure) -> (Box<CompileRefusal>, Value) {
    let result = Err(failure);
    let document = OutcomeDocument::from_check(&result);
    let bytes = document.to_bytes().unwrap();
    assert_eq!(bytes, document.to_bytes().unwrap());
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["category"], "refusal");
    let Err(failure) = result else {
        panic!("the front end refused")
    };
    let refusal = spine::refusal_or_fault(failure).unwrap();
    let messages: Vec<_> = value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["message"].as_str().unwrap())
        .collect();
    assert_eq!(refusal.to_string(), messages.join("\n"));
    assert!(!refusal.to_string().contains(", and "));
    (refusal, value)
}

#[test]
#[trace("TC-3044", "FR-056-AC-73", "FR-286-AC-24")]
fn a_refused_model_lists_every_violation() {
    for count in [3, 1] {
        let mut packages = package(3, false);
        if count == 1 {
            let bytes = packages.get_mut("acme/orders").unwrap();
            let mut document: Value = serde_json::from_slice(bytes).unwrap();
            for index in [1, 2] {
                document["types"][0]["operations"][index]["params"][1]["name"] = "other".into();
            }
            *bytes = serde_json::to_vec(&document).unwrap();
        }
        let (refusal, value) = outcome(check(UNIT, &packages));
        let CompileRefusal::Intake {
            region: Some(region),
            ..
        } = &*refusal
        else {
            panic!("expected a located intake refusal: {refusal:?}");
        };
        let start = UNIT.find("model M").unwrap();
        assert_eq!(region.start(), u64::try_from(start).unwrap());
        assert_eq!(
            &UNIT[start..usize::try_from(region.end()).unwrap()],
            "model M = \"acme/orders\";"
        );
        let expected_locus = serde_json::to_value(fsl_replay::OutcomeLocus::from(
            &fsl_foundation::diagnostic::Locus::Region(region.clone()),
        ))
        .unwrap();
        let entries = value["diagnostics"].as_array().unwrap();
        assert_eq!(entries.len(), count);
        assert_eq!(refusal.to_string().lines().count(), count);
        for (index, entry) in entries.iter().enumerate() {
            assert_eq!(entry["code"], "invalid_model_binding");
            assert_eq!(entry["cause"], "malformed-declaration");
            assert_eq!(entry["details"], Value::Null);
            assert_eq!(entry["locus"], expected_locus);
            let message = entry["message"].as_str().unwrap();
            assert!(message.contains("DUPLICATE_PARAM"), "{message}");
            assert!(
                message.contains(&format!("/ir/types/0/operations/{index}/params/1/name")),
                "{message}"
            );
        }
    }
}

#[test]
#[trace("TC-3044", "FR-056-AC-19", "FR-056-AC-73")]
fn reader_detail_stays_in_its_conflicting_binding_entry() {
    let (refusal, value) = outcome(check(UNIT, &package(2, true)));
    let entries = value["diagnostics"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(refusal.to_string().lines().count(), 4);
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry["code"], "invalid_model_binding");
        assert_eq!(entry["cause"], "conflicting-binding");
        assert_eq!(entry["details"], Value::Null);
        let lines: Vec<_> = entry["message"].as_str().unwrap().lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("DUPLICATE_IDENTITY"), "{lines:?}");
        assert!(lines[1].contains("DUPLICATE_PARAM"), "{lines:?}");
        assert!(
            lines[1].contains(&format!("/ir/types/0/operations/{index}/params/1/name")),
            "{lines:?}"
        );
    }
}

#[test]
#[trace("TC-3044", "FR-056-AC-73", "FR-286-AC-24")]
fn assembly_errors_each_have_an_entry_and_message_line() {
    let text = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        type First = MissingFirst;\ntype Second = MissingSecond;\n";
    let (refusal, value) = outcome(check(text, &BTreeMap::new()));
    let CompileRefusal::Assembly {
        refusal: assembly, ..
    } = &*refusal
    else {
        panic!("expected assembly errors: {refusal:?}");
    };
    assert_eq!(assembly.errors.len(), 2);
    let entries = value["diagnostics"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(refusal.to_string().lines().count(), 2);
    for (entry, name) in entries.iter().zip(["MissingFirst", "MissingSecond"]) {
        assert_eq!(entry["code"], "missing_declaration");
        assert_eq!(entry["cause"], "missing-name");
        assert_eq!(
            entry["message"],
            format!("no declaration is named `{name}`")
        );
        assert_eq!(entry["details"], Value::Null);
    }
    assert_ne!(entries[0]["locus"], entries[1]["locus"]);
}
