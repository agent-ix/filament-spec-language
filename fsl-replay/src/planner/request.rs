// SPDX-License-Identifier: AGPL-3.0-or-later
//! `planner::read_settlement_request` (FSL FR-529, FSpec FR-492 "Request" and
//! "Admission"): the strict reader of one `filament.planner-settlement/v1`
//! request against the declaration it settles.
//!
//! The reader refuses at the first defect, in FR-492's order, and settles no
//! slot: (1) the document, its members and their JSON kinds; (2) the format,
//! package, item role and node; (3) the slots against the derived slots;
//! (4) which slots are filled; (5) each premise result against its slot key;
//! (6) each environment record against its declaration member; (7) each
//! discharge entry.

use std::collections::{BTreeMap, BTreeSet};

use filament_exact::NodeKey;
use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use fsl_package::CheckedPackage;
use fsl_semantics::check::wire_id::WireGraph;
use fsl_semantics::check::{ComponentPackages, CompositionSlot, CompositionSlotKey};
use serde_json::{Map, Value};

use super::declaration::{
    DerivedSlot, EnvSupplies, EnvironmentMember, ItemRole, PlannerDeclaration, SlotBacking,
    SlotRole,
};
use super::entry::{read_result_entry, EntryOrigin, LimitFacts, ResultEntry, ResultEntryRefusal};
use super::item::{read_premise_item, PremiseItem};
use super::json::{
    array, check_members, count, object, parse_strict, require, string, Cause, Fault, Parsed,
};
use super::key::{read_node, ItemKey, SlotKey};
use crate::bounds::DEFAULT_REPLAY_INPUT_BYTES;
use crate::proof_result::{child, Certification, TerminalRecord, TerminalValue};

/// The request format.
pub const FORMAT: &str = "filament.planner-settlement/v1";

/// The planner's published default of `planner.max_work` (FSpec FR-461).
pub const DEFAULT_MAX_WORK: u64 = 100_000;

/// The reader's limits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadLimits {
    /// The most encoded bytes a request may hold (`replay.input_bytes`).
    pub input_bytes: u64,
}

impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            input_bytes: DEFAULT_REPLAY_INPUT_BYTES,
        }
    }
}

/// A premise result: the item answered, its result as a terminal record and
/// the record's certification (FSpec FR-492 "Request").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PremiseResult {
    /// The premise request item as read.
    pub(crate) item: PremiseItem,
    /// The result entry as a terminal record.
    pub(crate) record: TerminalRecord,
    /// The record's FR-354 certification when `proved`. An entry read from
    /// request bytes is never `certified`; only [`Self::from_record`] keeps
    /// it.
    pub(crate) certification: Option<Certification>,
    /// The names of the assumptions a `model-claim` result was proved under.
    /// FR-331 carries them in no result member, so a result read from bytes
    /// has none; an in-process producer sets them.
    pub(crate) model_claim_assumptions: BTreeSet<String>,
    /// The entry's FR-272 diagnostic, kept as written.
    pub(crate) diagnostic: Option<Value>,
    /// The limit an `incomplete` entry's `limit-reached` cause names.
    pub(crate) limit_reached: Option<LimitFacts>,
}

impl PremiseResult {
    /// The slot key of the item.
    pub fn key(&self) -> &ItemKey {
        &self.item.key
    }

    /// A premise result built in process from FSL's own terminal record: its
    /// certification, `certified` included, is the record's.
    pub fn from_record(item: PremiseItem, record: TerminalRecord) -> Self {
        let certification = match record.value() {
            TerminalValue::Proved { certification, .. } => Some(*certification),
            _ => None,
        };
        Self {
            item,
            record,
            certification,
            model_claim_assumptions: BTreeSet::new(),
            diagnostic: None,
            limit_reached: None,
        }
    }

    /// A premise result read from request bytes: the entry's certification
    /// is capped, so `certified` is read as `uncertified`.
    pub fn from_entry(item: PremiseItem, entry: ResultEntry) -> Self {
        let entry = entry.read_from_bytes();
        Self {
            item,
            certification: entry.certification,
            model_claim_assumptions: BTreeSet::new(),
            diagnostic: entry.diagnostic,
            limit_reached: entry.limit_reached,
            record: entry.record,
        }
    }
}

/// What an environment record supplies, as the request spells it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecordSupplies {
    /// A scheduling premise.
    Fairness {
        /// The fairness kind.
        kind: FairnessKind,
        /// The granularity.
        granularity: FairnessGranularity,
        /// The operation's context node.
        context: NodeKey,
        /// The operation name.
        operation: String,
    },
    /// An assumption: the `predicate` node.
    Assumption(NodeKey),
}

/// An environment record: an assumption that carries no certification and is
/// never `trusted` (FSpec FR-492 "Request").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentRecord {
    /// The member's name.
    pub(crate) name: String,
    /// The supplier.
    pub(crate) supplier: String,
    /// What it supplies.
    pub(crate) supplies: RecordSupplies,
}

impl EnvironmentRecord {
    /// Whether this record is `member` exactly.
    fn matches(&self, member: &EnvironmentMember) -> bool {
        let supplies = match (&self.supplies, &member.supplies) {
            (
                RecordSupplies::Fairness {
                    kind,
                    granularity,
                    context,
                    operation,
                },
                EnvSupplies::Fairness(premise),
            ) => {
                *kind == premise.constraint.kind()
                    && *granularity == premise.constraint.granularity()
                    && *context == premise.operation.context
                    && *operation == premise.operation.operation
            }
            (RecordSupplies::Assumption(node), EnvSupplies::Assumption(declared)) => {
                node == declared
            }
            _ => false,
        };
        supplies && self.name == member.name && self.supplier == member.supplier
    }
}

