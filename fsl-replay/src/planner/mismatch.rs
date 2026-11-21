// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-492 "Interface mismatch": the refutation of a composition by a
//! declared interface that is absent or weaker than a premise needs.
//!
//! The mismatch is decided from the checked declaration alone, never from a
//! result a request supplies, so the planner settles it and an analyze run
//! recomputes it to admit or refuse a provider's `interface-mismatch`
//! counterexample (FSpec FR-331). One order decides both, reason-major: the
//! first `absent` mismatch over the `rely` and `liveness-dependency` slots in
//! ascending slot order, and only when there is none the first `weaker` one
//! over the same slots.

use filament_exact::NodeKey;
use fsl_foundation::node_wire::node_key_to_json;
use fsl_semantics::check::{CheckedComposition, ClauseRef, CompositionSlot, SlotRole};
use serde_json::{json, Value};

use super::declaration::strength_implies;
use super::key::KeyWireError;

/// Why a declared interface does not meet a premise (FSpec FR-492 "Interface
/// mismatch").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MismatchReason {
    /// The declaration has no peer guarantee at the clause the premise names.
    Absent,
    /// The peer's declared guarantee is conditional on a fairness constraint
    /// the composition supplies nothing that implies.
    Weaker,
}

impl MismatchReason {
    /// The `reason` spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Weaker => "weaker",
        }
    }
}

/// A declared interface mismatch: FSpec FR-331's `interface-mismatch`
/// counterexample, and the planner result's `mismatch` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterfaceMismatch {
    /// The peer component `D`.
    pub component: String,
    /// The node of the peer guarantee's clause `H`.
    pub interface: NodeKey,
    /// The index, in the item's `premises`, of the slot that needs the peer.
    pub premise: usize,
    /// Why the interface does not meet it.
    pub reason: MismatchReason,
}

impl InterfaceMismatch {
    /// The mismatch's members, as the planner result's `mismatch` member
    /// spells them (FSpec FR-492 "Planner result").
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when the interface node cannot be written.
    pub fn to_members(&self) -> Result<Value, KeyWireError> {
        Ok(json!({
            "component": self.component,
            "interface": node_key_to_json(&self.interface)?,
            "premise": self.premise,
            "reason": self.reason.as_str(),
        }))
    }

    /// The mismatch as FSpec FR-331 spells the `interface-mismatch`
    /// counterexample.
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when the interface node cannot be written.
    pub fn to_wire(&self) -> Result<Value, KeyWireError> {
        let mut wire = self.to_members()?;
        if let Some(members) = wire.as_object_mut() {
            members.insert("type".to_owned(), json!("interface-mismatch"));
        }
        Ok(wire)
    }
}

/// The first interface mismatch `composition` declares, in FSpec FR-492's
/// reason-major order; `None` when no premise names an absent or weaker
/// interface.
pub fn interface_mismatch(composition: &CheckedComposition) -> Option<InterfaceMismatch> {
    let slots = composition.slots();
    let needs: Vec<(&CompositionSlot, &ClauseRef)> = slots
        .iter()
        .filter(|slot| matches!(slot.role, SlotRole::Rely | SlotRole::LivenessDependency))
        .filter_map(|slot| Some((slot, slot.on.as_ref()?)))
        .collect();
    let declared = |peer: &ClauseRef| {
        composition
            .guarantees()
            .iter()
            .any(|g| g.reference == *peer)
    };
    let mismatch = |slot: &CompositionSlot, peer: &ClauseRef, reason| InterfaceMismatch {
        component: peer.component.clone(),
        interface: peer.node.clone(),
        premise: slot.index,
        reason,
    };
    needs
        .iter()
        .find(|(_, peer)| !declared(peer))
        .map(|(slot, peer)| mismatch(slot, peer, MismatchReason::Absent))
        .or_else(|| {
            needs
                .iter()
                .find(|(_, peer)| !fairness_supplied(composition, peer))
                .map(|(slot, peer)| mismatch(slot, peer, MismatchReason::Weaker))
        })
}

/// Whether every fairness constraint of the peer clause `peer` is implied by a
/// constraint the composition supplies (FSpec FR-380 "Fairness implication").
fn fairness_supplied(composition: &CheckedComposition, peer: &ClauseRef) -> bool {
    let Some(facts) = composition
        .clauses()
        .iter()
        .find(|facts| facts.reference == *peer)
    else {
        return true;
    };
    facts.fairness.iter().all(|needed| {
        composition.supplies().iter().any(|supplied| {
            supplied.context == needed.context
                && supplied.operation == needed.operation
                && strength_implies(
                    (supplied.kind, supplied.granularity),
                    (needed.constraint.kind(), needed.constraint.granularity()),
                )
        })
    })
}
