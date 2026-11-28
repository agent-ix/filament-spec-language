// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-1030: the one declared type of the
//! `native-refinement-transfer-result/1` document (ADR-037).
//!
//! Each transfer's `result` is the `filament.planner-result/v1` document of
//! FSpec FR-492. That format has no declared type in FSL yet, so the value is
//! carried as written and admitted only when its own `format` is the catalog's
//! planner-result label. The `step_for_step` fact is FSpec FR-820's declared
//! type.

use filament_exact::NodeKey;
use fsl_foundation::node_wire::{node_key_from_json, node_key_to_json};
use fsl_foundation::wire_format::WireFormat;
use fsl_replay::planner::StepForStep;
use fsl_replay::spine::{
    read_route_result, FormatMarker, Label, Labelled, NativeRunLimits, NativeRunReadError,
    RouteResult,
};
use ix_trace_rs::implements;
use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;

/// The marker of [`WireFormat::RefinementTransferResult`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RefinementTransferMarker;

impl FormatMarker for RefinementTransferMarker {
    const FORMAT: WireFormat = WireFormat::RefinementTransferResult;
}

/// The `native-refinement-transfer-result/1` document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementTransferDocument {
    /// The document's catalog label.
    pub format: Label<RefinementTransferMarker>,
    /// The refinement and its transfers.
    pub refinement_transfer: RefinementTransferRecord,
}

impl Labelled for RefinementTransferDocument {
    const FORMAT: WireFormat = WireFormat::RefinementTransferResult;
}

fn write_node<S: Serializer>(node: &NodeKey, serializer: S) -> Result<S::Ok, S::Error> {
    node_key_to_json(node)
        .map_err(S::Error::custom)?
        .serialize(serializer)
}

fn read_node<'de, D: Deserializer<'de>>(decoder: D) -> Result<NodeKey, D::Error> {
    let value = serde_json::Value::deserialize(decoder)?;
    node_key_from_json(&value).map_err(D::Error::custom)
}

/// The refinement the transfers follow from.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementFact {
    /// The refinement's name.
    pub name: String,
    /// The refinement declaration node.
    #[serde(serialize_with = "write_node", deserialize_with = "read_node")]
    pub node: NodeKey,
    /// FSpec FR-820's step-for-step fact.
    pub step_for_step: StepForStep,
}

#[derive(Deserialize)]
struct PlannerResultHead {
    format: String,
}

/// A `filament.planner-result/v1` document, carried as written.
#[derive(Clone, Debug)]
pub struct PlannerResultValue(Box<RawValue>);

impl PlannerResultValue {
    /// The planner result `wire` as the value of a transfer's `result`.
    ///
    /// # Errors
    ///
    /// The document cannot be held as a raw value.
    pub fn new(wire: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::value::to_raw_value(wire).map(Self)
    }

    /// The document as written.
    pub fn get(&self) -> &str {
        self.0.get()
    }
}

impl PartialEq for PlannerResultValue {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for PlannerResultValue {}

impl Serialize for PlannerResultValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PlannerResultValue {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let raw = Box::<RawValue>::deserialize(decoder)?;
        let head: PlannerResultHead = serde_json::from_str(raw.get()).map_err(D::Error::custom)?;
        WireFormat::PlannerResult
            .admit(&head.format)
            .map_err(D::Error::custom)?;
        Ok(Self(raw))
    }
}

/// One clause's transfer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transfer {
    /// The clause name.
    pub clause: String,
    /// The settled planner result.
    pub result: PlannerResultValue,
}

/// The refinement and the transfer of each selected clause.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementTransferRecord {
    /// The refinement.
    pub refinement: RefinementFact,
    /// The transfers, in selection order.
    pub transfers: Vec<Transfer>,
}

/// Read the result of a refinement-transfer run: its own document, or the
/// `native-run-result/2` command error of a refused request.
///
/// # Errors
///
/// [`NativeRunReadError`] for another label, a refused bound, or an unknown,
/// missing or mistyped member, naming that member.
#[implements("FR-1030-AC-12")]
pub fn read_refinement_transfer_result(
    bytes: &[u8],
    limits: NativeRunLimits,
) -> Result<RouteResult<RefinementTransferDocument>, NativeRunReadError> {
    read_route_result(bytes, limits)
}
