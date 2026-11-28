// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-267: the one declared type of the `native-run-result/2` clause-run
//! document (FSpec FR-352 `Members`, `Outcome axes` and the disposition
//! members), ADR-037's FSL wire module for this format.
//!
//! The production writer constructs these types and the strict reader
//! deserializes the same types, so a member added or retyped here changes both
//! at once. The byte-stage bounded admission, the budgets and the validators
//! stay in the parent module; this module holds the shape only.
//!
//! The temporal row (`disposition.temporal`), the progress ledger, the
//! separating witness and the correction members keep the owner modules that
//! write and read them; they are carried here as the JSON value those modules
//! produce and decode.

use std::collections::BTreeMap;

use filament_checked::key::DeclarationKey;
use fsl_foundation::diagnostic::Code;
use fsl_foundation::wire_format::WireFormat;
use fsl_foundation::LocatedSpan;
use fsl_semantics::model::observation::{
    DocumentRef, FrameChange, FrameWitness, LinkDifference, SelectedObject,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::NativeRunReadError;
use crate::proof_result::SettlementBasis;
use crate::result::RecordWire;

mod extraction;
mod locus;

pub(super) use extraction::validate_extraction;
pub use extraction::{
    ExtractionBinding, ExtractionBody, ExtractionMap, ExtractionOriginal, ExtractionOutcome,
    ExtractionPosition, ExtractionRecord, ExtractionRegion, ExtractionSegment, ExtractionSegments,
    ExtractionSpan,
};
pub(super) use locus::source_identity_uri;
pub use locus::{DocumentObject, Located, RefusalLocus, SourceRef};

pub(super) use fsl_foundation::serde_object::present;

/// A member present as a JSON value other than `null`: an absent optional
/// member is omitted, never written as `null` (FSpec FR-352-AC-8).
fn present_non_null<'de, D>(decoder: D) -> Result<Option<serde_json::Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match serde_json::Value::deserialize(decoder)? {
        serde_json::Value::Null => Err(serde::de::Error::custom(
            "an absent optional member is omitted, never `null`",
        )),
        value => Ok(Some(value)),
    }
}

/// A member present as a closed JSON object (Serde's derived struct decoder
/// also accepts a positional array).
fn present_object<'de, D, T>(decoder: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    fsl_foundation::serde_object::from_object::<D, T>(decoder).map(Some)
}

/// The one `format` member of the wire: it writes the catalog's label for
/// [`WireFormat::RunResult`] and reads nothing else.
pub(super) type RunResultFormat = super::label::Label<super::label::RunResultMarker>;

/// The disposition stage label.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Stage {
    Compile,
    Select,
    Admit,
    Evaluate,
}

impl Stage {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Select => "select",
            Self::Admit => "admit",
            Self::Evaluate => "evaluate",
        }
    }

    fn of(stage: super::super::ClauseRunStage) -> Self {
        match stage {
            super::super::ClauseRunStage::Compile => Self::Compile,
            super::super::ClauseRunStage::Select => Self::Select,
            super::super::ClauseRunStage::Admit => Self::Admit,
            super::super::ClauseRunStage::Evaluate => Self::Evaluate,
        }
    }
}

/// The outcome category label (FSpec FR-352's catalog categories).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Category {
    Success,
    Violation,
    Undefined,
    Refusal,
    Unsupported,
    Incomplete,
    Inconclusive,
    InternalFailure,
    Pending,
    Indeterminate,
}

impl Category {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Indeterminate => "indeterminate",
            Self::Success => "success",
            Self::Violation => "violation",
            Self::Undefined => "undefined",
            Self::Refusal => "refusal",
            Self::Unsupported => "unsupported",
            Self::Incomplete => "incomplete",
            Self::Inconclusive => "inconclusive",
            Self::InternalFailure => "internal-failure",
        }
    }

    pub(super) fn of(category: filament_semantic_value::catalog::Category) -> Self {
        use filament_semantic_value::catalog::Category as Catalog;
        match category {
            Catalog::Success => Self::Success,
            Catalog::Violation => Self::Violation,
            Catalog::Undefined => Self::Undefined,
            Catalog::Refusal => Self::Refusal,
            Catalog::Unsupported => Self::Unsupported,
            Catalog::Incomplete => Self::Incomplete,
            Catalog::Pending => Self::Pending,
            Catalog::Indeterminate => Self::Indeterminate,
            Catalog::Inconclusive => Self::Inconclusive,
            Catalog::InternalFailure => Self::InternalFailure,
        }
    }
}

