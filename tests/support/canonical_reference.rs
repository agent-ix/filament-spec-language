// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-181's tagged canonical `reference` value for a ConfigVersion
//! spine object.

use crate::support::config_version_spine::spine;
use serde_json::{json, Value};

/// The canonical reference to the ConfigVersion whose key is `key`.
pub fn config_version(key: &str) -> Value {
    let declaration = json!({
        "package": spine::PACKAGE_IDENTITY,
        "path": ["model", "object_type", format!("ix://{}/ConfigVersion", spine::PACKAGE_IDENTITY)],
        "ordinal": 0
    });
    let identity: String = key.bytes().map(|byte| format!("{byte:02x}")).collect();
    json!({"type": "reference", "universe": declaration.clone(),
        "object_type": declaration, "identity": identity})
}
