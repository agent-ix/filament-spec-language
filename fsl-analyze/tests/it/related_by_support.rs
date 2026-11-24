// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "shared relation fixture helpers span split integration test binaries"
)]
//! Shared relationship fixture helper for integration targets.

use serde_json::{Value as Json, json};

fn id(suffix: &str) -> String {
    format!("ix://{}/{}", crate::fill::PACKAGE, suffix)
}

/// Add the `Cell`-to-`Cell` relationship used by protocol correlation fixtures.
pub(crate) fn add_owns_relationship(doc: &mut Json) {
    let cell = doc["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == json!(id("Cell")))
        .expect("the Cell type");
    let many = json!({"lower": 0, "upper": 2, "ordered": false, "unique": true});
    cell["relationships"] = json!([{
        "identity": id("relationship/owns"), "category": "structural", "composite": false,
        "direction": "source-to-target",
        "origin": {"source": {"sourceIdentity": id("spec"), "path": "model.fspec",
            "startLine": 1, "startColumn": 1}},
        "sourceEnd": {"type": id("Cell"), "role": "owner", "multiplicity": many},
        "targetEnd": {"type": id("Cell"), "role": "owned", "multiplicity": many}
    }]);
}