/// One evaluation counter and the amount it consumed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CounterUsage {
    pub kind: String,
    pub consumed: u64,
}

/// The admission bytes, objects and values consumed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AdmissionUsage {
    pub document_bytes: u64,
    pub objects: u64,
    pub values: u64,
}

/// FSpec FR-352's closed accounting object.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Usage {
    pub admission: AdmissionUsage,
    pub evaluation_admissions: u64,
    pub evaluation_consumed: Vec<CounterUsage>,
}

/// The tagged value of a completed outcome.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum OutcomeValue {
    Boolean { value: bool },
    Integer { decimal: String },
}

/// The FR-100 outcome a disposition retains.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Outcome {
    Completed {
        value: OutcomeValue,
    },
    Refused {
        code: String,
        cause: String,
        #[serde(
            default,
            deserialize_with = "present",
            skip_serializing_if = "Option::is_none"
        )]
        fields: Option<BTreeMap<String, String>>,
        #[serde(
            default,
            deserialize_with = "present",
            skip_serializing_if = "Option::is_none"
        )]
        locus: Option<CallLocusRecord>,
        #[serde(
            default,
            deserialize_with = "present",
            skip_serializing_if = "Option::is_none"
        )]
        location: Option<Box<CallLocation>>,
    },
    Undefined {
        reason: String,
    },
    Incomplete {
        limit: String,
    },
}

impl Outcome {
    pub(super) fn undefined(reason: &str) -> Self {
        Self::Undefined {
            reason: reason.to_owned(),
        }
    }

    pub(super) fn incomplete(limit: &str) -> Self {
        Self::Incomplete {
            limit: limit.to_owned(),
        }
    }

    /// A refusal with no locus or location.
    pub(super) fn refused(
        code: &str,
        cause: &str,
        fields: Option<&BTreeMap<&'static str, String>>,
    ) -> Self {
        Self::Refused {
            code: code.to_owned(),
            cause: cause.to_owned(),
            fields: fields.map(text_fields),
            locus: None,
            location: None,
        }
    }
}

/// A text-only field map, owned.
pub(super) fn text_fields(fields: &BTreeMap<&'static str, String>) -> BTreeMap<String, String> {
    fields
        .iter()
        .map(|(name, text)| ((*name).to_owned(), text.clone()))
        .collect()
}

/// A decoded zero-based byte offset with its one-based line and column.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunPosition {
    /// Zero-based byte offset.
    pub byte: usize,
    /// One-based line number.
    pub line: usize,
    /// One-based column number.
    pub column: usize,
}

/// A decoded source span.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunSpan {
    /// First position in the source region.
    pub start: RunPosition,
    /// Position after the source region.
    pub end: RunPosition,
}

impl From<LocatedSpan> for RunSpan {
    fn from(span: LocatedSpan) -> Self {
        let position = |at: fsl_foundation::source::Position| RunPosition {
            byte: at.byte,
            line: at.line,
            column: at.column,
        };
        Self {
            start: position(span.start),
            end: position(span.end),
        }
    }
}

/// Source locus attached to an evaluation refusal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallLocusRecord {
    /// Authored path of the source that owns this span.
    pub file: String,
    /// The located source span.
    pub span: RunSpan,
}

/// `requires` or `ensures`, the clause list of a model operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationClause {
    /// A `requires` clause.
    Requires,
    /// An `ensures` clause.
    Ensures,
}

impl OperationClause {
    /// The wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requires => "requires",
            Self::Ensures => "ensures",
        }
    }
}

impl From<filament_semantic_value::location::OperationClauseKind> for OperationClause {
    fn from(kind: filament_semantic_value::location::OperationClauseKind) -> Self {
        match kind {
            filament_semantic_value::location::OperationClauseKind::Requires => Self::Requires,
            filament_semantic_value::location::OperationClauseKind::Ensures => Self::Ensures,
        }
    }
}

/// Declaration origin of an expression location.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CallOrigin {
    /// Function body clause.
    Body {
        /// Function name.
        function: String,
        /// Body clause index.
        index: usize,
    },
    /// Function measure clause.
    Measure {
        /// Function name.
        function: String,
        /// Measure clause index.
        index: usize,
    },
    /// Standalone expression.
    Expression,
    /// Named type declaration.
    TypeDeclaration {
        /// Type name.
        name: String,
    },
    /// State clause declaration.
    StateClause {
        /// State clause name.
        clause: String,
        /// Clause index.
        index: usize,
    },
    /// Protocol attempt declaration.
    ProtocolAttempt {
        /// Protocol index.
        protocol: usize,
        /// Attempt index.
        attempt: usize,
    },
    /// One of a model operation's kept `requires` or `ensures` clauses.
    ModelOperationClause {
        /// Operation declaration node identity.
        operation: String,
        /// `requires` or `ensures`.
        clause: OperationClause,
        /// Zero-based position in that clause list.
        position: usize,
    },
}

