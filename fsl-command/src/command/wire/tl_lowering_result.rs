// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-931: the one declared type of the `native-tl-lowering-result/1`
//! document (ADR-037).
//!
//! An optional member is omitted when absent and never written as `null`; the
//! strict reader refuses `null` in its place. The two dispositions are told
//! apart by `disposition`, and a record is refused whose members are not the
//! ones its disposition carries.

use std::collections::BTreeMap;

use fsl_foundation::serde_object::omitted_not_null as not_null;
use fsl_foundation::wire_format::WireFormat;
use fsl_replay::spine::{
    read_route_result, FormatMarker, Label, Labelled, NativeRunLimits, NativeRunReadError,
    RouteResult,
};
use ix_trace_rs::implements;
use serde::{Deserialize, Serialize};

/// The marker of [`WireFormat::TlLoweringResult`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlLoweringMarker;

impl FormatMarker for TlLoweringMarker {
    const FORMAT: WireFormat = WireFormat::TlLoweringResult;
}

/// The `native-tl-lowering-result/1` document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TlLoweringDocument {
    /// The document's catalog label.
    pub format: Label<TlLoweringMarker>,
    /// The lowering record of the selected clause.
    pub tl_lowering: TlLoweringRecord,
}

impl Labelled for TlLoweringDocument {
    const FORMAT: WireFormat = WireFormat::TlLoweringResult;
}

/// The dimension in which a clause or correspondence is unsupported.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Dimension {
    /// The semantic profile differs.
    Profile,
    /// The clause reads an infinite trace.
    InfiniteTrace,
    /// Past and future operators mix, or the direction differs.
    Direction,
    /// The closure differs.
    Closure,
}

/// The fragment a formula lies in, by the direction of its operators.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fragment {
    /// Only past operators.
    BoundedPast,
    /// Only future operators.
    BoundedFuture,
}

/// The correspondence and fragment a formula is classified into.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Classification {
    /// The correspondence identity.
    pub correspondence: String,
    /// The fragment.
    pub fragment: Fragment,
}

/// The only `kind` of a classification fault.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClassificationLossKind {
    /// The lowered formula's classification differs from the clause's.
    #[serde(rename = "classification-loss")]
    ClassificationLoss,
}

/// A lowering whose classification differs from the native one.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassificationFault {
    /// Always `classification-loss`.
    pub kind: ClassificationLossKind,
    /// The clause.
    pub clause: String,
    /// The native classification.
    pub native: Classification,
    /// The lowered formula's classification.
    pub lowered: Classification,
}

/// The only `kind` of a cross-evaluation fault.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CorrespondenceLossKind {
    /// The native and TL verdicts differ at a position.
    #[serde(rename = "correspondence-loss")]
    CorrespondenceLoss,
}

/// A native verdict: a boolean, or the outcome's name when it is neither
/// holding nor violated.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NativeVerdict {
    /// Holds or is violated.
    Holds(bool),
    /// Any other outcome, by its name.
    Other(String),
}

/// One position at which the native and TL verdicts differ.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossEvaluationFault {
    /// Always `correspondence-loss`.
    pub kind: CorrespondenceLossKind,
    /// The clause.
    pub clause: String,
    /// The trace index.
    pub trace: usize,
    /// The position.
    pub position: usize,
    /// The native verdict.
    pub native: NativeVerdict,
    /// The TL verdict.
    pub tl: bool,
}

/// The comparison of the native and TL verdicts over the traces.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossEvaluation {
    /// The traces compared.
    pub traces: usize,
    /// The positions compared.
    pub positions: usize,
    /// Whether every position agrees.
    pub agree: bool,
    /// The positions that disagree.
    pub faults: Vec<CrossEvaluationFault>,
}

/// A clause that lowers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportedLowering {
    /// The clause.
    pub clause: String,
    /// The correspondence identity the formula is lowered under.
    pub correspondence: String,
    /// The lowered formula, printed.
    pub formula: String,
    /// The atom texts by proposition name.
    pub atoms: BTreeMap<String, String>,
    /// The classification, when it matches the lowered formula's.
    pub classification: Option<Classification>,
    /// The classification fault, when it does not.
    pub classification_fault: Option<ClassificationFault>,
    /// The cross-evaluation, when the run holds traces.
    pub cross_evaluation: Option<CrossEvaluation>,
}

