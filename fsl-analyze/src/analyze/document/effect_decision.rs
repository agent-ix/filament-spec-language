// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1046: canonical receipt admission and item-local typed trace binding.

use std::collections::BTreeMap;
use std::fmt;

use filament_semantic_value::catalog::{CatalogCode, Category};
use fsl_replay::{OutcomeDiagnostic, TerminalRecord};
use fsl_semantics::model::observation::{AdmissionFailure, AdmissionRecord, DocumentProvision};
use quire_canonical::{Encode, Node, NodeRef};
use serde_json::{Map, Value};

use super::{DocumentSelection, ItemSelection, SettledRun};

/// Recognize the existing receipt document family when assembling formal
/// premises. Admission still validates the complete selected bytes separately.
/// Command document intake uses the same classifier to keep opaque receipt
/// contents out of its ordinary document metadata decoder.
///
/// # Errors
/// Propagates resource and invariant failures; they never select another parser.
pub fn is_receipt(bytes: &[u8]) -> Result<bool, ReceiptError> {
    receipt_document(bytes).map(|document| document.is_some())
}

/// Recognize a receipt and retain its iterative arena for metadata extraction.
/// Malformed ordinary documents retain their original admission path.
///
/// # Errors
/// Propagates resource and invariant failures without an ordinary-parser fallback.
pub fn receipt_document(bytes: &[u8]) -> Result<Option<quire_canonical::Document>, ReceiptError> {
    classify(quire_canonical::read(
        bytes,
        u64::try_from(bytes.len()).unwrap_or(u64::MAX),
    ))
}

fn classify(
    read: Result<quire_canonical::Document, quire_canonical::ReadError>,
) -> Result<Option<quire_canonical::Document>, ReceiptError> {
    match read {
        Ok(document) => Ok(document
            .root()
            .get("contract_version")
            .is_some_and(|version| {
                matches!(version.node(), Node::String("filament.effect-decision/v1"))
            })
            .then_some(document)),
        Err(
            quire_canonical::ReadError::Malformed { .. }
            | quire_canonical::ReadError::DuplicateName { .. }
            | quire_canonical::ReadError::NumberOutOfRange { .. },
        ) => Ok(None),
        Err(error) => Err(reader_failure(error)),
    }
}

/// Receipt data refusal, allocator failure, or an established local invariant.
/// Local derived ceilings carry no caller setting or budget authority.
#[derive(Debug, thiserror::Error)]
pub enum ReceiptError {
    /// A typed input-data refusal.
    #[error(transparent)]
    Refusal(#[from] ReceiptRefusal),
    /// The shared reader or encoder could not allocate these bytes.
    #[error("effect-decision allocation failed requesting {requested} bytes")]
    Allocation {
        /// Bytes requested by the failed allocator operation.
        requested: usize,
    },
    /// An established derived bound or shared-reader/encoder invariant failed.
    #[error("effect-decision canonical invariant failed")]
    Fault,
}

fn allocation_failure(requested: usize) -> AdmissionFailure {
    AdmissionFailure::Incomplete(Box::new(AdmissionRecord {
        catalog: fsl_foundation::diagnostic::ALLOCATION_FAILED,
        fields: BTreeMap::from([("requested", requested.to_string())]),
        path: Vec::new(),
        kernel: None,
    }))
}

impl ReceiptError {
    /// Preserve the existing analyze resource or internal-failure domain.
    pub fn into_analyze_error(self) -> super::AnalyzeDocumentError {
        match self {
            Self::Refusal(refusal) => super::AnalyzeDocumentError::Refused(
                super::super::AnalyzeRefusal::EffectDecision(refusal),
            ),
            Self::Allocation { requested } => super::AnalyzeDocumentError::Refused(
                super::super::AnalyzeRefusal::Document(allocation_failure(requested)),
            ),
            Self::Fault => super::AnalyzeDocumentError::Fault,
        }
    }

