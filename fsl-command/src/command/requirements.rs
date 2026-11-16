// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-960: the `call.requirements` member of a native-run/1 call, which asks
//! the run to settle the call's requirement items and names the backend of
//! each, read into a typed selection before any source is compiled.

use std::collections::BTreeMap;

use fsl_foundation::BackendId;
use serde_json::{Map, Value};

use super::{Result, RunCause};

/// The pointer of the `requirements` member of a run's call.
const POINTER: &str = "/call/requirements";

/// A backend and its options, named for every item of the call or for one.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Named {
    /// The named backend.
    pub backend: Option<BackendId>,
    /// The named backend's options; empty unless `backend` is named.
    pub options: BTreeMap<String, String>,
}

/// One `items` entry: the claim of one item of the call and its own backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Entry {
    /// The FSpec FR-331 claim `{node, occurrence}`, as the request spells it.
    pub claim: Value,
    /// The entry's own backend and options.
    pub named: Named,
}

/// The read `call.requirements` member (FR-960 "Inputs").
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Requirements {
    /// The call-level backend and options.
    pub call: Named,
    /// The per-item entries, in request order.
    pub items: Vec<Entry>,
}

impl Requirements {
    /// The backend and options of the item whose claim spells `claim`: its
    /// entry's when the entry names a backend, else the call-level ones
    /// (FR-960 "Precedence").
    pub fn named_for(&self, claim: &Value) -> &Named {
        self.items
            .iter()
            .find(|entry| &entry.claim == claim)
            .map(|entry| &entry.named)
            .filter(|named| named.backend.is_some())
            .unwrap_or(&self.call)
    }
}

fn refuse(pointer: String, reason: &str) -> RunCause {
    RunCause::ClauseRequest {
        member: pointer,
        reason: reason.to_owned(),
    }
}

/// Read `value`, the request's `call.requirements` member, refusing at
/// stage `request` with `invalid-request` naming the member's JSON pointer
/// (FR-960 "Refusals before the call").
pub(super) fn read(value: &Value) -> Result<Requirements> {
    let Value::Object(members) = value else {
        return Err(refuse(POINTER.to_owned(), "expected a closed object"));
    };
    let mut items = Vec::new();
    for key in members.keys() {
        if !matches!(key.as_str(), "backend" | "options" | "items") {
            return Err(refuse(
                format!("{POINTER}/{}", token(key)),
                "an unknown member",
            ));
        }
    }
    let call = named(members, POINTER)?;
    if let Some(entries) = members.get("items") {
        let pointer = format!("{POINTER}/items");
        let Value::Array(entries) = entries else {
            return Err(refuse(pointer, "expected an array"));
        };
        for (position, entry) in entries.iter().enumerate() {
            let pointer = format!("{pointer}/{position}");
            let entry = item(entry, &pointer)?;
            if items.iter().any(|seen: &Entry| seen.claim == entry.claim) {
                return Err(refuse(
                    pointer,
                    "an entry repeats the claim of an earlier entry",
                ));
            }
            items.push(entry);
        }
    }
    Ok(Requirements { call, items })
}

/// One `items` entry at `pointer`: a closed object with a `claim`.
fn item(value: &Value, pointer: &str) -> Result<Entry> {
    let Value::Object(members) = value else {
        return Err(refuse(
            pointer.to_owned(),
            "expected a closed object with a claim",
        ));
    };
    for key in members.keys() {
        if !matches!(key.as_str(), "claim" | "backend" | "options") {
            return Err(refuse(
                format!("{pointer}/{}", token(key)),
                "an unknown member",
            ));
        }
    }
    let Some(claim) = members.get("claim") else {
        return Err(refuse(
            pointer.to_owned(),
            "expected a closed object with a claim",
        ));
    };
    let closed_claim = claim.as_object().is_some_and(|claim| {
        claim.len() == 2 && claim.contains_key("node") && claim.contains_key("occurrence")
    });
    if !closed_claim {
        return Err(refuse(
            format!("{pointer}/claim"),
            "expected an FSpec FR-331 claim {node, occurrence}",
        ));
    }
    Ok(Entry {
        claim: claim.clone(),
        named: named(members, pointer)?,
    })
}

/// The `backend` and `options` members of the object at `pointer`.
fn named(members: &Map<String, Value>, pointer: &str) -> Result<Named> {
    let backend = match members.get("backend") {
        None => None,
        Some(Value::String(backend)) => Some(BackendId::from_wire(backend).map_err(|_| {
            refuse(
                format!("{pointer}/backend"),
                "expected a non-empty BackendId",
            )
        })?),
        Some(_) => {
            return Err(refuse(
                format!("{pointer}/backend"),
                "expected a non-empty BackendId",
            ))
        }
    };
    let mut options = BTreeMap::new();
    if let Some(value) = members.get("options") {
        let pointer = format!("{pointer}/options");
        let Value::Object(entries) = value else {
            return Err(refuse(pointer, "expected an object of string values"));
        };
        if backend.is_none() {
            return Err(refuse(
                pointer,
                "options are accepted only beside a backend",
            ));
        }
        for (name, value) in entries {
            let Value::String(value) = value else {
                return Err(refuse(
                    format!("{pointer}/{}", token(name)),
                    "expected a string value",
                ));
            };
            options.insert(name.clone(), value.clone());
        }
    }
    Ok(Named { backend, options })
}

/// `key` as one RFC 6901 reference token.
fn token(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}