/// Expression location within a declaration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallLocation {
    /// Declaration origin of the expression.
    pub origin: CallOrigin,
    /// Child-index path within the origin.
    pub path: Vec<usize>,
}

impl CallLocation {
    /// The typed location this record restores.
    #[ix_trace_rs::string_edge]
    pub(super) fn typed(&self) -> filament_semantic_value::location::ExpressionLocation {
        use filament_semantic_value::location::{
            ExpressionLocation as Location, ExpressionOwner as Origin, OperationClauseKind as Kind,
        };
        let origin = match &self.origin {
            CallOrigin::Body { function, index } => Origin::Body {
                function: function.clone(),
                index: *index,
            },
            CallOrigin::Measure { function, index } => Origin::Measure {
                function: function.clone(),
                index: *index,
            },
            CallOrigin::Expression => Origin::Expression,
            CallOrigin::TypeDeclaration { name } => Origin::TypeDeclaration { name: name.clone() },
            CallOrigin::StateClause { clause, index } => Origin::StateClause {
                clause: clause.clone(),
                index: *index,
            },
            CallOrigin::ProtocolAttempt { protocol, attempt } => Origin::ProtocolAttempt {
                protocol: *protocol,
                attempt: *attempt,
            },
            CallOrigin::ModelOperationClause {
                operation,
                clause,
                position,
            } => Origin::ModelOperationClause {
                operation: operation.clone(),
                kind: match clause {
                    OperationClause::Requires => Kind::Requires,
                    OperationClause::Ensures => Kind::Ensures,
                },
                position: *position,
            },
        };
        Location::at(origin, &self.path)
    }
}

impl From<&filament_semantic_value::location::ExpressionLocation> for CallLocation {
    fn from(location: &filament_semantic_value::location::ExpressionLocation) -> Self {
        use filament_semantic_value::location::ExpressionOwner as Origin;
        let origin = match &location.origin {
            Origin::Body { function, index } => CallOrigin::Body {
                function: function.clone(),
                index: *index,
            },
            Origin::Measure { function, index } => CallOrigin::Measure {
                function: function.clone(),
                index: *index,
            },
            Origin::Expression => CallOrigin::Expression,
            Origin::TypeDeclaration { name } => CallOrigin::TypeDeclaration { name: name.clone() },
            Origin::StateClause { clause, index } => CallOrigin::StateClause {
                clause: clause.clone(),
                index: *index,
            },
            Origin::ProtocolAttempt { protocol, attempt } => CallOrigin::ProtocolAttempt {
                protocol: *protocol,
                attempt: *attempt,
            },
            Origin::ModelOperationClause {
                operation,
                kind,
                position,
            } => CallOrigin::ModelOperationClause {
                operation: operation.clone(),
                clause: OperationClause::from(*kind),
                position: *position,
            },
        };
        Self {
            origin,
            path: location.path(),
        }
    }
}

/// The stable identifiers of an internal failure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InternalFault {
    pub stage: String,
    pub invariant: String,
}

impl InternalFault {
    pub(super) fn new(stage: &str, invariant: &str) -> Self {
        Self {
            stage: stage.to_owned(),
            invariant: invariant.to_owned(),
        }
    }
}

/// One member of a refusal record's `fields`: text, or the list of FR-645's
/// `expected` trace kinds.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldValue {
    /// A text member.
    Text(String),
    /// A list member, read as its kinds joined by commas.
    List(Vec<String>),
}

impl FieldValue {
    /// The member as one text, a list joined by commas.
    pub fn joined(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::Text(text) => std::borrow::Cow::Borrowed(text),
            Self::List(kinds) => std::borrow::Cow::Owned(kinds.join(",")),
        }
    }

    /// The text of a text member.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::List(_) => None,
        }
    }
}

/// A refusal record of a pre-evaluation, frame or trace refusal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RefusalRecord {
    pub code: String,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub cause: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub fields: Option<BTreeMap<String, FieldValue>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub message: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub locus: Option<RefusalLocus>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub frame_witness: Option<FrameWitnessRecord>,
}

