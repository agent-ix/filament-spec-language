// SPDX-License-Identifier: AGPL-3.0-or-later
//! The published FSpec FR-336 `$defs.CallInput` shape of a corpus call file.

use serde_json::{json, Value};

const CORPUS_SCHEMA: &str = "proposals/conformance-corpus-v1/schema.json";
const PACKAGE_SCHEMA: &str = "proposals/checked-package-v2/schema.json";

/// Validators for a call document and for one of its arguments.
pub(crate) struct CallSchemas {
    call: jsonschema::JSONSchema,
    argument: jsonschema::JSONSchema,
}

impl CallSchemas {
    /// The validators over the pinned FSpec schemas, or `None` when a pinned
    /// schema is absent or does not compile, which is a defect of the build.
    pub(crate) fn published() -> Option<Self> {
        let read = |path| {
            serde_json::from_slice::<Value>(filament_specification::published_file(path)?).ok()
        };
        let corpus = read(CORPUS_SCHEMA)?;
        let package = read(PACKAGE_SCHEMA)?;
        let compile =
            |name: &str| {
                jsonschema::JSONSchema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .with_document("urn:filament:checked-package:v2".to_owned(), package.clone())
                .with_document(
                    "urn:filament:conformance-corpus:v1".to_owned(),
                    corpus.clone(),
                )
                .compile(
                    &json!({"$ref": format!("urn:filament:conformance-corpus:v1#/$defs/{name}")}),
                )
                .ok()
            };
        Some(Self {
            call: compile("CallInput")?,
            argument: compile("InputValue")?,
        })
    }

    pub(crate) fn admits_call(&self, document: &Value) -> bool {
        self.call.is_valid(document)
    }

    pub(crate) fn admits_argument(&self, argument: &Value) -> bool {
        self.argument.is_valid(argument)
    }
}
