// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1046: canonical receipt admission and item-local typed trace binding.

use std::collections::BTreeMap;
use std::fmt;

use filament_semantic_value::catalog::{CatalogCode, Category};
use fsl_replay::{OutcomeDiagnostic, TerminalRecord};
use fsl_semantics::model::observation::DocumentProvision;
use quire_canonical::{Encode, Node, NodeRef};
use serde_json::{Map, Value};

use super::{DocumentSelection, ItemSelection, SettledRun};

/// Recognize the existing receipt document family when assembling formal
/// premises. Admission still validates the complete selected bytes separately.
pub(crate) fn is_receipt(bytes: &[u8]) -> bool {
    quire_canonical::read(bytes, u64::try_from(bytes.len()).unwrap_or(u64::MAX)).is_ok_and(
        |document| {
            document
                .root()
                .get("contract_version")
                .is_some_and(|version| {
                    matches!(version.node(), Node::String("filament.effect-decision/v1"))
                })
        },
    )
}

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
    // The existing intake has already paid for every byte of this slice.
    // The shared arena reader, encoder and destructor all use explicit stacks.
    let bound = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let document = quire_canonical::read(bytes, bound).map_err(|error| match error {
        quire_canonical::ReadError::DuplicateName { pointer, .. } => {
            malformed(ReceiptCause::DuplicateMember, &pointer)
        }
        quire_canonical::ReadError::NumberOutOfRange { pointer, .. } => {
            malformed(ReceiptCause::InvalidValue, &pointer)
        }
        quire_canonical::ReadError::Malformed { .. } => malformed(ReceiptCause::MalformedJson, ""),
        _ => malformed(ReceiptCause::InvalidValue, ""),
    })?;
    let root = document.root();
    let Node::Object(members) = root.node() else {
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
    let object: BTreeMap<_, _> = members.collect();
    for name in object.keys() {
        if !FIELDS.contains(name) {
            return Err(malformed(
                ReceiptCause::UnknownMember,
                &format!("/{}", token(name)),
            ));
        }
    }
    for name in &FIELDS[..7] {
        if !object.contains_key(name) {
            return Err(malformed(ReceiptCause::MissingMember, &format!("/{name}")));
        }
    }
    for (name, member) in &object {
        let valid = match *name {
            "contract_version" | "idempotency_key" => matches!(member.node(), Node::String(_)),
            _ => matches!(member.node(), Node::Object(_)),
        };
        if !valid {
            return Err(malformed(ReceiptCause::WrongValueKind, &format!("/{name}")));
        }
    }
    if !object.get("contract_version").is_some_and(|version| {
        matches!(version.node(), Node::String("filament.effect-decision/v1"))
    }) {
        return Err(malformed(ReceiptCause::InvalidValue, "/contract_version"));
    }
    if let Some(pointer) = invalid_integer(root) {
        return Err(malformed(ReceiptCause::InvalidValue, &pointer));
    }
    let canonical = quire_canonical::to_vec(&document, quire_canonical::Limits::new(bound))
        .map_err(|_| malformed(ReceiptCause::InvalidValue, ""))?;
    if canonical != bytes {
        return Err(malformed(ReceiptCause::InvalidValue, ""));
    }
    let required = |name: &str| {
        object
            .get(name)
            .copied()
            .ok_or_else(|| malformed(ReceiptCause::MissingMember, &format!("/{name}")))
    };
    let decision = required("decision")?;
    let encode = |node: NodeRef<'_>| {
        quire_canonical::to_vec(&node, quire_canonical::Limits::new(bound))
            .map_err(|_| malformed(ReceiptCause::InvalidValue, ""))
    };
    // FR-329's five key components borrow the arena. No opaque JSON subtree
    // is converted to Value, recursively cloned or repaired into acceptance.
    let key = RequestKey {
        decision,
        grant: required("authority_grant")?,
        key: required("idempotency_key")?,
    };
    let key = quire_canonical::to_vec(
        &key,
        quire_canonical::Limits::new(bound.saturating_mul(2).saturating_add(16)),
    )
    .map_err(|_| malformed(ReceiptCause::InvalidValue, ""))?;
    Ok(Receipt {
        decision: encode(decision)?,
        key,
        effect_request: encode(required("effect_request")?)?,
    })
}

struct RequestKey<'d> {
    decision: NodeRef<'d>,
    grant: NodeRef<'d>,
    key: NodeRef<'d>,
}

