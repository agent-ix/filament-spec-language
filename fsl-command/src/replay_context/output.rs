// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-621 outcome carrier keeps original package JSON inside RawValue.

use std::collections::BTreeMap;

use fsl_foundation::RequestIndex;
use fsl_replay::replay_context::ContextError;
use serde::Serialize;
use serde_json::value::RawValue;
use serde_json::Value;

/// An analyze outcome whose reconstruction context bytes are retained exactly.
/// This is the output carrier, not a semantic JSON view of package authority.
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct RawContextOutcome(Box<RawValue>);

#[derive(Serialize)]
struct Document<'a> {
    #[serde(flatten)]
    members: BTreeMap<&'a str, &'a Value>,
    items: Vec<Item<'a>>,
}

#[derive(Serialize)]
struct Item<'a> {
    #[serde(flatten)]
    members: &'a serde_json::Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replay_context: Option<&'a RawValue>,
}

impl RawContextOutcome {
    /// Combine native outcome members with separately retained original
    /// context bytes. The package is never parsed or re-emitted by this writer.
    pub fn analyze(
        document: &Value,
        contexts: &BTreeMap<RequestIndex, &RawValue>,
    ) -> Result<Self, ContextError> {
        if document.get("format").and_then(Value::as_str) != Some("filament-outcome/1")
            || document.get("operation").and_then(Value::as_str) != Some("analyze")
        {
            return Err(ContextError::invalid("", "expected native analyze outcome"));
        }
        let members = document
            .as_object()
            .ok_or_else(|| ContextError::invalid("", "expected native outcome object"))?;
        let rows = document
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(|| ContextError::invalid("/items", "expected native outcome items"))?;
        let mut items = Vec::new();
        items
            .try_reserve(rows.len())
            .map_err(|_| ContextError::Allocation {
                requested: rows.len(),
            })?;
        let mut seen = std::collections::BTreeSet::new();
        for (position, item) in rows.iter().enumerate() {
            let item = item.as_object().ok_or_else(|| {
                ContextError::invalid(format!("/items/{position}"), "expected native item object")
            })?;
            if item.contains_key("replay_context") {
                return Err(ContextError::invalid(
                    format!("/items/{position}/replay_context"),
                    "context must come from the retained raw carrier",
                ));
            }
            let index = item
                .get("request_index")
                .and_then(Value::as_u64)
                .and_then(|index| usize::try_from(index).ok())
                .map(RequestIndex::new)
                .ok_or_else(|| {
                    ContextError::invalid(
                        format!("/items/{position}/request_index"),
                        "native request index is absent or out of range",
                    )
                })?;
            if !seen.insert(index) {
                return Err(ContextError::invalid(
                    format!("/items/{position}/request_index"),
                    "duplicate native request index",
                ));
            }
            let refuted = item
                .get("record")
                .and_then(|record| record.get("result"))
                .and_then(Value::as_str)
                == Some("refuted");
            items.push(Item {
                members: item,
                replay_context: refuted.then(|| contexts.get(&index).copied()).flatten(),
            });
        }
        if contexts.keys().any(|index| !seen.contains(index)) {
            return Err(ContextError::invalid(
                "/items",
                "retained context has no originating native item",
            ));
        }
        let output = Document {
            members: members
                .iter()
                .filter(|(name, _)| name.as_str() != "items")
                .map(|(name, value)| (name.as_str(), value))
                .collect(),
            items,
        };
        Ok(Self(serde_json::value::to_raw_value(&output)?))
    }

    /// The original output bytes, including untouched context/package fragments.
    pub fn as_raw(&self) -> &RawValue {
        &self.0
    }

    /// Transfer the raw output to the command adapter without a Value roundtrip.
    pub fn into_raw(self) -> Box<RawValue> {
        self.0
    }

    /// The bytes written by the external driver.
    pub fn to_bytes(&self) -> &[u8] {
        self.0.get().as_bytes()
    }
}

impl std::fmt::Display for RawContextOutcome {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0.get())
    }
}