/// What fills a slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SlotFill {
    /// Nothing: the slot is unfilled, or settled by the planner.
    Unfilled,
    /// A premise result.
    Result(Box<PremiseResult>),
    /// An environment record.
    Record(EnvironmentRecord),
}

/// One slot of a request, after admission equal to the derived slot of the
/// same index in role and key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestSlot {
    /// The slot's role.
    pub(crate) role: SlotRole,
    /// What fills it.
    pub(crate) fill: SlotFill,
}

/// What discharges a composition slot (FSpec FR-492 `discharges`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Discharger {
    /// A `guarantees` pool entry, by index.
    Guarantee(usize),
    /// A filled `environment` slot, by slot index: a premise supplied only
    /// as an assumption.
    Environment(usize),
}

/// One `discharges` entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Discharge {
    /// The slot index claimed discharged.
    pub(crate) slot: usize,
    /// What is claimed to discharge it.
    pub(crate) by: Discharger,
}

/// The item a request settles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemRef {
    /// `guarantee` or `composition`.
    pub(crate) role: ItemRole,
    /// The declaration node.
    pub(crate) node: NodeKey,
}

/// One admitted `filament.planner-settlement/v1` request. Built only by
/// [`read_settlement_request`]; its slots equal the derived slots in count,
/// order, role and key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementRequest {
    /// The checked package's identity.
    pub(crate) package: String,
    /// The item settled.
    pub(crate) item: ItemRef,
    /// The slots, in derived order.
    pub(crate) slots: Vec<RequestSlot>,
    /// The pool of supplied component guarantee results (composition only).
    pub(crate) guarantees: Vec<PremiseResult>,
    /// The explicit discharge claims (composition only).
    pub(crate) discharges: Vec<Discharge>,
    /// `limits.max_work`, when the request sets it.
    pub(crate) max_work: Option<u64>,
}

/// A request the planner refuses; nothing is settled and no planner result is
/// written.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RequestRefusal {
    /// The request itself is malformed, forged or mismatched
    /// (`invalid_runtime_input`).
    #[error("{0}")]
    Invalid(Fault),
    /// A premise's result entry is refused by FR-331's reader, at the slot
    /// (`invalid_result`).
    #[error("{refusal} under `{pointer}`")]
    Entry {
        /// The pointer of the entry within the request.
        pointer: String,
        /// The entry reader's refusal.
        refusal: ResultEntryRefusal,
    },
}

impl RequestRefusal {
    /// The FR-272 code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(fault) => fault.cause.code(),
            Self::Entry { refusal, .. } => refusal.cause.code(),
        }
    }

    /// The FR-272 cause spelling.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::Invalid(fault) => fault.cause.as_str(),
            Self::Entry { refusal, .. } => refusal.cause.as_str(),
        }
    }

    /// The offending member's RFC 6901 pointer within the request.
    pub fn pointer(&self) -> String {
        match self {
            Self::Invalid(fault) => fault.pointer.clone(),
            Self::Entry { pointer, refusal } => format!("{pointer}{}", refusal.pointer),
        }
    }
}

impl From<Fault> for RequestRefusal {
    fn from(fault: Fault) -> Self {
        Self::Invalid(fault)
    }
}

type Refusal = RequestRefusal;

/// Read and admit one settlement request against `package` (FSL FR-529,
/// FSpec FR-492 "Admission"). The package identity that fixes the relation
/// revision, and the declaration the request settles, are the checked
/// package's own; the request names the declaration by its item node.
///
/// # Errors
///
/// [`RequestRefusal`] naming the first defect in FSpec FR-492's admission
/// order. An item node that names no planner-local guarantee or composition
/// of the package is `invalid-value` at `/item/node`.
#[ix_trace_rs::string_edge]
pub fn read_settlement_request(
    package: &CheckedPackage,
    dependencies: &DependencyGraphs,
    bytes: &[u8],
    limits: &ReadLimits,
) -> Result<SettlementRequest, RequestRefusal> {
    let document = parse_document(bytes, limits)?;
    let Value::Object(top) = &document else {
        return Err(Fault::new(Cause::MalformedJson, "").into());
    };
    let composition = top
        .get("item")
        .and_then(|item| item.get("role"))
        .and_then(Value::as_str)
        .and_then(ItemRole::parse)
        == Some(ItemRole::Composition);
    admit_header(composition, top, package.graph().package_name())?;
    let declaration = declare(package, top)?;
    let scope = held_scope(package, &declaration, dependencies);
    admit_parsed(&declaration, top, scope.as_ref())
}

/// Read and admit one settlement request for `declaration`.
#[cfg(test)]
#[ix_trace_rs::string_edge]
pub(crate) fn read_declared(
    declaration: &PlannerDeclaration,
    bytes: &[u8],
    limits: &ReadLimits,
) -> Result<SettlementRequest, RequestRefusal> {
    let document = parse_document(bytes, limits)?;
    let Value::Object(top) = &document else {
        return Err(Fault::new(Cause::MalformedJson, "").into());
    };
    admit_parsed(declaration, top, None)
}

fn parse_document(bytes: &[u8], limits: &ReadLimits) -> Result<Value, Refusal> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.input_bytes {
        return Err(Fault::new(Cause::InputBytesExceeded, "").into());
    }
    match parse_strict(bytes) {
        Parsed::Document(document) => Ok(document),
        Parsed::Malformed => Err(Fault::new(Cause::MalformedJson, "").into()),
        Parsed::Duplicate(pointer) => Err(Fault::new(Cause::DuplicateMember, pointer).into()),
    }
}