impl RefusalRecord {
    pub(super) fn new(code: &str, cause: Option<&str>) -> Self {
        Self {
            code: code.to_owned(),
            cause: cause.map(str::to_owned),
            fields: None,
            message: None,
            locus: None,
            frame_witness: None,
        }
    }

    /// The record with these text fields.
    pub(super) fn fields(mut self, fields: BTreeMap<&'static str, String>) -> Self {
        self.fields = Some(
            fields
                .into_iter()
                .map(|(name, text)| (name.to_owned(), FieldValue::Text(text)))
                .collect(),
        );
        self
    }
}

/// A refusal's or outcome's fields as one text each.
#[derive(Clone, Copy, Debug)]
pub enum FieldsView<'a> {
    /// The fields of a refusal record.
    Refusal(&'a BTreeMap<String, FieldValue>),
    /// The fields of a refused outcome.
    Outcome(&'a BTreeMap<String, String>),
}

impl<'a> FieldsView<'a> {
    /// The member `name` as text; a list member reads as its kinds joined by
    /// commas.
    pub fn get(&self, name: &str) -> Option<std::borrow::Cow<'a, str>> {
        match *self {
            Self::Refusal(fields) => fields.get(name).map(FieldValue::joined),
            Self::Outcome(fields) => fields
                .get(name)
                .map(|text| std::borrow::Cow::Borrowed(text.as_str())),
        }
    }

    /// The number of members.
    pub fn len(&self) -> usize {
        match *self {
            Self::Refusal(fields) => fields.len(),
            Self::Outcome(fields) => fields.len(),
        }
    }

    /// Whether there is no member.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The members in name order, each as text.
    pub fn iter(&self) -> impl Iterator<Item = (&'a str, std::borrow::Cow<'a, str>)> + 'a {
        let view = *self;
        let names: Vec<&'a str> = match view {
            Self::Refusal(fields) => fields.keys().map(String::as_str).collect(),
            Self::Outcome(fields) => fields.keys().map(String::as_str).collect(),
        };
        names
            .into_iter()
            .filter_map(move |name| view.get(name).map(|text| (name, text)))
    }
}

/// A document identity, its two FR-001 labels.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DocumentRefWire {
    pub authority: String,
    pub identity: String,
}

impl DocumentRefWire {
    #[ix_trace_rs::string_edge]
    pub(super) fn typed(&self) -> DocumentRef {
        DocumentRef {
            authority: self.authority.clone(),
            identity: self.identity.clone(),
        }
    }
}

impl From<&DocumentRef> for DocumentRefWire {
    fn from(value: &DocumentRef) -> Self {
        Self {
            authority: value.authority.clone(),
            identity: value.identity.clone(),
        }
    }
}

/// A declaration key, its package and node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclarationKeyWire {
    pub package: String,
    pub node: String,
}

impl DeclarationKeyWire {
    pub(super) fn typed(&self) -> DeclarationKey {
        DeclarationKey {
            package: self.package.clone(),
            node: self.node.clone(),
        }
    }
}

impl From<&DeclarationKey> for DeclarationKeyWire {
    fn from(value: &DeclarationKey) -> Self {
        Self {
            package: value.package.clone(),
            node: value.node.clone(),
        }
    }
}

/// A population and an object key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObjectRefWire {
    pub population: String,
    pub key: String,
}

impl ObjectRefWire {
    fn typed(&self) -> SelectedObject {
        SelectedObject {
            population: self.population.clone(),
            key: self.key.clone(),
        }
    }
}

impl From<&SelectedObject> for ObjectRefWire {
    fn from(value: &SelectedObject) -> Self {
        Self {
            population: value.population.clone(),
            key: value.key.clone(),
        }
    }
}

/// Whether a link was added or removed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum LinkDifferenceWire {
    Added,
    Removed,
}

/// FR-115's one change outside the frame.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum FrameChangeWire {
    FieldWrite {
        object: String,
        field: String,
        modifies: Vec<DeclarationKeyWire>,
    },
    Created {
        object: String,
        type_name: DeclarationKeyWire,
        creates: Vec<DeclarationKeyWire>,
    },
    Deleted {
        object: String,
        type_name: DeclarationKeyWire,
        deletes: Vec<DeclarationKeyWire>,
    },
    Retyped {
        object: String,
        pre_type: DeclarationKeyWire,
        post_type: DeclarationKeyWire,
    },
    Link {
        relationship: DeclarationKeyWire,
        difference: LinkDifferenceWire,
        source: ObjectRefWire,
        target: ObjectRefWire,
        modifies: Vec<DeclarationKeyWire>,
    },
}

