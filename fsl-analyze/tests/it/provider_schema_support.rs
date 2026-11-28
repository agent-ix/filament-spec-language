// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec's backend-provider schema, shared by split integration test binaries.

use serde_json::{json, Value};

/// One definition of FSpec's backend-provider schema, with the checked-package
/// schema it refers to.
pub(crate) fn provider_definition(name: &str) -> jsonschema::JSONSchema {
    let schema = crate::published_fspec::json("proposals/backend-provider-v1/schema.json");
    let package = crate::published_fspec::json("proposals/checked-package-v2/schema.json");
    let definition = json!({
        "$schema": schema["$schema"],
        "$defs": schema["$defs"],
        "$ref": format!("#/$defs/{name}"),
    });
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_document("urn:filament:checked-package:v2".to_owned(), package)
        .compile(&definition)
        .expect("the schema compiles")
}

pub(crate) fn assert_valid(compiled: &jsonschema::JSONSchema, instance: &Value, what: &str) {
    if let Err(errors) = compiled.validate(instance) {
        let errors: Vec<_> = errors.map(|error| error.to_string()).collect();
        panic!("{what} violates the schema: {errors:?}\n{instance}");
    }
}
