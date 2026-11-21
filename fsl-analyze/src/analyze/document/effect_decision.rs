// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1046: canonical receipt admission and item-local typed trace binding.

use std::collections::BTreeMap;
use std::fmt;

use filament_semantic_value::catalog::{CatalogCode, Category};
use fsl_foundation::canonical_json;
use fsl_replay::{OutcomeDiagnostic, TerminalRecord};
use fsl_semantics::model::observation::DocumentProvision;
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

use super::{DocumentSelection, ItemSelection, SettledRun};

/// Existing FR-272 causes admitted by the receipt reader.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptCause {
    /// Invalid JSON syntax.
    MalformedJson,
    /// A required outer property is absent.
    MissingMember,
    /// A member repeats within one object.
    DuplicateMember,
    /// An outer property is not declared by FR-329.
    UnknownMember,
    /// A property has the wrong JSON type.
    WrongValueKind,
    /// A version, domain value or encoding is invalid.
    InvalidValue,
}

impl ReceiptCause {
    /// The existing catalog cause spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MalformedJson => "malformed-json",
            Self::MissingMember => "missing-member",
            Self::DuplicateMember => "duplicate-member",
            Self::UnknownMember => "unknown-member",
            Self::WrongValueKind => "wrong-value-kind",
            Self::InvalidValue => "invalid-value",
        }
    }
}

/// The typed location of a refused receipt binding.
#[derive(Clone, Debug)]
pub struct ReceiptLocation {
    /// Request pointer to the selector.
    pub selector: String,
    /// Provision identity of the selected document.
    pub document: DocumentSelection,
    /// Pointer within the canonical receipt document.
    pub pointer: String,
}

/// A closed receipt refusal, selected before diagnostic formatting.
#[derive(Clone, Debug, thiserror::Error)]
pub enum ReceiptRefusal {
    /// The selected bytes are not a canonical FR-329 receipt.
    #[error("effect-decision malformed-receipt {location}")]
    Malformed {
        /// Existing catalog cause.
        cause: ReceiptCause,
        /// Selected receipt location.
        location: ReceiptLocation,
    },
    /// The selected item's actual trace contains no matching recommendation.
    #[error("effect-decision wrong-decision {0}")]
    WrongDecision(ReceiptLocation),
    /// The selected item's trace contains several matching occurrences.
    #[error("effect-decision ambiguous-decision {0}")]
    AmbiguousDecision(ReceiptLocation),
    /// A later receipt changes the effect request under an existing key.
    #[error("effect-decision request-key-contradiction {location}; first document {{{}, {}}}", first.authority, first.identity)]
    RequestKeyContradiction {
        /// Conflicting receipt location.
        location: ReceiptLocation,
        /// Provision identity of the first request under this key.
        first: DocumentSelection,
    },
}

impl fmt::Display for ReceiptLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} document {{{}, {}}} receipt pointer {:?}",
            self.selector, self.document.authority, self.document.identity, self.pointer
        )
    }
}

impl ReceiptRefusal {
    /// The existing catalog cause, independently of the diagnostic message.
    pub const fn cause(&self) -> ReceiptCause {
        match self {
            Self::Malformed { cause, .. } => *cause,
            Self::WrongDecision(_)
            | Self::AmbiguousDecision(_)
            | Self::RequestKeyContradiction { .. } => ReceiptCause::InvalidValue,
        }
    }

    /// The existing diagnostic envelope, with null details and source locus.
    pub fn diagnostic(&self) -> OutcomeDiagnostic {
        OutcomeDiagnostic::new(
            Some(self.cause().as_str()),
            "invalid_runtime_input",
            None,
            self.to_string(),
        )
    }
}

/// A selector shape refusal, before reading dependent files.
#[derive(Debug, thiserror::Error)]
#[error("{pointer}: {message}")]
pub struct SelectorRefusal {
    /// Pointer in the request document.
    pub pointer: String,
    /// Description of the invalid shape or unresolved selector.
    pub message: &'static str,
}

/// Read the item-local selectors without forwarding them to FR-331.
///
/// # Errors
/// Refuses non-arrays and malformed closed document references at their pointer.
pub fn selections(items: &[ItemSelection]) -> Result<Vec<Vec<DocumentSelection>>, SelectorRefusal> {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let pointer = format!("/items/{index}/effect_decisions");
            let Some(value) = &item.effect_decisions else {
                return Ok(Vec::new());
            };
            let Some(values) = value.as_array() else {
                return Err(SelectorRefusal {
                    pointer,
                    message: "effect_decisions must be an array",
                });
            };
            values
                .iter()
                .enumerate()
                .map(|(position, value)| {
                    serde_json::from_value(value.clone()).map_err(|_| SelectorRefusal {
                        pointer: format!("{pointer}/{position}"),
                        message: "malformed document selector",
                    })
                })
                .collect()
        })
        .collect()
}

