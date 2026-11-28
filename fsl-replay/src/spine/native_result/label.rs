// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-037: the one label rule of the native result documents and the byte
//! stages every labelled document reader shares.
//!
//! A document type carries its `format` member as a [`Label`] over a
//! [`FormatMarker`], so the label is written and admitted by one rule
//! ([`WireFormat::admit`]) and never as a literal. [`read_labelled`] is the
//! one bounded byte stage; [`read_route_result`] is the one dispatch of a
//! route whose result is its own document or a `native-run-result/2` command
//! error.

use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;

use fsl_foundation::wire_format::WireFormat;
use quire_canonical::{Document, Node};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::{
    read_native_run_result_bounded, read_typed, CommandErrorRecord, NativeRunLimits,
    NativeRunReadError, NativeRunResult, WorkingBudget,
};

/// The catalog format one document type carries as its `format` member.
pub trait FormatMarker {
    /// The one label this marker writes and admits.
    const FORMAT: WireFormat;
}

/// The `format` member of a document: writes the marker's label and reads
/// nothing else.
pub struct Label<F>(PhantomData<fn() -> F>);

impl<F> Label<F> {
    /// The label of `F`.
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<F> Default for Label<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F> Clone for Label<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> Copy for Label<F> {}

impl<F> fmt::Debug for Label<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Label")
    }
}

impl<F> PartialEq for Label<F> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl<F> Eq for Label<F> {}

impl<F> Hash for Label<F> {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

impl<F: FormatMarker> Serialize for Label<F> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        F::FORMAT.serialize(serializer)
    }
}

impl<'de, F: FormatMarker> Deserialize<'de> for Label<F> {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let found = String::deserialize(decoder)?;
        F::FORMAT.admit(&found).map_err(serde::de::Error::custom)?;
        Ok(Self::new())
    }
}

/// The marker of [`WireFormat::RunResult`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunResultMarker;

impl FormatMarker for RunResultMarker {
    const FORMAT: WireFormat = WireFormat::RunResult;
}

/// A document whose root `format` member is its one catalog label.
pub trait Labelled: DeserializeOwned {
    /// The label the document's `format` member admits.
    const FORMAT: WireFormat;
}

pub(super) fn root_label(tree: &Document) -> String {
    tree.root()
        .get("format")
        .and_then(|node| match node.node() {
            Node::String(value) => Some(value.to_owned()),
            _ => None,
        })
        .unwrap_or_else(|| "<missing>".to_owned())
}

pub(super) fn open_tree(
    bytes: &[u8],
    limits: NativeRunLimits,
    budget: &mut WorkingBudget,
) -> Result<Document, NativeRunReadError> {
    match quire_canonical::read_with_reservation(bytes, limits.max_input_bytes, budget) {
        Err(quire_canonical::ReadError::ReservationDenied { .. }) => Err(budget.denied()),
        other => Ok(other?),
    }
}

/// The `format` of the document `bytes` holds, under the caller's bounds.
pub(super) fn read_root_label(
    bytes: &[u8],
    limits: NativeRunLimits,
) -> Result<String, NativeRunReadError> {
    let mut budget = WorkingBudget::new(limits.max_working_bytes);
    Ok(root_label(&open_tree(bytes, limits, &mut budget)?))
}

/// Read one labelled document under the caller's encoded-input and
/// working-byte bounds: the bounded tree, the root label checked by the
/// document's own format, the byte length charged, then the strict typed read
/// that names the failing member.
///
/// # Errors
///
/// [`NativeRunReadError::UnsupportedFormat`] for any other label, the bound
/// refusals, and [`NativeRunReadError::MalformedMember`] for an unknown,
/// missing or mistyped member.
#[ix_trace_rs::string_edge]
pub fn read_labelled<T: Labelled>(
    bytes: &[u8],
    limits: NativeRunLimits,
) -> Result<T, NativeRunReadError> {
    let mut budget = WorkingBudget::new(limits.max_working_bytes);
    let tree = open_tree(bytes, limits, &mut budget)?;
    let found = root_label(&tree);
    if T::FORMAT.admit(&found).is_err() {
        return Err(NativeRunReadError::UnsupportedFormat(found));
    }
    budget.charge(u64::try_from(bytes.len()).unwrap_or(u64::MAX))?;
    read_typed(bytes)
}

/// What a route that has its own result document returns: that record, or the
/// `native-run-result/2` command error of a refused request.
pub enum RouteResult<R> {
    /// The route's own document.
    Record(R),
    /// A command error of the `native-run-result/2` format (FR-267).
    CommandError(Box<CommandErrorRecord>),
}

/// Read a route's result: its own document under [`Labelled::FORMAT`], or a
/// `native-run-result/2` command error. A clause-run or protocol document of
/// that format is not a result of the route and is refused.
///
/// # Errors
///
/// [`NativeRunReadError::UnsupportedFormat`] for any other label, and the
/// errors of the reader the label selects.
#[ix_trace_rs::string_edge]
pub fn read_route_result<T: Labelled>(
    bytes: &[u8],
    limits: NativeRunLimits,
) -> Result<RouteResult<T>, NativeRunReadError> {
    let label = read_root_label(bytes, limits)?;
    match WireFormat::from_label(&label) {
        Some(format) if format == T::FORMAT => {
            read_labelled(bytes, limits).map(RouteResult::Record)
        }
        Some(WireFormat::RunResult) => match read_native_run_result_bounded(bytes, limits)? {
            NativeRunResult::CommandError(error) => Ok(RouteResult::CommandError(error)),
            NativeRunResult::Clause(_) | NativeRunResult::ProtocolAnalysis(_) => {
                Err(NativeRunReadError::Invalid {
                    member: "format",
                    reason: "not a command error of this route",
                })
            }
        },
        _ => Err(NativeRunReadError::UnsupportedFormat(label)),
    }
}