    /// Preserve classifier failures while assembling native replay premises.
    pub(crate) fn into_replay_error(self) -> fsl_replay::ReplayRefusal {
        match self {
            Self::Allocation { requested } => {
                fsl_replay::ReplayRefusal::Admission(allocation_failure(requested))
            }
            Self::Refusal(_) | Self::Fault => fsl_replay::ReplayRefusal::Fault(
                filament_semantic_value::catalog::InternalFault::new(
                    "analyze",
                    "receipt-classification",
                ),
            ),
        }
    }
}

fn reader_failure(error: quire_canonical::ReadError) -> ReceiptError {
    match error {
        quire_canonical::ReadError::Allocation { requested } => {
            ReceiptError::Allocation { requested }
        }
        // read() was given the already-paid slice length, not a caller budget
        // or reservation. A denial here has no known caller limit to report.
        _ => ReceiptError::Fault,
    }
}

fn encoder_failure(error: quire_canonical::Error) -> ReceiptError {
    match error {
        quire_canonical::Error::Allocation { requested } => ReceiptError::Allocation { requested },
        _ => ReceiptError::Fault,
    }
}

fn whole_encoder_failure(error: quire_canonical::Error, at: &ReceiptLocation) -> ReceiptError {
    match error {
        // Before canonicality is established, canonical spelling can grow
        // beyond the input (1e5 -> 100000). This cap is not a caller limit.
        quire_canonical::Error::Limit(limit)
            if limit.kind == quire_canonical::LimitKind::CanonicalBytes =>
        {
            ReceiptRefusal::Malformed {
                cause: ReceiptCause::InvalidValue,
                location: at.clone(),
            }
            .into()
        }
        error => encoder_failure(error),
    }
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

fn read(bytes: &[u8], at: &ReceiptLocation) -> Result<Receipt, ReceiptError> {
    let malformed = |cause, pointer: &str| {
        ReceiptError::Refusal(ReceiptRefusal::Malformed {
            cause,
            location: ReceiptLocation {
                pointer: pointer.to_owned(),
                ..at.clone()
            },
        })
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
        error => reader_failure(error),
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
        .map_err(|error| whole_encoder_failure(error, at))?;
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
        quire_canonical::to_vec(&node, quire_canonical::Limits::new(bound)).map_err(encoder_failure)
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
    .map_err(encoder_failure)?;
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
                    if number
                        .text()
                        .parse::<u64>()
                        .is_ok_and(|value| value > 9_007_199_254_740_991)
                        || number
                            .text()
                            .parse::<i64>()
                            .is_ok_and(|value| value.unsigned_abs() > 9_007_199_254_740_991) =>
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
) -> Result<(), ReceiptError> {
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
                let receipt = read(bytes, &location(item, position, document, ""))
                    .map_err(ReceiptError::into_analyze_error)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn at() -> ReceiptLocation {
        location(
            0,
            0,
            &DocumentSelection {
                authority: "receipt-authority".to_owned(),
                identity: "receipt".to_owned(),
            },
            "",
        )
    }

    fn assert_allocation(error: ReceiptError, requested: usize) {
        let super::super::AnalyzeDocumentError::Refused(refusal) = error.into_analyze_error()
        else {
            panic!("allocation must remain a typed resource refusal");
        };
        let document = refusal.document();
        assert_eq!(document.category().exit_code(), 22);
        let value = serde_json::to_value(document).unwrap();
        assert_eq!(value["last_stage"], Value::Null);
        assert_eq!(value["diagnostics"][0]["code"], "resource_exhausted");
        assert_eq!(value["diagnostics"][0]["cause"], "allocation-failed");
        assert_eq!(
            value["diagnostics"][0]["details"],
            serde_json::json!({"requested": requested.to_string()})
        );
    }

    #[test]
    #[trace("FR-259", "FR-1046-AC-7")]
    fn canonical_allocation_errors_retain_the_requested_bytes() {
        // Typed boundary projection, not a claim to force host OOM. These are
        // the exact shared-library failures at reader/classifier and encoder
        // boundaries; whole, component and key sites use these adapters.
        assert_allocation(
            reader_failure(quire_canonical::ReadError::Allocation { requested: 417 }),
            417,
        );
        assert_allocation(
            classify(Err(quire_canonical::ReadError::Allocation {
                requested: 418,
            }))
            .unwrap_err(),
            418,
        );
        assert_allocation(
            whole_encoder_failure(quire_canonical::Error::Allocation { requested: 419 }, &at()),
            419,
        );
        assert_allocation(
            encoder_failure(quire_canonical::Error::Allocation { requested: 420 }),
            420,
        );
        let replay = classify(Err(quire_canonical::ReadError::Allocation {
            requested: 421,
        }))
        .unwrap_err()
        .into_replay_error();
        assert_eq!(replay.code().as_str(), "resource_exhausted");
        assert_eq!(replay.cause(), Some("allocation-failed"));
        let fsl_replay::ReplayRefusal::Admission(AdmissionFailure::Incomplete(record)) = replay
        else {
            panic!("replay must retain allocation evidence");
        };
        assert_eq!(
            record.fields,
            BTreeMap::from([("requested", "421".to_owned())])
        );
    }

    struct Deny;
    impl quire_canonical::Reservation for Deny {
        fn reserve(&mut self, _: usize) -> Result<(), quire_canonical::ReservationDenied> {
            Err(quire_canonical::ReservationDenied)
        }
    }

    #[test]
    #[trace("FR-259", "FR-277")]
    fn classifier_resource_denials_never_choose_an_ordinary_parser() {
        let bytes = br#"{"contract_version":"filament.effect-decision/v1"}"#;
        let error = quire_canonical::read_with_reservation(bytes, bytes.len() as u64, &mut Deny)
            .unwrap_err();
        assert!(matches!(
            error,
            quire_canonical::ReadError::ReservationDenied { .. }
        ));
        // The production classifier has no caller reservation. An unexpected
        // denial is an invariant, never Ok(None) or a fabricated caller limit.
        let error = classify(Err(error)).unwrap_err();
        assert!(matches!(
            error.into_analyze_error(),
            super::super::AnalyzeDocumentError::Fault
        ));
        let error = quire_canonical::read_bounded(bytes, bytes.len() as u64, 0).unwrap_err();
        assert!(matches!(error, quire_canonical::ReadError::Limit(_)));
        assert!(matches!(
            classify(Err(error)).unwrap_err().into_replay_error(),
            fsl_replay::ReplayRefusal::Fault(_)
        ));
        assert!(receipt_document(bytes).unwrap().is_some());
        assert!(receipt_document(b"{").unwrap().is_none());
        assert!(
            receipt_document(br#"{"format":"filament.state.snapshot/v1"}"#)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    #[trace("FR-259", "FR-1046-AC-7")]
    fn derived_encoder_caps_distinguish_noncanonical_input_from_invariants() {
        let document = quire_canonical::read(b"1e5", 3).unwrap();
        let error =
            quire_canonical::to_vec(&document, quire_canonical::Limits::new(3)).unwrap_err();
        let ReceiptError::Refusal(refusal) = whole_encoder_failure(error, &at()) else {
            panic!("expanding noncanonical spelling is input data");
        };
        assert_eq!(refusal.cause(), ReceiptCause::InvalidValue);

        let document =
            quire_canonical::read(br#"{"arguments":[],"operation":{},"value":true}"#, 100).unwrap();
        let root = document.root();
        let key = RequestKey {
            decision: root,
            grant: root,
            key: root,
        };
        // Generate real component and key encoder denials at the shared
        // boundary. Such denials after whole canonicality are internal faults.
        for error in [
            quire_canonical::to_vec(&root, quire_canonical::Limits::new(0)).unwrap_err(),
            quire_canonical::to_vec(&key, quire_canonical::Limits::new(0)).unwrap_err(),
        ] {
            assert!(matches!(error, quire_canonical::Error::Limit(_)));
            assert!(matches!(
                encoder_failure(error).into_analyze_error(),
                super::super::AnalyzeDocumentError::Fault
            ));
        }
        let mut bytes = Vec::new();
        let mut deny = Deny;
        let mut writer = quire_canonical::Writer::with_reservation(
            &mut bytes,
            quire_canonical::Limits::new(100),
            &mut deny,
        );
        let error = root.encode_into(&mut writer).unwrap_err();
        assert!(matches!(
            error,
            quire_canonical::Error::ReservationDenied { .. }
        ));
        assert!(matches!(encoder_failure(error), ReceiptError::Fault));
    }
}