/// A clause that does not lower under the correspondence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedLowering {
    /// The clause.
    pub clause: String,
    /// The named correspondence, when the request named one.
    pub correspondence: Option<String>,
    /// The classification's correspondence, when it differs from the named one.
    pub classified: Option<String>,
    /// The dimensions in which it is unsupported.
    pub dimensions: Vec<Dimension>,
}

/// The lowering record of one clause.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Wire", into = "Wire")]
pub enum TlLoweringRecord {
    /// The clause lowers.
    Supported(Box<SupportedLowering>),
    /// The clause does not lower.
    Unsupported(UnsupportedLowering),
}

impl TlLoweringRecord {
    /// The clause.
    pub fn clause(&self) -> &str {
        match self {
            Self::Supported(record) => &record.clause,
            Self::Unsupported(record) => &record.clause,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Disposition {
    Supported,
    Unsupported,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    clause: String,
    disposition: Disposition,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "not_null"
    )]
    correspondence: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "not_null"
    )]
    dimensions: Option<Vec<Dimension>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "not_null"
    )]
    classified: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "not_null"
    )]
    formula: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "not_null"
    )]
    atoms: Option<BTreeMap<String, String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "not_null"
    )]
    classification: Option<Classification>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "not_null"
    )]
    classification_fault: Option<ClassificationFault>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "not_null"
    )]
    cross_evaluation: Option<CrossEvaluation>,
}

impl From<TlLoweringRecord> for Wire {
    fn from(record: TlLoweringRecord) -> Self {
        match record {
            TlLoweringRecord::Supported(record) => {
                let SupportedLowering {
                    clause,
                    correspondence,
                    formula,
                    atoms,
                    classification,
                    classification_fault,
                    cross_evaluation,
                } = *record;
                Self {
                    clause,
                    disposition: Disposition::Supported,
                    correspondence: Some(correspondence),
                    dimensions: None,
                    classified: None,
                    formula: Some(formula),
                    atoms: Some(atoms),
                    classification,
                    classification_fault,
                    cross_evaluation,
                }
            }
            TlLoweringRecord::Unsupported(record) => Self {
                clause: record.clause,
                disposition: Disposition::Unsupported,
                correspondence: record.correspondence,
                dimensions: Some(record.dimensions),
                classified: record.classified,
                formula: None,
                atoms: None,
                classification: None,
                classification_fault: None,
                cross_evaluation: None,
            },
        }
    }
}

impl TryFrom<Wire> for TlLoweringRecord {
    type Error = String;

    fn try_from(wire: Wire) -> Result<Self, Self::Error> {
        let Wire {
            clause,
            disposition,
            correspondence,
            dimensions,
            classified,
            formula,
            atoms,
            classification,
            classification_fault,
            cross_evaluation,
        } = wire;
        match disposition {
            Disposition::Supported => {
                if dimensions.is_some() || classified.is_some() {
                    return Err(
                        "a supported lowering carries no `dimensions` or `classified`".into(),
                    );
                }
                if classification.is_some() == classification_fault.is_some() {
                    return Err(
                        "a supported lowering carries exactly one of `classification` and `classification_fault`"
                            .into(),
                    );
                }
                Ok(Self::Supported(Box::new(SupportedLowering {
                    clause,
                    correspondence: correspondence
                        .ok_or("a supported lowering needs its `correspondence`")?,
                    formula: formula.ok_or("a supported lowering needs its `formula`")?,
                    atoms: atoms.ok_or("a supported lowering needs its `atoms`")?,
                    classification,
                    classification_fault,
                    cross_evaluation,
                })))
            }
            Disposition::Unsupported => {
                if formula.is_some()
                    || atoms.is_some()
                    || classification.is_some()
                    || classification_fault.is_some()
                    || cross_evaluation.is_some()
                {
                    return Err("an unsupported lowering carries no formula, atoms, classification or cross-evaluation".into());
                }
                Ok(Self::Unsupported(UnsupportedLowering {
                    clause,
                    correspondence,
                    classified,
                    dimensions: dimensions
                        .ok_or("an unsupported lowering needs its `dimensions`")?,
                }))
            }
        }
    }
}

/// Read the result of a temporal lowering run: its own document, or the
/// `native-run-result/2` command error of a refused request.
///
/// # Errors
///
/// [`NativeRunReadError`] for another label, a refused bound, or an unknown,
/// missing or mistyped member, naming that member.
#[implements("FR-931-AC-20")]
pub fn read_tl_lowering_result(
    bytes: &[u8],
    limits: NativeRunLimits,
) -> Result<RouteResult<TlLoweringDocument>, NativeRunReadError> {
    read_route_result(bytes, limits)
}
