// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec TC-2054's `test/refunds` fixture for optional operation parameters.

use serde_json::{Value, json};

pub(crate) const PACKAGE: &str = "test/refunds";

pub(crate) fn id(node: &str) -> String {
    format!("ix://{PACKAGE}/{node}")
}

fn origin(node: &str) -> Value {
    let identity = id(node);
    json!({"generated": {
        "generatorIdentity": identity,
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity],
    }})
}

fn field(owner: &str, name: &str, type_ref: &str) -> Value {
    let node = format!("{owner}/{name}");
    json!({
        "identity": id(&node), "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&node),
    })
}

fn object_type(name: &str, fields: Vec<Value>, operations: Vec<Value>) -> Value {
    let identity = id(name);
    let identity_fields = if name == "Staff" {
        vec![id("Staff/badge")]
    } else if name == "Order" {
        vec![id("Order/total")]
    } else {
        vec![id("Desk/order")]
    };
    json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": "thing"},
        "roles": [], "origin": origin(name), "constraints": [],
        "extensions": [], "unknownPolicy": "reject",
        "identityFields": identity_fields, "fields": fields, "operations": operations,
    })
}

fn parameter(operation: &str, name: &str, optional: bool) -> Value {
    let node = format!("{operation}/{name}");
    json!({
        "identity": id(&node), "name": name, "typeRef": id("Staff"),
        "presence": if optional { "optional" } else { "required" },
        "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
        "origin": origin(&node),
    })
}

fn note_operation() -> Value {
    let node = "Order/note";
    json!({
        "identity": id(node), "name": "note",
        "params": [
            parameter(node, "firstApprover", false),
            parameter(node, "secondApprover", true),
        ],
        "returns": {
            "typeRef": "ix://filament/native/Boolean",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
            "nullable": false,
        },
        "pre": [], "post": [],
        "frame": {"modifies": [], "creates": [], "deletes": []},
        "origin": origin(node),
    })
}

fn population(name: &str, member: &str) -> Value {
    json!({
        "identity": id(name), "displayName": id(name),
        "kind": {"module": PACKAGE, "name": "population"},
        "members": [id(member)], "extent": "closed", "origin": origin(name),
    })
}

/// The minimal TC-2054 `test/refunds` package: Staff, Order and Desk, with
/// `Order.note(firstApprover, secondApprover)` and its optional second parameter.
pub(crate) fn document() -> Value {
    let native_integer = "ix://filament/native/Integer";
    let types = vec![
        object_type(
            "Staff",
            vec![field("Staff", "badge", native_integer)],
            vec![],
        ),
        object_type(
            "Order",
            vec![field("Order", "total", native_integer)],
            vec![note_operation()],
        ),
        object_type(
            "Desk",
            vec![
                field("Desk", "order", &id("Order")),
                field("Desk", "staff", &id("Staff")),
            ],
            vec![],
        ),
    ];
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": id("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": PACKAGE, "version": "1.0.0", "mappingVersions": [], "profileVersions": []},
        "constructs": [
            {"kind": {"module": PACKAGE, "name": "thing"}, "moduleVersion": "1.0.0",
             "construct": {"identity": "none", "shape": "record", "members": {"identityFields": "required"},
                           "meaning": "filament.meaning.model.object-type/v1"}},
            {"kind": {"module": PACKAGE, "name": "population"}, "moduleVersion": "1.0.0",
             "construct": {"identity": "none", "shape": "record", "members": {},
                           "meaning": "filament.meaning.model.population/v1"}},
        ],
        "types": types, "occurrences": [], "extensions": [],
        "populations": [population("staff", "Staff"), population("orders", "Order"), population("desks", "Desk")],
    })
}