impl From<&FrameChange> for FrameChangeWire {
    fn from(value: &FrameChange) -> Self {
        let keys = |keys: &[DeclarationKey]| keys.iter().map(DeclarationKeyWire::from).collect();
        match value {
            FrameChange::FieldWrite {
                object,
                field,
                modifies,
            } => Self::FieldWrite {
                object: object.clone(),
                field: field.clone(),
                modifies: keys(modifies),
            },
            FrameChange::Created {
                object,
                type_name,
                creates,
            } => Self::Created {
                object: object.clone(),
                type_name: DeclarationKeyWire::from(type_name),
                creates: keys(creates),
            },
            FrameChange::Deleted {
                object,
                type_name,
                deletes,
            } => Self::Deleted {
                object: object.clone(),
                type_name: DeclarationKeyWire::from(type_name),
                deletes: keys(deletes),
            },
            FrameChange::Retyped {
                object,
                pre_type,
                post_type,
            } => Self::Retyped {
                object: object.clone(),
                pre_type: DeclarationKeyWire::from(pre_type),
                post_type: DeclarationKeyWire::from(post_type),
            },
            FrameChange::Link {
                relationship,
                difference,
                source,
                target,
                modifies,
            } => Self::Link {
                relationship: DeclarationKeyWire::from(relationship),
                difference: match difference {
                    LinkDifference::Added => LinkDifferenceWire::Added,
                    LinkDifference::Removed => LinkDifferenceWire::Removed,
                },
                source: ObjectRefWire::from(source),
                target: ObjectRefWire::from(target),
                modifies: keys(modifies),
            },
        }
    }
}

impl FrameChangeWire {
    fn typed(&self) -> FrameChange {
        let keys =
            |keys: &[DeclarationKeyWire]| keys.iter().map(DeclarationKeyWire::typed).collect();
        match self {
            Self::FieldWrite {
                object,
                field,
                modifies,
            } => FrameChange::FieldWrite {
                object: object.clone(),
                field: field.clone(),
                modifies: keys(modifies),
            },
            Self::Created {
                object,
                type_name,
                creates,
            } => FrameChange::Created {
                object: object.clone(),
                type_name: type_name.typed(),
                creates: keys(creates),
            },
            Self::Deleted {
                object,
                type_name,
                deletes,
            } => FrameChange::Deleted {
                object: object.clone(),
                type_name: type_name.typed(),
                deletes: keys(deletes),
            },
            Self::Retyped {
                object,
                pre_type,
                post_type,
            } => FrameChange::Retyped {
                object: object.clone(),
                pre_type: pre_type.typed(),
                post_type: post_type.typed(),
            },
            Self::Link {
                relationship,
                difference,
                source,
                target,
                modifies,
            } => FrameChange::Link {
                relationship: relationship.typed(),
                difference: match difference {
                    LinkDifferenceWire::Added => LinkDifference::Added,
                    LinkDifferenceWire::Removed => LinkDifference::Removed,
                },
                source: source.typed(),
                target: target.typed(),
                modifies: keys(modifies),
            },
        }
    }
}

/// FR-115's evaluated frame witness.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FrameWitnessRecord {
    pub code: String,
    pub cause: String,
    pub invocation: DocumentRefWire,
    pub pre: DocumentRefWire,
    pub post: DocumentRefWire,
    pub population: String,
    pub change: FrameChangeWire,
}

impl From<&FrameWitness> for FrameWitnessRecord {
    fn from(value: &FrameWitness) -> Self {
        Self {
            code: value.code.as_str().to_owned(),
            cause: value.cause.to_owned(),
            invocation: DocumentRefWire::from(&value.invocation),
            pre: DocumentRefWire::from(&value.pre),
            post: DocumentRefWire::from(&value.post),
            population: value.population.clone(),
            change: FrameChangeWire::from(&value.change),
        }
    }
}

impl FrameWitnessRecord {
    #[ix_trace_rs::string_edge]
    pub(super) fn typed(&self) -> Result<FrameWitness, NativeRunReadError> {
        if self.code != "frame_violation" || self.cause != "unauthorized-change" {
            return Err(NativeRunReadError::Invalid {
                member: "disposition.refusal.frame_witness",
                reason: "frame witness code or cause disagrees with catalog",
            });
        }
        if let FrameChangeWire::Link { source, .. } = &self.change {
            if source.population != self.population {
                return Err(NativeRunReadError::Invalid {
                    member: "disposition.refusal.frame_witness.population",
                    reason: "link witness population is not its source end's population",
                });
            }
        }
        Ok(FrameWitness {
            code: Code::FrameViolation,
            cause: "unauthorized-change",
            invocation: self.invocation.typed(),
            pre: self.pre.typed(),
            post: self.post.typed(),
            population: self.population.clone(),
            change: self.change.typed(),
        })
    }
}