fn location(
    item: usize,
    position: usize,
    document: &DocumentSelection,
    pointer: &str,
) -> ReceiptLocation {
    ReceiptLocation {
        selector: format!("/items/{item}/effect_decisions/{position}"),
        document: document.clone(),
        pointer: pointer.to_owned(),
    }
}

struct Receipt {
    decision: Vec<u8>,
    key: Vec<u8>,
    effect_request: Vec<u8>,
}

fn read(bytes: &[u8], at: &ReceiptLocation) -> Result<Receipt, ReceiptRefusal> {
    let malformed = |cause, pointer: &str| ReceiptRefusal::Malformed {
        cause,
        location: ReceiptLocation {
            pointer: pointer.to_owned(),
            ..at.clone()
        },
    };
    let mut duplicate = None;
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let parsed = UniqueValue {
        pointer: String::new(),
        duplicate: &mut duplicate,
    }
    .deserialize(&mut decoder);
    let value = match parsed {
        Ok(value) if decoder.end().is_ok() => value,
        _ => {
            return Err(match duplicate {
                Some(pointer) => malformed(ReceiptCause::DuplicateMember, &pointer),
                None => malformed(ReceiptCause::MalformedJson, ""),
            })
        }
    };
    let Some(object) = value.as_object() else {
        return Err(malformed(ReceiptCause::WrongValueKind, ""));
    };
    const FIELDS: &[&str] = &[
        "contract_version",
        "decision",
        "authority_grant",
        "idempotency_key",
        "effect_request",
        "transport_receipt",
        "lineage",
        "effect_observation",
    ];
    for name in object.keys() {
        if !FIELDS.contains(&name.as_str()) {
            return Err(malformed(
                ReceiptCause::UnknownMember,
                &format!("/{}", token(name)),
            ));
        }
    }
    for name in &FIELDS[..7] {
        if !object.contains_key(*name) {
            return Err(malformed(ReceiptCause::MissingMember, &format!("/{name}")));
        }
    }
    for (name, member) in object {
        let valid_kind = match name.as_str() {
            "contract_version" | "idempotency_key" => member.is_string(),
            _ => member.is_object(),
        };
        if !valid_kind {
            return Err(malformed(ReceiptCause::WrongValueKind, &format!("/{name}")));
        }
    }
    if value["contract_version"] != "filament.effect-decision/v1" {
        return Err(malformed(ReceiptCause::InvalidValue, "/contract_version"));
    }
    let canonical =
        canonical_json::encode(&value).map_err(|_| malformed(ReceiptCause::InvalidValue, ""))?;
    if canonical != bytes {
        return Err(malformed(ReceiptCause::InvalidValue, ""));
    }
    let encode = |value: &Value| {
        canonical_json::encode(value).map_err(|_| malformed(ReceiptCause::InvalidValue, ""))
    };
    // The complete decision already contains the canonical operation and
    // parameter-ordered typed arguments; include those explicitly in FR-329's key.
    let key = encode(&Value::Array(vec![
        value["decision"].clone(),
        value["authority_grant"].clone(),
        value["idempotency_key"].clone(),
        value["decision"]["operation"].clone(),
        value["decision"]["arguments"].clone(),
    ]))?;
    Ok(Receipt {
        decision: encode(&value["decision"])?,
        key,
        effect_request: encode(&value["effect_request"])?,
    })
}

/// Validate paid-for receipt bytes before any item consumes a document.
///
/// # Errors
/// Returns a typed FR-272 refusal for non-canonical or malformed receipts.
pub fn validate(
    bytes: &[u8],
    item: usize,
    position: usize,
    document: &DocumentSelection,
) -> Result<(), ReceiptRefusal> {
    read(bytes, &location(item, position, document, "")).map(|_| ())
}