/// Steps 1 and 2 up to the item: members and kinds, format and package.
#[ix_trace_rs::string_edge]
fn admit_header(composition: bool, top: &Map<String, Value>, package: &str) -> Result<(), Fault> {
    let optional: &[&str] = if composition {
        &["discharges", "guarantees", "limits"]
    } else {
        &["limits"]
    };
    check_members(top, "", &["format", "item", "package", "slots"], optional)?;
    check_shapes(top)?;
    if string(require(top, "format", "")?, "/format")? != FORMAT {
        return Err(Fault::invalid("/format".to_owned()));
    }
    if string(require(top, "package", "")?, "/package")? != package {
        return Err(Fault::invalid("/package".to_owned()));
    }
    Ok(())
}

/// The package's declaration the request's item names.
fn declare(
    package: &CheckedPackage,
    top: &Map<String, Value>,
) -> Result<PlannerDeclaration, Fault> {
    let item = object(require(top, "item", "")?, "/item")?;
    let role = string(require(item, "role", "/item")?, "/item/role")?;
    ItemRole::parse(role).ok_or_else(|| Fault::invalid("/item/role".to_owned()))?;
    let node = read_node(require(item, "node", "/item")?, "/item/node")?;
    PlannerDeclaration::from_package(package, &node)
        .map_err(|_| Fault::invalid("/item/node".to_owned()))
}

fn admit_parsed(
    declaration: &PlannerDeclaration,
    top: &Map<String, Value>,
    held: Option<&HeldScope<'_>>,
) -> Result<SettlementRequest, RequestRefusal> {
    // Steps 1 and 2: members, kinds, format, package, item.
    admit_header(
        declaration.role == ItemRole::Composition,
        top,
        &declaration.package,
    )?;
    let item = admit_item(declaration, top)?;
    let max_work = admit_limits(top)?;

    // Step 3: the slots are exactly the derived slots, a held component slot
    // identity read in its structural form first.
    let slots = array(require(top, "slots", "")?, "/slots")?;
    let resolved;
    let slots = match held {
        Some(scope) => {
            resolved = resolve_held_slots(scope, slots)?;
            &resolved
        }
        None => slots,
    };
    admit_slot_keys(declaration, slots)?;

    // Step 4: which slots are filled.
    admit_fill(declaration, slots)?;

    // Step 5: each premise result, then the pool.
    let mut request_slots = Vec::with_capacity(slots.len());
    for (index, (slot, derived)) in slots.iter().zip(&declaration.slots).enumerate() {
        request_slots.push(read_slot_result(index, slot, derived)?);
    }
    let mut guarantees = Vec::new();
    if let Some(pool) = top.get("guarantees") {
        for (index, entry) in array(pool, "/guarantees")?.iter().enumerate() {
            let pointer = format!("/guarantees/{index}");
            let (item, entry_value) = read_premise_parts(entry, &pointer)?;
            read_assumptions(entry, &pointer, false)?;
            guarantees.push(PremiseResult::from_entry(item, entry_value));
        }
    }

    // Step 6: each environment record against its member.
    for (index, (slot, derived)) in slots.iter().zip(&declaration.slots).enumerate() {
        if let Some(record) = slot.get("record") {
            let pointer = format!("/slots/{index}/record");
            let read = read_environment_record(record, &pointer)?;
            let member = match &derived.backing {
                SlotBacking::Environment { member: Some(name) } => declaration
                    .environment
                    .iter()
                    .find(|member| member.name == *name),
                _ => None,
            };
            if !member.is_some_and(|member| read.matches(member)) {
                return Err(Fault::invalid(pointer).into());
            }
            request_slots[index].fill = SlotFill::Record(read);
        }
    }

    // Step 7: each discharge entry.
    let discharges = admit_discharges(declaration, top, &request_slots, &guarantees)?;
    Ok(SettlementRequest {
        package: declaration.package.clone(),
        item,
        slots: request_slots,
        guarantees,
        discharges,
        max_work,
    })
}

