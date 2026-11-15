// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec vectors read through FSL's public spine: the published `acme/orders`
//! domain package names its collection-typed field through a package map type
//! (FSpec FR-154-AC-50), a rendered type text in a `typeRef` refuses, and an
//! operation anchor carrying an `execution_point` binding validates against
//! the published checked-package V2 schema (FSpec FR-342-AC-12).
//!
//! Reads the filament-specification checkout named by `FSPEC_DIR`.

use std::path::PathBuf;

use fsl_foundation::{Code, IntakeLimits};
use fsl_semantics::model::intake::{admit, package_input, read_records};
use ix_trace_rs::trace;
use serde_json::{json, Value};

/// The `proposals/checked-package-v2` directory of the FSpec checkout.
fn checked_package_v2() -> PathBuf {
    let fspec = std::env::var_os("FSPEC_DIR")
        .expect("FSPEC_DIR must name a filament-specification checkout");
    PathBuf::from(fspec).join("proposals/checked-package-v2")
}

fn read_json(path: &std::path::Path) -> Value {
    let bytes =
        std::fs::read(path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display()))
}

fn orders() -> Value {
    read_json(&checked_package_v2().join("domain-package-acme-orders.json"))
}

/// The record refusals of a domain package document, empty when every node
/// reads.
fn refusal_codes(document: &Value) -> Vec<Code> {
    let bytes = serde_json::to_vec(document).expect("JSON");
    let input = package_input([bytes.as_slice()]).expect("the document declares an identity");
    let (_, parsed) =
        admit("acme/orders", &input, IntakeLimits::default()).expect("the selection admits");
    match read_records("acme/orders", &parsed) {
        Ok(_) => Vec::new(),
        Err(refusals) => refusals.into_iter().map(|refusal| refusal.code).collect(),
    }
}

/// FSpec FR-154-AC-50: the published orders document types `Order.clock` by
/// the package map type `ClockMap` and every node reads.
#[trace("FSpec-TC-1481", "FSpec-FR-154-AC-50")]
#[test]
#[ignore = "STD-544: the published orders document does not read through domain-package intake yet (refuses invalid_model_binding)"]
fn the_published_orders_document_names_its_clock_map_and_reads() {
    let document = orders();
    assert_eq!(
        document["types"][8]["fields"][18]["typeRef"],
        json!("ix://acme/orders/ClockMap")
    );
    assert_eq!(refusal_codes(&document), Vec::<Code>::new());
}

/// FSpec FR-154-AC-50: the same field typed by the rendered type text
/// `Map<Text, ix://filament/native/Clock>` refuses
/// `invalid_model_binding`/`malformed-declaration`.
#[trace("FSpec-TC-1481", "FSpec-FR-154-AC-50")]
#[test]
fn a_rendered_type_text_type_ref_refuses() {
    for rendered in [
        "Map<Text, ix://filament/native/Clock>",
        "Sequence<ix://filament/native/Integer>",
    ] {
        let mut document = orders();
        document["types"][8]["fields"][18]["typeRef"] = json!(rendered);
        let codes = refusal_codes(&document);
        assert!(
            codes.contains(&Code::InvalidModelBinding),
            "{rendered}: expected invalid_model_binding, got {codes:?}"
        );
    }
}

/// The published V2 schema, compiled.
fn v2_schema() -> jsonschema::JSONSchema {
    let schema = read_json(&checked_package_v2().join("schema.json"));
    jsonschema::JSONSchema::compile(&schema)
        .unwrap_or_else(|error| panic!("the published schema compiles: {error}"))
}

/// `positive-timed-clause-nodes.json` with `point` inserted as the fourth
/// binding of node 70, the operation anchor CL-03 patches.
fn anchor_with_point(point: Value) -> Value {
    let mut package =
        read_json(&checked_package_v2().join("fixtures/positive-timed-clause-nodes.json"));
    let members = package["semantic_graph"]["nodes"][70]["body"]["members"]
        .as_array_mut()
        .expect("the anchor body has members");
    members.insert(
        3,
        json!({"term": "binding", "name": "execution_point", "value": point}),
    );
    package
}

/// FSpec FR-342-AC-12: an operation anchor carrying `pre`, `post`,
/// `initialization` or `handler` as its `execution_point` validates against
/// the V2 schema; applicability is the reader's join.
#[trace("FSpec-TC-1480", "FSpec-FR-342-AC-12")]
#[test]
fn an_operation_anchor_with_an_execution_point_validates_against_the_v2_schema() {
    let schema = v2_schema();
    for point in [
        json!({"kind": "pre", "operation": "size"}),
        json!({"kind": "post", "operation": "size"}),
        json!({"kind": "initialization", "name": "start"}),
        json!({"kind": "handler", "name": "h"}),
    ] {
        let package = anchor_with_point(point.clone());
        assert!(schema.is_valid(&package), "{point}: schema-valid");
    }
}

/// FSpec FR-342-AC-12: an unknown point `kind` and a point with an extra
/// member fail the schema.
#[trace("FSpec-TC-1480", "FSpec-FR-342-AC-12")]
#[test]
fn a_malformed_execution_point_fails_the_v2_schema() {
    let schema = v2_schema();
    for point in [
        json!({"kind": "mid", "operation": "size"}),
        json!({"kind": "pre", "operation": "size", "extra": 1}),
    ] {
        let package = anchor_with_point(point.clone());
        assert!(!schema.is_valid(&package), "{point}: fails the schema");
    }
}
