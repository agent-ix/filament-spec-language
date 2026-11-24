// SPDX-License-Identifier: AGPL-3.0-or-later
//! The abstraction-relation fixture (FR-304, FR-305, FR-307): the object
//! type `ConfigVersion` (fields `version` and `note`, the operation
//! `attemptUpdate(n, m)` framed on `version`) in the population `configs`,
//! and `Base` (field `f`, operation `op`) with its subtype `Sub`, outside
//! any population. `Panel` is a systems interface, which has an object type
//! record but no object type node.

#![allow(dead_code, reason = "each test binary uses part of the fixture")]

use fsl_semantics::model::intake::meaning;
use serde_json::{json, Value};

#[path = "tc235_k2.rs"]
#[allow(dead_code, reason = "this fixture uses part of the shared K2 builder")]
#[allow(
    clippy::duplicate_mod,
    reason = "a test binary also loads the K2 builder directly"
)]
pub(crate) mod base;
pub(crate) use base::{id, PACKAGE};

const INTEGER: &str = "ix://filament/native/Integer";

fn parameter(operation: &str, name: &str) -> Value {
    typed_parameter(operation, name, INTEGER)
}

fn typed_parameter(operation: &str, name: &str, type_ref: &str) -> Value {
    let node = format!("{operation}/{name}");
    json!({
        "identity": id(&node), "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": base::origin(&node),
    })
}

/// The operation `name` of `owner` over the Integer `parameters`, framed
/// `frame`.
fn operation(owner: &str, name: &str, parameters: &[&str], frame: &Value) -> Value {
    let node = format!("{owner}/{name}");
    let params: Vec<Value> = parameters
        .iter()
        .map(|parameter_name| parameter(&node, parameter_name))
        .collect();
    json!({
        "identity": id(&node), "name": name, "params": params, "pre": [], "post": [],
        "frame": frame, "origin": base::origin(&node),
    })
}

fn object_type(name: &str, supertypes: &[&str], fields: &[&str], operations: Vec<Value>) -> Value {
    let identity = format!("{name}_id");
    let owner = supertypes.first().copied().unwrap_or(name);
    let mut all = Vec::new();
    if supertypes.is_empty() {
        all.push(base::field(name, &identity, INTEGER));
    }
    all.extend(fields.iter().map(|field| base::field(name, field, INTEGER)));
    base::declaration(
        name,
        "thing",
        json!({
            "supertypes": supertypes.iter().map(|ty| id(ty)).collect::<Vec<_>>(),
            "identityFields": [id(&format!("{owner}/{owner}_id"))],
            "fields": all,
            "operations": operations,
        }),
    )
}

/// The domain package document.
pub(crate) fn document() -> Value {
    let frame = json!({
        "modifies": [id("ConfigVersion/version")], "creates": [], "deletes": [],
    });
    let nothing = json!({"modifies": [], "creates": [], "deletes": []});
    let types = vec![
        object_type(
            "ConfigVersion",
            &[],
            &["version", "note"],
            vec![operation(
                "ConfigVersion",
                "attemptUpdate",
                &["n", "m"],
                &frame,
            )],
        ),
        object_type(
            "Base",
            &[],
            &["f"],
            vec![operation("Base", "op", &[], &nothing)],
        ),
        object_type("Sub", &["Base"], &[], Vec::new()),
        base::declaration(
            "Panel",
            "interface",
            json!({
                "supertypes": [],
                "fields": [base::field("Panel", "level", INTEGER)],
                "operations": [],
            }),
        ),
    ];
    let mut document = base::domain_document(
        types,
        vec![
            base::construct("thing", meaning::OBJECT_TYPE),
            base::construct("population", meaning::POPULATION),
            base::construct("interface", meaning::SYSTEMS_INTERFACE),
        ],
    );
    document["populations"] = json!([{
        "identity": id("configs"), "displayName": id("configs"),
        "kind": {"module": base::BUSINESS, "name": "population"},
        "members": [id("ConfigVersion")], "extent": "closed",
        "origin": base::origin("configs"),
    }]);
    document
}

/// A domain package whose object type `Leaf` extends `Base`, each declaring
/// one operation: the one type whose supertype lineage declares every
/// operation of the package is `Leaf`.
pub(crate) fn lineage_document() -> Value {
    let nothing = json!({"modifies": [], "creates": [], "deletes": []});
    let types = vec![
        object_type(
            "Base",
            &[],
            &["f"],
            vec![operation("Base", "op", &[], &nothing)],
        ),
        object_type(
            "Leaf",
            &["Base"],
            &[],
            vec![operation("Leaf", "step", &[], &nothing)],
        ),
    ];
    base::domain_document(types, vec![base::construct("thing", meaning::OBJECT_TYPE)])
}

/// A domain package whose object type `Timer` declares `wait(d)`, `d` a
/// quantity of milliseconds.
pub(crate) fn quantity_document() -> Value {
    let nothing = json!({"modifies": [], "creates": [], "deletes": []});
    let node = "Timer/wait";
    let wait = json!({
        "identity": id(node), "name": "wait",
        "params": [typed_parameter("Timer/wait", "d", "ix://filament/unit/ms")],
        "pre": [], "post": [], "frame": nothing, "origin": base::origin(node),
    });
    base::domain_document(
        vec![object_type("Timer", &[], &["f"], vec![wait])],
        vec![base::construct("thing", meaning::OBJECT_TYPE)],
    )
}