/// Step 1's nested kind checks: the slot list, each slot, each result and
/// record, the pool and the discharges are objects and arrays, never a
/// Boolean success flag.
#[ix_trace_rs::string_edge]
fn check_shapes(top: &Map<String, Value>) -> Result<(), Fault> {
    let mut names: Vec<&String> = top.keys().collect();
    names.sort_by(|left, right| crate::proof_result::name_order(left, right));
    for name in names {
        let pointer = child("", name);
        let value = require(top, name, "")?;
        match name.as_str() {
            "format" | "package" => {
                string(value, &pointer)?;
            }
            "item" => check_item_shape(value)?,
            "limits" => {
                let limits = object(value, &pointer)?;
                check_members(limits, &pointer, &["max_work"], &[])?;
                let max_work = require(limits, "max_work", &pointer)?;
                if !(max_work.is_u64() || max_work.is_i64()) {
                    return Err(Fault::wrong_kind(child(&pointer, "max_work")));
                }
            }
            "slots" => {
                for (index, slot) in array(value, &pointer)?.iter().enumerate() {
                    check_slot_shape(slot, &format!("{pointer}/{index}"))?;
                }
            }
            "guarantees" => {
                for (index, entry) in array(value, &pointer)?.iter().enumerate() {
                    check_premise_result_shape(entry, &format!("{pointer}/{index}"))?;
                }
            }
            "discharges" => {
                for (index, entry) in array(value, &pointer)?.iter().enumerate() {
                    check_discharge_shape(entry, &format!("{pointer}/{index}"))?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_item_shape(value: &Value) -> Result<(), Fault> {
    let members = object(value, "/item")?;
    check_members(members, "/item", &["node", "role"], &[])?;
    string(require(members, "role", "/item")?, "/item/role")?;
    object(require(members, "node", "/item")?, "/item/node")?;
    Ok(())
}

fn check_slot_shape(slot: &Value, pointer: &str) -> Result<(), Fault> {
    let members = object(slot, pointer)?;
    check_members(
        members,
        pointer,
        &["identity", "role"],
        &["record", "result"],
    )?;
    string(require(members, "role", pointer)?, &child(pointer, "role"))?;
    object(
        require(members, "identity", pointer)?,
        &child(pointer, "identity"),
    )?;
    if let Some(result) = members.get("result") {
        check_premise_result_shape(result, &child(pointer, "result"))?;
    }
    if let Some(record) = members.get("record") {
        object(record, &child(pointer, "record"))?;
    }
    Ok(())
}

fn check_premise_result_shape(value: &Value, pointer: &str) -> Result<(), Fault> {
    let members = object(value, pointer)?;
    check_members(
        members,
        pointer,
        &["request", "result"],
        &["assumptions", "counterexamples"],
    )?;
    object(
        require(members, "request", pointer)?,
        &child(pointer, "request"),
    )?;
    object(
        require(members, "result", pointer)?,
        &child(pointer, "result"),
    )?;
    if let Some(counterexamples) = members.get("counterexamples") {
        object(counterexamples, &child(pointer, "counterexamples"))?;
    }
    if let Some(assumptions) = members.get("assumptions") {
        let pointer = child(pointer, "assumptions");
        for (index, name) in array(assumptions, &pointer)?.iter().enumerate() {
            string(name, &format!("{pointer}/{index}"))?;
        }
    }
    Ok(())
}

/// The names of the environment assumptions a `model-claim` result was
/// proved under: the premise result's optional `assumptions` member, absent
/// meaning none. Only a `model-claim` slot's result carries it (FSpec FR-492
/// "Slot outcome").
fn read_assumptions(
    value: &Value,
    pointer: &str,
    allowed: bool,
) -> Result<BTreeSet<String>, Fault> {
    let Some(assumptions) = value.get("assumptions") else {
        return Ok(BTreeSet::new());
    };
    let pointer = child(pointer, "assumptions");
    if !allowed {
        return Err(Fault::invalid(pointer));
    }
    let mut names = BTreeSet::new();
    for (index, name) in array(assumptions, &pointer)?.iter().enumerate() {
        let member = format!("{pointer}/{index}");
        let name = string(name, &member)?;
        if name.is_empty() || !names.insert(name.to_owned()) {
            return Err(Fault::invalid(member));
        }
    }
    Ok(names)
}

fn check_discharge_shape(value: &Value, pointer: &str) -> Result<(), Fault> {
    let members = object(value, pointer)?;
    check_members(members, pointer, &["slot"], &["environment", "guarantee"])?;
    if members.contains_key("environment") && members.contains_key("guarantee") {
        return Err(Fault::unknown(child(pointer, "guarantee")));
    }
    if !members.contains_key("environment") && !members.contains_key("guarantee") {
        return Err(Fault::missing(child(pointer, "guarantee")));
    }
    for name in ["environment", "guarantee", "slot"] {
        if let Some(index) = members.get(name) {
            if !(index.is_u64() || index.is_i64()) {
                return Err(Fault::wrong_kind(child(pointer, name)));
            }
        }
    }
    Ok(())
}

fn admit_item(
    declaration: &PlannerDeclaration,
    top: &Map<String, Value>,
) -> Result<ItemRef, Fault> {
    let item = object(require(top, "item", "")?, "/item")?;
    let role = string(require(item, "role", "/item")?, "/item/role")?;
    let role = ItemRole::parse(role).ok_or_else(|| Fault::invalid("/item/role".to_owned()))?;
    if role != declaration.role {
        return Err(Fault::invalid("/item/role".to_owned()));
    }
    let node = read_node(require(item, "node", "/item")?, "/item/node")?;
    if node != declaration.node {
        return Err(Fault::invalid("/item/node".to_owned()));
    }
    if role == ItemRole::Composition && declaration.rule.as_deref() != Some("assume-guarantee") {
        return Err(Fault::invalid("/item/role".to_owned()));
    }
    Ok(ItemRef { role, node })
}

fn admit_limits(top: &Map<String, Value>) -> Result<Option<u64>, Fault> {
    let Some(limits) = top.get("limits") else {
        return Ok(None);
    };
    let members = object(limits, "/limits")?;
    count(require(members, "max_work", "/limits")?, "/limits/max_work").map(Some)
}

fn admit_slot_keys(declaration: &PlannerDeclaration, slots: &[Value]) -> Result<(), Fault> {
    for (index, (slot, derived)) in slots.iter().zip(&declaration.slots).enumerate() {
        let pointer = format!("/slots/{index}");
        let members = object(slot, &pointer)?;
        let role = string(
            require(members, "role", &pointer)?,
            &child(&pointer, "role"),
        )?;
        let identity = require(members, "identity", &pointer)?;
        let derived_identity = derived
            .key
            .to_wire()
            .map_err(|_| Fault::invalid(pointer.clone()))?;
        if role != derived.role.as_str()
            || super::json::canonical_text(identity)
                != super::json::canonical_text(&derived_identity)
        {
            return Err(Fault::invalid(pointer));
        }
    }
    match slots.len().cmp(&declaration.slots.len()) {
        std::cmp::Ordering::Less => Err(Fault::invalid(format!("/slots/{}", slots.len()))),
        std::cmp::Ordering::Greater => Err(Fault::invalid(format!(
            "/slots/{}",
            declaration.slots.len()
        ))),
        std::cmp::Ordering::Equal => Ok(()),
    }
}

fn admit_fill(declaration: &PlannerDeclaration, slots: &[Value]) -> Result<(), Fault> {
    for (index, (slot, derived)) in slots.iter().zip(&declaration.slots).enumerate() {
        let members = object(slot, "")?;
        let (result, record) = (
            members.contains_key("result"),
            members.contains_key("record"),
        );
        let admitted = match &derived.backing {
            SlotBacking::Item => result && !record,
            SlotBacking::Environment { member: Some(_) } => record && !result,
            SlotBacking::Environment { member: None }
            | SlotBacking::PlannerLocal
            | SlotBacking::Unfilled
            | SlotBacking::Pool => !result && !record,
        };
        if !admitted {
            return Err(Fault::invalid(format!("/slots/{index}")));
        }
    }
    Ok(())
}

/// A slot's premise result, read and matched against the slot key.
fn read_slot_result(
    index: usize,
    slot: &Value,
    derived: &DerivedSlot,
) -> Result<RequestSlot, Refusal> {
    let fill = match slot.get("result") {
        None => SlotFill::Unfilled,
        Some(result) => {
            let pointer = format!("/slots/{index}/result");
            let (item, entry) = read_premise_parts(result, &pointer)?;
            if SlotKey::Item(item.key.clone()) != derived.key {
                return Err(Fault::invalid(pointer).into());
            }
            let assumptions =
                read_assumptions(result, &pointer, derived.role == SlotRole::ModelClaim)?;
            let mut premise = PremiseResult::from_entry(item, entry);
            premise.model_claim_assumptions = assumptions;
            SlotFill::Result(Box::new(premise))
        }
    };
    Ok(RequestSlot {
        role: derived.role,
        fill,
    })
}

/// Read a premise result `{request, result}` with FR-331's strict readers.
#[ix_trace_rs::string_edge]
pub(super) fn read_premise_parts(
    value: &Value,
    pointer: &str,
) -> Result<(PremiseItem, ResultEntry), RequestRefusal> {
    let members = object(value, pointer)?;
    let request_pointer = child(pointer, "request");
    let item = read_premise_item(require(members, "request", pointer)?)
        .map_err(|fault| fault.under(&request_pointer))?;
    let entry_pointer = child(pointer, "result");
    let entry_value = require(members, "result", pointer)?;
    // The entry's own certification says who stands behind it (FSpec FR-331
    // "Certification"): `trusted` is a plugin's, which the core checks
    // nothing of. A backend name is a self-declared label and decides
    // nothing.
    let origin = match entry_value.get("certification").and_then(Value::as_str) {
        Some("trusted") => EntryOrigin::Plugin,
        _ => EntryOrigin::FirstParty,
    };
    let entry = read_result_entry(
        entry_value,
        usize::try_from(item.request_index).unwrap_or(0),
        origin,
    )
    .map_err(|refusal| Refusal::Entry {
        pointer: entry_pointer.clone(),
        refusal,
    })?;
    if entry.request_index != item.request_index {
        return Err(Fault::invalid(child(&entry_pointer, "request_index")).into());
    }
    if entry.kind != item.key.kind {
        return Err(Fault::invalid(child(&entry_pointer, "kind")).into());
    }
    let refuted = matches!(entry.record.value(), TerminalValue::Refuted);
    if refuted != members.contains_key("counterexamples") {
        return Err(Fault::invalid(child(pointer, "counterexamples")).into());
    }
    Ok((item, entry))
}

#[ix_trace_rs::string_edge]
fn read_environment_record(value: &Value, pointer: &str) -> Result<EnvironmentRecord, Fault> {
    let members = object(value, pointer)?;
    check_members(members, pointer, &["name", "supplier", "supplies"], &[])?;
    let text = |name: &str| -> Result<String, Fault> {
        let member_pointer = child(pointer, name);
        let text = string(require(members, name, pointer)?, &member_pointer)?;
        if text.is_empty() {
            return Err(Fault::invalid(member_pointer));
        }
        Ok(text.to_owned())
    };
    let name = text("name")?;
    let supplier = text("supplier")?;
    let supplies_pointer = child(pointer, "supplies");
    let supplies = object(require(members, "supplies", pointer)?, &supplies_pointer)?;
    let supplies = if let Some(fairness) = supplies.get("fairness") {
        if supplies.len() != 1 {
            return Err(Fault::unknown(supplies_pointer));
        }
        read_scheduling(fairness, &child(&supplies_pointer, "fairness"))?
    } else if let Some(node) = supplies.get("assumption") {
        if supplies.len() != 1 {
            return Err(Fault::unknown(supplies_pointer));
        }
        RecordSupplies::Assumption(read_node(node, &child(&supplies_pointer, "assumption"))?)
    } else {
        return Err(Fault::missing(child(&supplies_pointer, "fairness")));
    };
    Ok(EnvironmentRecord {
        name,
        supplier,
        supplies,
    })
}

#[ix_trace_rs::string_edge]
fn read_scheduling(value: &Value, pointer: &str) -> Result<RecordSupplies, Fault> {
    let members = object(value, pointer)?;
    check_members(
        members,
        pointer,
        &["context", "granularity", "kind", "operation"],
        &[],
    )?;
    let kind_pointer = child(pointer, "kind");
    let kind = match string(require(members, "kind", pointer)?, &kind_pointer)? {
        "weak" => FairnessKind::Weak,
        "strong" => FairnessKind::Strong,
        _ => return Err(Fault::invalid(kind_pointer)),
    };
    let granularity_pointer = child(pointer, "granularity");
    let granularity = match string(
        require(members, "granularity", pointer)?,
        &granularity_pointer,
    )? {
        "whole" => FairnessGranularity::Whole,
        "each" => FairnessGranularity::Each,
        _ => return Err(Fault::invalid(granularity_pointer)),
    };
    let context = read_node(
        require(members, "context", pointer)?,
        &child(pointer, "context"),
    )?;
    let operation_pointer = child(pointer, "operation");
    let operation = string(require(members, "operation", pointer)?, &operation_pointer)?;
    if operation.is_empty() {
        return Err(Fault::invalid(operation_pointer));
    }
    Ok(RecordSupplies::Fairness {
        kind,
        granularity,
        context,
        operation: operation.to_owned(),
    })
}

/// Step 7: each `discharges` entry names a `rely` or `liveness-dependency`
/// slot, and a pool entry about the slot's subject or a filled environment
/// slot.
fn admit_discharges(
    declaration: &PlannerDeclaration,
    top: &Map<String, Value>,
    slots: &[RequestSlot],
    pool: &[PremiseResult],
) -> Result<Vec<Discharge>, Fault> {
    let Some(entries) = top.get("discharges") else {
        return Ok(Vec::new());
    };
    let mut discharges = Vec::new();
    for (index, entry) in array(entries, "/discharges")?.iter().enumerate() {
        let pointer = format!("/discharges/{index}");
        let members = object(entry, &pointer)?;
        let index_of = |name: &str| -> Result<Option<usize>, Fault> {
            members
                .get(name)
                .map(|value| {
                    let at = count(value, &child(&pointer, name))?;
                    usize::try_from(at).map_err(|_| Fault::invalid(pointer.clone()))
                })
                .transpose()
        };
        let slot = index_of("slot")?.ok_or_else(|| Fault::missing(child(&pointer, "slot")))?;
        let derived = declaration
            .slots
            .get(slot)
            .filter(|derived| matches!(derived.role, SlotRole::Rely | SlotRole::LivenessDependency))
            .ok_or_else(|| Fault::invalid(pointer.clone()))?;
        let by = if let Some(guarantee) = index_of("guarantee")? {
            let entry = pool
                .get(guarantee)
                .ok_or_else(|| Fault::invalid(pointer.clone()))?;
            let subject = entry
                .item
                .key
                .temporal
                .as_ref()
                .map(|temporal| &temporal.subject);
            if derived.premise_subject.as_ref() != subject {
                return Err(Fault::invalid(pointer));
            }
            Discharger::Guarantee(guarantee)
        } else {
            let environment = index_of("environment")?
                .ok_or_else(|| Fault::missing(child(&pointer, "guarantee")))?;
            let filled = slots.get(environment).is_some_and(|candidate| {
                candidate.role == SlotRole::Environment
                    && matches!(candidate.fill, SlotFill::Record(_))
            });
            if !filled {
                return Err(Fault::invalid(pointer));
            }
            Discharger::Environment(environment)
        };
        discharges.push(Discharge { slot, by });
    }
    Ok(discharges)
}

/// The graphs of the admitted packages of a checked package's dependency
/// closure, by package identity: the graphs the node ids of a composition's
/// component slots resolve against (FSpec FR-492 "Inputs", FR-322 "Scope").
pub type DependencyGraphs = BTreeMap<String, WireGraph>;

/// Why the graphs of a package's dependency closure could not be built.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DependencyGraphsRefusal {
    /// A dependency did not emit; the emit limit is
    /// [`fsl_package::EmitRefusal::OutputLimit`].
    #[error("the dependency {package} did not emit: {refusal}")]
    Emission {
        /// The dependency's package identity.
        package: String,
        /// Why it did not emit.
        refusal: fsl_package::EmitRefusal,
    },
    /// A dependency's emission holds no readable `semantic_graph`.
    #[error("the dependency {package} holds no readable graph: {message}")]
    Unreadable {
        /// The dependency's package identity.
        package: String,
        /// The reader's message.
        message: String,
    },
}

/// The graphs of `package`'s dependency closure, each emitted under its own
/// `emit` limit (`emit.output_bytes`), never the request's `input_bytes`.
///
/// # Errors
///
/// [`DependencyGraphsRefusal`] for the first dependency that has no graph.
pub fn dependency_graphs(
    package: &CheckedPackage,
    emit: fsl_package::EmitLimits,
) -> Result<DependencyGraphs, DependencyGraphsRefusal> {
    let mut graphs = DependencyGraphs::new();
    for dependency in package.dependencies().values() {
        let name = dependency.graph().package_name().to_owned();
        let emission = fsl_package::emit_checked(dependency, emit).map_err(|refusal| {
            DependencyGraphsRefusal::Emission {
                package: name.clone(),
                refusal,
            }
        })?;
        let graph = WireGraph::of_package_bytes(emission.package().bytes()).map_err(|message| {
            DependencyGraphsRefusal::Unreadable {
                package: name.clone(),
                message,
            }
        })?;
        graphs.insert(name, graph);
    }
    Ok(graphs)
}

/// What a request's held component slot identities resolve against.
struct HeldScope<'a> {
    packages: ComponentPackages,
    slots: Vec<CompositionSlot>,
    graphs: &'a DependencyGraphs,
}

/// The scope of the composition `declaration` names in `package`, when it
/// names one.
fn held_scope<'a>(
    package: &CheckedPackage,
    declaration: &PlannerDeclaration,
    graphs: &'a DependencyGraphs,
) -> Option<HeldScope<'a>> {
    let composition = package
        .graph()
        .compositions()
        .iter()
        .find(|composition| composition.node() == declaration.node())?;
    Some(HeldScope {
        packages: composition.component_packages(),
        slots: composition.item().premises,
        graphs,
    })
}

/// FSpec FR-322 "Scope", FR-492 "Request": the `identity` of a
/// `component-guarantee` slot holds its node ids as index segments of the
/// graph of the package of the slot's component, a dependency of the checked
/// package; a peer clause id of its `assumptions` holds them as index
/// segments of the graph of the package of the component of the peer
/// guarantee the declaration's assumption set names at that position. Write
/// each such identity in its structural form, so that step 3 compares the form
/// the declaration derives.
///
/// # Errors
///
/// `invalid-value` at `/slots/{i}` (as step 3 refuses a slot that is not the
/// derived one) for an identity of another package than its component's,
/// whose component package has no graph among `graphs`, or whose ids do not
/// resolve in it.
fn resolve_held_slots(scope: &HeldScope<'_>, slots: &[Value]) -> Result<Vec<Value>, Fault> {
    use fsl_semantics::check::wire_id::{
        check_component_key_package, component_key_structural, key_holds_index_segment,
    };
    let mut resolved = slots.to_vec();
    for (at, slot) in resolved.iter_mut().enumerate() {
        let pointer = format!("/slots/{at}");
        let Some(derived) = scope.slots.get(at) else {
            continue;
        };
        let Some(identity) = slot
            .as_object_mut()
            .filter(|slot| {
                slot.get("role").and_then(Value::as_str)
                    == Some(SlotRole::ComponentGuarantee.as_str())
            })
            .and_then(|slot| slot.get_mut("identity"))
        else {
            continue;
        };
        let package = derived
            .guarantee
            .as_ref()
            .and_then(|clause| scope.packages.of_component(&clause.component));
        check_component_key_package(package, identity)
            .map_err(|_| Fault::invalid(pointer.clone()))?;
        let Some(package) = package else {
            return Err(Fault::invalid(pointer));
        };
        if !key_holds_index_segment(identity) {
            continue;
        }
        let graph = scope
            .graphs
            .get(package)
            .ok_or_else(|| Fault::invalid(pointer.clone()))?;
        let assumed: &[NodeKey] = match &derived.key {
            Some(CompositionSlotKey::ComponentGuarantee(guarantee)) => &guarantee.assumptions,
            _ => &[],
        };
        let peers = assumed
            .iter()
            .map(|node| {
                scope
                    .packages
                    .of_clause(node)
                    .and_then(|p| scope.graphs.get(p))
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| Fault::invalid(pointer.clone()))?;
        *identity = component_key_structural(graph, Some(package), identity, &peers)
            .map_err(|_| Fault::invalid(pointer))?;
    }
    Ok(resolved)
}

#[cfg(test)]
mod held_slot_tests {
    //! FSpec FR-322 "Scope", FR-492 "Admission": a held `component-guarantee`
    //! slot identity is read in its structural form at step 3, after the
    //! step 1 shape checks.

    use filament_exact::{Origin, Role};
    use fsl_foundation::node_wire::node_key_from_json;
    use fsl_foundation::source::provenance::OccurrenceKey;
    use fsl_semantics::check::{
        ClauseRef, CompositionSlot, SlotBacking as SemBacking, SlotRole as SemRole,
    };
    use ix_trace_rs::trace;
    use serde_json::{json, Value};

    use super::super::key::{ClaimKind, ExtentClass, TemporalItemKind, TemporalKey};
    use super::super::tests::{item_json, proved_entry, slot as derived_slot, PACKAGE};
    use super::*;

    const PRODUCER: &str = "producer@1";
    const CONSUMER: &str = "consumer@1";

    /// `label` of `package`, wrapped: an id with one operand segment.
    fn wrapped(package: &str, label: &str) -> Value {
        json!({"package": package, "ordinal": 0, "path": [
            "wrap", {"package": package, "path": [label], "ordinal": 0}]})
    }

    /// The graph of `package`: `label`, then its wrapper holding `{"node": 0}`.
    fn graph(package: &str, label: &str) -> WireGraph {
        WireGraph::of_package_value(&json!({"semantic_graph": {"nodes": [
            {"node_id": {"package": package, "path": [label], "ordinal": 0}},
            {"node_id": {"package": package, "path": ["wrap", {"node": 0}], "ordinal": 0}}
        ]}}))
        .expect("a graph")
    }

    fn key(package: &str, label: &str) -> ItemKey {
        let node = node_key_from_json(&wrapped(package, label)).expect("a node key");
        ItemKey {
            claim: OccurrenceKey::new(node, Origin::new(Role::new("claim"), 0)),
            kind: ClaimKind::TemporalSatisfaction,
            temporal: Some(TemporalKey {
                subject: super::super::json::canonical_text(
                    &json!({"identity": format!("{package} using Model")}),
                ),
                item_kind: TemporalItemKind::Formula,
                fairness: Vec::new(),
                stutter_closure: true,
            }),
            correspondence: None,
            premises: Vec::new(),
            extent: ExtentClass::Unbounded,
        }
    }

    fn declaration() -> PlannerDeclaration {
        let slot = |package, label, start| {
            derived_slot(
                SlotRole::ComponentGuarantee,
                SlotKey::Item(key(package, label)),
                SlotBacking::Item,
                start,
            )
        };
        PlannerDeclaration {
            role: ItemRole::Composition,
            node: super::super::tests::node("S"),
            package: PACKAGE.to_owned(),
            rule: Some("assume-guarantee".to_owned()),
            clause: None,
            relation_sources: Vec::new(),
            relation_node: None,
            environment: Vec::new(),
            mismatch: None,
            slots: vec![slot(PRODUCER, "Sends", 0), slot(CONSUMER, "Done", 10)],
            supplies: Vec::new(),
        }
    }

    fn clause_slot(index: usize, component: &str, package: &str, label: &str) -> CompositionSlot {
        CompositionSlot {
            index,
            role: SemRole::ComponentGuarantee,
            backing: SemBacking::Item,
            key: None,
            guarantee: Some(ClauseRef {
                component: component.to_owned(),
                name: label.to_owned(),
                node: super::super::tests::node_in(package, label),
            }),
            on: None,
            constraint: None,
            environment: None,
            source: fsl_foundation::Span { start: 0, end: 0 },
        }
    }

    fn scope(graphs: &DependencyGraphs) -> HeldScope<'_> {
        HeldScope {
            packages: ComponentPackages::new(
                BTreeMap::from([
                    ("Producer".to_owned(), PRODUCER.to_owned()),
                    ("Consumer".to_owned(), CONSUMER.to_owned()),
                ]),
                BTreeMap::new(),
            ),
            slots: vec![
                clause_slot(0, "Producer", PRODUCER, "Sends"),
                clause_slot(1, "Consumer", CONSUMER, "Done"),
            ],
            graphs,
        }
    }

    fn graphs() -> DependencyGraphs {
        BTreeMap::from([
            (PRODUCER.to_owned(), graph(PRODUCER, "Sends")),
            (CONSUMER.to_owned(), graph(CONSUMER, "Done")),
        ])
    }

    /// The request for `declaration`, each slot's identity held in its
    /// component's graph and its result proved.
    fn held_request(declaration: &PlannerDeclaration, graphs: &DependencyGraphs) -> Value {
        let slots: Vec<Value> = declaration
            .slots
            .iter()
            .enumerate()
            .map(|(at, derived)| {
                let SlotKey::Item(key) = &derived.key else {
                    panic!("an item slot");
                };
                let identity = derived.key.to_wire().expect("an identity");
                let package = if at == 0 { PRODUCER } else { CONSUMER };
                json!({
                    "role": derived.role.as_str(),
                    "identity": graphs[package].document_value(&identity).expect("held"),
                    "result": {"request": item_json(key, at), "result": proved_entry(key, at)},
                })
            })
            .collect();
        json!({
            "format": FORMAT,
            "package": PACKAGE,
            "item": {"role": "composition",
                "node": super::super::tests::node_key_json(&declaration.node)},
            "slots": slots,
        })
    }

    fn read_held(
        declaration: &PlannerDeclaration,
        graphs: &DependencyGraphs,
        request: &Value,
    ) -> Result<SettlementRequest, RequestRefusal> {
        let bytes = serde_json::to_vec(request).expect("bytes");
        let document = parse_document(&bytes, &ReadLimits::default())?;
        let Value::Object(top) = &document else {
            panic!("an object");
        };
        admit_parsed(declaration, top, Some(&scope(graphs)))
    }

    fn refusal(
        declaration: &PlannerDeclaration,
        graphs: &DependencyGraphs,
        request: &Value,
    ) -> (&'static str, &'static str, String) {
        let refusal = read_held(declaration, graphs, request).expect_err("refused");
        (refusal.code(), refusal.cause(), refusal.pointer())
    }

    fn invalid(pointer: &str) -> (&'static str, &'static str, String) {
        ("invalid_runtime_input", "invalid-value", pointer.to_owned())
    }

    /// FR-492 "Request": held component slot identities, written in the
    /// graphs of their components' packages, are admitted as the derived
    /// slots.
    #[trace("FSpec-TC-233", "FSpec-FR-322-AC-520")]
    #[test]
    fn a_held_component_slot_identity_round_trips() {
        let (declaration, graphs) = (declaration(), graphs());
        let request = held_request(&declaration, &graphs);
        assert_eq!(
            request["slots"][0]["identity"]["claim"]["node"]["path"][1],
            json!({"node": 0})
        );
        read_held(&declaration, &graphs, &request).expect("admitted");
    }

    /// NI-27: an index the component's graph does not hold refuses at the
    /// slot.
    #[trace("FSpec-TC-233", "FSpec-FR-322-AC-520")]
    #[test]
    fn a_held_slot_identity_index_out_of_range_refuses_at_the_slot() {
        let (declaration, graphs) = (declaration(), graphs());
        let mut request = held_request(&declaration, &graphs);
        request["slots"][1]["identity"]["claim"]["node"]["path"][1] = json!({"node": 99});
        assert_eq!(
            refusal(&declaration, &graphs, &request),
            invalid("/slots/1")
        );
    }

    /// NI-28: an identity of another package refuses at the slot.
    #[trace("FSpec-TC-233", "FSpec-FR-322-AC-520")]
    #[test]
    fn a_held_slot_identity_of_a_foreign_package_refuses_at_the_slot() {
        let (declaration, graphs) = (declaration(), graphs());
        let mut request = held_request(&declaration, &graphs);
        request["slots"][0]["identity"]["temporal"]["subject"]["identity"] =
            json!("other@1 using Model");
        assert_eq!(
            refusal(&declaration, &graphs, &request),
            invalid("/slots/0")
        );
    }

    /// FR-492 "Admission": step 1's shape checks precede the identity
    /// resolution, so a foreign identity in slot 0 does not mask a later
    /// slot's wrong value kind.
    #[trace("FSpec-TC-233", "FSpec-FR-322-AC-520")]
    #[test]
    fn a_foreign_identity_does_not_mask_a_later_wrong_value_kind() {
        let (declaration, graphs) = (declaration(), graphs());
        let mut request = held_request(&declaration, &graphs);
        request["slots"][0]["identity"]["temporal"]["subject"]["identity"] =
            json!("other@1 using Model");
        request["slots"][1]["result"] = json!(true);
        assert_eq!(
            refusal(&declaration, &graphs, &request),
            (
                "invalid_runtime_input",
                "wrong-value-kind",
                "/slots/1/result".to_owned()
            )
        );
    }

    /// FR-492 "Admission" 3: a foreign identity in an earlier slot is the
    /// first defect, ahead of a later slot's wrong identity.
    #[trace("FSpec-TC-233", "FSpec-FR-322-AC-520")]
    #[test]
    fn the_first_differing_slot_is_named() {
        let (declaration, graphs) = (declaration(), graphs());
        let mut request = held_request(&declaration, &graphs);
        request["slots"][0]["identity"]["temporal"]["subject"]["identity"] =
            json!("other@1 using Model");
        request["slots"][1]["identity"]["claim"]["node"]["path"][1] = json!({"node": 99});
        assert_eq!(
            refusal(&declaration, &graphs, &request),
            invalid("/slots/0")
        );
    }
}