impl Encode for RequestKey<'_> {
    fn encode_into<S: quire_canonical::Sink + ?Sized>(
        &self,
        writer: &mut quire_canonical::Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        for member in [
            Some(self.decision),
            Some(self.grant),
            Some(self.key),
            self.decision.get("operation"),
            self.decision.get("arguments"),
        ] {
            match member {
                Some(member) => member.encode_into(writer)?,
                None => writer.null()?,
            }
        }
        writer.end_array()
    }
}

/// Check the existing exact JSON integer domain without allocating a copy
/// of the path for every node or recursing through opaque object contents.
fn invalid_integer(root: NodeRef<'_>) -> Option<String> {
    enum Task<'d> {
        Value(NodeRef<'d>),
        Member(&'d str, NodeRef<'d>),
        Index(usize, NodeRef<'d>),
        Leave(usize),
    }
    let mut tasks = vec![Task::Value(root)];
    let mut pointer = String::new();
    while let Some(task) = tasks.pop() {
        match task {
            Task::Member(name, node) => {
                let previous = pointer.len();
                pointer.push('/');
                pointer.push_str(&token(name));
                tasks.push(Task::Leave(previous));
                tasks.push(Task::Value(node));
            }
            Task::Index(index, node) => {
                let previous = pointer.len();
                pointer.push('/');
                pointer.push_str(&index.to_string());
                tasks.push(Task::Leave(previous));
                tasks.push(Task::Value(node));
            }
            Task::Leave(length) => pointer.truncate(length),
            Task::Value(node) => match node.node() {
                Node::Number(number)
                    if number.value().fract() == 0.0
                        && number.value().abs() > 9_007_199_254_740_991.0 =>
                {
                    return Some(pointer)
                }
                Node::Object(members) => {
                    tasks.extend(
                        members
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .map(|(name, node)| Task::Member(name, node)),
                    );
                }
                Node::Array(items) => {
                    tasks.extend(
                        items
                            .enumerate()
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .map(|(index, node)| Task::Index(index, node)),
                    );
                }
                Node::Null | Node::Bool(_) | Node::Number(_) | Node::String(_) => {}
            },
        }
    }
    None
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
        let found_by_item: BTreeMap<_, _> = self
            .counterexamples
            .iter()
            .map(|found| (found.request_index, found))
            .collect();
        let mut keys: BTreeMap<&[u8], (&[u8], &DocumentSelection)> = BTreeMap::new();
        for (item_index, entries) in selected.iter().enumerate() {
            if entries.is_empty() {
                continue;
            }
            let index = fsl_foundation::RequestIndex::new(item_index);
            let Some(found) = found_by_item.get(&index) else {
                continue;
            };
            let super::super::Counterexample::Protocol { counterexample, .. } =
                &found.counterexample
            else {
                continue;
            };
            let mut occurrences: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
            // Prefix transitions are events; persistent state copies are not.
            for (position, step) in counterexample.prefix.iter().enumerate() {
                if let Some(record) = step
                    .transition
                    .recommendation_document()
                    .map_err(|_| super::AnalyzeDocumentError::Fault)?
                {
                    let count = occurrences.entry(record).or_default();
                    if counterexample
                        .lasso
                        .as_ref()
                        .is_some_and(|lasso| !lasso.terminal_stutter && position >= lasso.entry)
                    {
                        // The prefix holds one traversal of this native cycle.
                        // Its recommendation event occurs on every traversal.
                        *count = 2;
                    } else {
                        *count = count.saturating_add(1);
                    }
                }
            }
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
        let records: BTreeMap<_, _> = self
            .records
            .iter()
            .map(|record| (record.request_index(), record))
            .collect();
        for item in &mut self.items {
            let Some(index) = item["request_index"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
            else {
                continue;
            };
            let index = fsl_foundation::RequestIndex::new(index);
            if !failures.contains_key(&index) {
                continue;
            }
            let record = records
                .get(&index)
                .ok_or(super::AnalyzeDocumentError::Fault)?;
            let claim_name = item["claim_name"].as_str().map(str::to_owned);
            let request = item["request"].clone();
            let written = serde_json::to_value(fsl_replay::OutcomeItem::from_terminal(record))
                .map_err(|_| super::AnalyzeDocumentError::Fault)?;
            *item = super::item_document(
                written,
                super::ItemMembers {
                    claim_name,
                    request: Some(request),
                    identity: None,
                    target: None,
                    detail: None,
                    scoped: item.get("axes").is_some(),
                    axes: fsl_replay::OutcomeAxes::of_record(record.value(), true),
                    producer: None,
                    disposed: true,
                },
            );
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
                    if usize::try_from(index).ok().is_some_and(|index| {
                        failures.contains_key(&fsl_foundation::RequestIndex::new(index))
                    }) {
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