impl SettledRun {
    /// Bind canonical receipts to actual occurrences in each item's native trace.
    /// Unsettled items without an eligible trace keep their original outcome.
    ///
    /// # Errors
    /// Malformed selectors or canonical documents refuse the request; an internal
    /// projection fault is reported separately from a binding refusal.
    pub fn bind_effect_decisions(
        &mut self,
        items: &[ItemSelection],
        documents: &DocumentProvision,
    ) -> Result<(), super::AnalyzeDocumentError> {
        let selected = selections(items).map_err(|_| super::AnalyzeDocumentError::Fault)?;
        // Normalize each provision identity once, including identities selected
        // repeatedly. File-byte accounting has already happened in command intake.
        let mut receipts = BTreeMap::new();
        for (item, entries) in selected.iter().enumerate() {
            for (position, document) in entries.iter().enumerate() {
                if receipts.contains_key(document) {
                    continue;
                }
                let bytes = documents
                    .get(&document.reference())
                    .ok_or(super::AnalyzeDocumentError::Fault)?;
                let receipt =
                    read(bytes, &location(item, position, document, "")).map_err(|error| {
                        super::AnalyzeDocumentError::Refused(
                            super::super::AnalyzeRefusal::EffectDecision(error),
                        )
                    })?;
                receipts.insert(document, receipt);
            }
        }
        let mut failures = BTreeMap::new();
        for found in &self.counterexamples {
            let super::super::Counterexample::Protocol { counterexample, .. } =
                &found.counterexample
            else {
                continue;
            };
            let index = found.request_index;
            let Some(entries) = selected
                .get(index.get())
                .filter(|entries| !entries.is_empty())
            else {
                continue;
            };
            let mut occurrences: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
            // Prefix transitions are events; persistent state copies are not.
            for step in &counterexample.prefix {
                if let Some(record) = step
                    .transition
                    .recommendation_document()
                    .map_err(|_| super::AnalyzeDocumentError::Fault)?
                {
                    let count = occurrences.entry(record).or_default();
                    *count = count.saturating_add(1);
                }
            }
            let mut keys: BTreeMap<&[u8], (&[u8], &DocumentSelection)> = BTreeMap::new();
            for (position, document) in entries.iter().enumerate() {
                let receipt = receipts
                    .get(document)
                    .ok_or(super::AnalyzeDocumentError::Fault)?;
                let at = location(index.get(), position, document, "/decision");
                let refusal = match occurrences.get(&receipt.decision).copied().unwrap_or(0) {
                    0 => Some(ReceiptRefusal::WrongDecision(at)),
                    1 => None,
                    _ => Some(ReceiptRefusal::AmbiguousDecision(at)),
                };
                if let Some(refusal) = refusal {
                    failures.insert(index, refusal);
                    break;
                }
                if let Some((request, first)) = keys.get(receipt.key.as_slice()) {
                    if *request != receipt.effect_request {
                        failures.insert(
                            index,
                            ReceiptRefusal::RequestKeyContradiction {
                                location: location(
                                    index.get(),
                                    position,
                                    document,
                                    "/effect_request",
                                ),
                                first: (*first).clone(),
                            },
                        );
                        break;
                    }
                } else {
                    keys.insert(&receipt.key, (&receipt.effect_request, document));
                }
            }
        }
        if failures.is_empty() {
            return Ok(());
        }
        for record in &mut self.records {
            if failures.contains_key(&record.request_index()) {
                *record = TerminalRecord::disposition_invalid_request(
                    record.request_index(),
                    CatalogCode::new("invalid_runtime_input", "invalid-value"),
                    Map::new(),
                );
            }
        }
        self.counterexamples
            .retain(|found| !failures.contains_key(&found.request_index));
        for item in &mut self.items {
            let Some(index) = item["request_index"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
            else {
                continue;
            };
            let Some((_, refusal)) = failures.iter().find(|(key, _)| key.get() == index) else {
                continue;
            };
            let request_index = item["request_index"].clone();
            let claim_name = item["claim_name"].clone();
            let request = item["request"].clone();
            *item = serde_json::json!({"request_index": request_index, "claim_name": claim_name,
                "request": request, "category": "refusal", "diagnostic": {
                    "code": "invalid_runtime_input", "cause": refusal.cause().as_str(), "detail": {}}});
        }
        self.document["items"] = Value::Array(self.items.clone());
        let diagnostics = self.document["diagnostics"]
            .as_array_mut()
            .ok_or(super::AnalyzeDocumentError::Fault)?;
        for refusal in failures.values() {
            diagnostics.push(
                serde_json::to_value(refusal.diagnostic())
                    .map_err(|_| super::AnalyzeDocumentError::Fault)?,
            );
        }
        if let Some(gaps) = self.document["aggregate"]["gaps"].as_array_mut() {
            for gap in gaps {
                if let Some(index) = gap["request_index"].as_u64() {
                    if failures
                        .keys()
                        .any(|key| u64::try_from(key.get()).ok() == Some(index))
                    {
                        gap["result"] = Value::String("invalid-request".to_owned());
                    }
                }
            }
        }
        self.exit_code =
            Category::most_severe([self.exit_code, Category::Refusal.exit_code()]).unwrap_or(20);
        if self.exit_code == Category::Refusal.exit_code() {
            self.document["category"] = Value::String(Category::Refusal.as_str().to_owned());
        }
        Ok(())
    }
}

fn token(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

struct UniqueValue<'a> {
    pointer: String,
    duplicate: &'a mut Option<String>,
}

impl<'de> DeserializeSeed<'de> for UniqueValue<'_> {
    type Value = Value;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Value, D::Error> {
        decoder.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for UniqueValue<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }
    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("invalid JSON number"))
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }
    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element_seed(UniqueValue {
            pointer: format!("{}/{}", self.pointer, values.len()),
            duplicate: self.duplicate,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(name) = map.next_key::<String>()? {
            let pointer = format!("{}/{}", self.pointer, token(&name));
            if values.contains_key(&name) {
                *self.duplicate = Some(pointer);
                return Err(serde::de::Error::custom("duplicate JSON member"));
            }
            let value = map.next_value_seed(UniqueValue {
                pointer,
                duplicate: self.duplicate,
            })?;
            values.insert(name, value);
        }
        Ok(Value::Object(values))
    }
}