/// An operation contract violation's named clause.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OperationContract {
    pub operation: DeclarationKeyWire,
    pub name: String,
    pub kind: OperationClause,
    pub position: usize,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub artifact: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub span: Option<RunSpan>,
    pub location: CallLocation,
    pub member: String,
}

impl From<&crate::spine::clause::OperationContractViolation> for OperationContract {
    fn from(value: &crate::spine::clause::OperationContractViolation) -> Self {
        Self {
            operation: DeclarationKeyWire::from(&value.operation),
            name: value.operation_name.clone(),
            kind: OperationClause::from(value.kind),
            position: value.position,
            artifact: value.artifact.clone(),
            span: value.span.map(RunSpan::from),
            location: CallLocation::from(
                &filament_semantic_value::location::ExpressionLocation::root(value.origin.clone()),
            ),
            member: value.member.to_string(),
        }
    }
}

/// FSpec FR-352's `disposition` member.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Disposition {
    pub stage: Stage,
    pub category: Category,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub truth: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub outcome: Option<Outcome>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub internal_fault: Option<InternalFault>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub refusal: Option<RefusalRecord>,
    /// The temporal row, which `temporal` writes and decodes.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub temporal: Option<serde_json::Value>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub operation_contract: Option<OperationContract>,
}

impl Disposition {
    /// An empty disposition at `stage` with the catalog `category`.
    pub(super) fn new(stage: super::super::ClauseRunStage, category: Category) -> Self {
        Self {
            stage: Stage::of(stage),
            category,
            truth: None,
            outcome: None,
            internal_fault: None,
            refusal: None,
            temporal: None,
            operation_contract: None,
        }
    }
}

/// The effective limits of a `model` trace (FR-641-AC-8).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ModelLimits {
    #[serde(rename = "expansion.max_candidates")]
    pub max_candidates: String,
}

/// One decision-support pair of a finite-window result (FR-501).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SupportWire {
    pub position: u64,
    pub atom: String,
}

/// One `late` entry of a finite-window result (FR-501).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LateWire {
    pub position: u64,
    pub observation: String,
    pub stamp: u64,
    pub arrival: u64,
    pub anchors: Vec<u64>,
}

/// One `admitted_late` entry of a superseding repair (FR-526).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AdmittedWire {
    pub position: u64,
    pub observation: String,
}

/// The clause-run form of `native-run-result/2` (FSpec FR-352 `Members`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ClauseRunDocument {
    pub format: RunResultFormat,
    pub disposition: Disposition,
    pub usage: Usage,
    pub basis: SettlementBasis,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub witness: Option<RecordWire<Box<RawValue>>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub extraction: Option<ExtractionRecord>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub ledger: Option<serde_json::Value>,
    /// FR-641-AC-8, FSpec FR-352-AC-19: the effective limits a `model`
    /// trace ran under, defaults written out, so runs that differ only in a
    /// limit have different result identities. A sample trace writes none.
    #[serde(
        default,
        deserialize_with = "present_object",
        skip_serializing_if = "Option::is_none"
    )]
    pub limits: Option<ModelLimits>,
    pub axes: crate::OutcomeAxes,
    /// FR-501 "Result report": the decision support of a finite-window
    /// result, as `(position, atom)` pairs in canonical positions.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub support: Option<Vec<SupportWire>>,
    /// FR-501 "Request-file spelling": the observations a late-data cutoff
    /// excluded, written whenever a cutoff is selected.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub late: Option<Vec<LateWire>>,
    /// FR-526: the late observations a superseding repair admitted.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub admitted_late: Option<Vec<AdmittedWire>>,
    /// FR-526: the result's identity, its input identity, its relation, its
    /// lineage and the updated evidence ledger.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub result_identity: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub input_identity: Option<String>,
    #[serde(
        default,
        deserialize_with = "present_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub relation: Option<serde_json::Value>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub lineage: Option<u64>,
    #[serde(
        default,
        deserialize_with = "present_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub evidence_ledger: Option<serde_json::Value>,
}
