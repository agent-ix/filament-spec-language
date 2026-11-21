// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical, caller-metered source-policy states for step-counting plans.

use std::cell::RefCell;

use filament_exact::{
    CancelCause, Charge, ChargePoint, Incomplete, Meter, NodeKey, NodePathIter, Origin,
};
use fsl_foundation::node_wire::{project_node_key, NodeIdentitySink};
use fsl_foundation::IdentityLimits;
use fsl_semantics::check::{Activation, CheckedTemporalClause, FormulaNodeId, TemporalProfile};
use quire_canonical::{Error, Limits, Node, NodeRef, Sink, Writer};

use crate::policy_budget::{Budget, Output, Provision};

use super::{
    closure_shape, invariant_atom, BoundedKind, BoundedState, SafetyState, AUTOMATON_STATE_KEY_V1,
};

/// A supported positive-length policy's checked control shape.
#[derive(Clone, Copy, Debug)]
pub struct StepPolicy {
    atom: FormulaNodeId,
    kind: Kind,
}
#[derive(Clone, Copy, Debug)]
enum Kind {
    Safety,
    Bounded { kind: BoundedKind, upper: u64 },
}

/// One instance's state, independently reconstructible from canonical bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyState {
    /// The shared safety translation state.
    Safety(SafetyState),
    /// The shared bounded translation state.
    Bounded(BoundedState),
}

/// Borrowed checked identity and actual binding for a policy instance.
#[derive(Clone, Copy)]
pub struct PolicyContext<'a> {
    /// Authored clause node.
    pub node: &'a NodeKey,
    /// The clause's claim occurrence.
    pub claim: &'a Origin,
    /// Checked over parameter.
    pub parameter: &'a NodeKey,
    /// The actual instance key.
    pub instance: &'a str,
}

/// Stable category of a policy representation refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyStateCode {
    /// Original scalar accounting denial.
    Accounting,
    /// Original cancellation.
    Cancelled,
    /// Malformed, unreachable or foreign state.
    Invalid,
    /// Existing model-instance refusal.
    Instances,
    /// Existing identity byte ceiling.
    IdentityLimit,
    /// Actual allocation or encoding error.
    Encoding,
}

/// A policy representation operation stopped before admitting a result.
#[derive(Debug, thiserror::Error)]
pub enum PolicyStateError {
    /// Original scalar denial, with its named point and amount.
    #[error("policy accounting stopped: {0:?}")]
    Accounting(Incomplete),
    /// Original cancellation cause.
    #[error("policy operation cancelled: {0:?}")]
    Cancelled(CancelCause),
    /// Malformed, noncanonical, unreachable or foreign policy state.
    #[error("invalid canonical policy state")]
    Invalid,
    /// Existing model-instance admission refusal.
    #[error(transparent)]
    Instances(#[from] crate::simulation::NotSimulated),
    /// Existing per-identity encoding byte ceiling.
    #[error("policy identity byte limit {value}")]
    IdentityLimit {
        /// The existing bound in force.
        value: u64,
    },
    /// Actual allocation/encoding failure.
    #[error("policy encoding: {0}")]
    Encoding(#[from] Error),
}

impl PolicyStateError {
    /// The stable category, independent of diagnostic wording.
    pub const fn code(&self) -> PolicyStateCode {
        match self {
            Self::Accounting(_) => PolicyStateCode::Accounting,
            Self::Cancelled(_) => PolicyStateCode::Cancelled,
            Self::Invalid => PolicyStateCode::Invalid,
            Self::Instances(_) => PolicyStateCode::Instances,
            Self::IdentityLimit { .. } => PolicyStateCode::IdentityLimit,
            Self::Encoding(_) => PolicyStateCode::Encoding,
        }
    }
}

struct Identity<'a, 'w, 's, 'm, S: Sink + ?Sized> {
    writer: &'w mut Writer<'s, S>,
    budget: &'a Budget<'m>,
}
impl<S: Sink + ?Sized> NodeIdentitySink for Identity<'_, '_, '_, '_, S> {
    type Error = Error;
    fn visit(&mut self) -> Result<(), Error> {
        self.budget.event(1)
    }
    fn begin_object(&mut self) -> Result<(), Error> {
        self.writer.begin_object()
    }
    fn name(&mut self, name: &'static str) -> Result<(), Error> {
        self.budget.event(name.len())?;
        self.writer.name(name)
    }
    fn ordinal(&mut self, ordinal: u64) -> Result<(), Error> {
        self.budget.event(1)?;
        self.writer.integer(i128::from(ordinal))
    }
    fn text(&mut self, text: &str) -> Result<(), Error> {
        self.budget.event(text.len())?;
        self.writer.string(text)
    }
    fn begin_array(&mut self) -> Result<(), Error> {
        self.writer.begin_array()
    }
    fn end_array(&mut self) -> Result<(), Error> {
        self.writer.end_array()
    }
    fn end_object(&mut self) -> Result<(), Error> {
        self.writer.end_object()
    }
    fn push_path<'a>(
        &mut self,
        stack: &mut Vec<NodePathIter<'a>>,
        path: NodePathIter<'a>,
    ) -> Result<(), Error> {
        self.budget
            .growth(stack, 1)
            .map_err(|error| self.budget.bridge(error))?;
        stack.push(path);
        Ok(())
    }
}
fn text<S: Sink + ?Sized>(
    writer: &mut Writer<'_, S>,
    budget: &Budget<'_>,
    value: &str,
) -> Result<(), Error> {
    budget.event(value.len())?;
    writer.string(value)
}
fn decimal<S: Sink + ?Sized>(
    writer: &mut Writer<'_, S>,
    budget: &Budget<'_>,
    mut value: u64,
) -> Result<(), Error> {
    let mut buffer = [0_u8; 20];
    let mut start = buffer.len();
    loop {
        start -= 1;
        buffer[start] = b'0'
            + u8::try_from(value % 10).map_err(|_| Error::Serialize("decimal digit".into()))?;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    let digits = std::str::from_utf8(&buffer[start..])
        .map_err(|_| Error::Serialize("decimal text".into()))?;
    text(writer, budget, digits)
}
fn empty<S: Sink + ?Sized>(writer: &mut Writer<'_, S>) -> Result<(), Error> {
    writer.begin_array()?;
    writer.end_array()
}
fn state<S: Sink + ?Sized>(
    writer: &mut Writer<'_, S>,
    budget: &Budget<'_>,
    value: PolicyState,
    kind: Kind,
) -> Result<(), Error> {
    budget.event(1)?;
    writer.begin_array()?;
    match value {
        PolicyState::Safety(safety) => {
            text(writer, budget, "safety")?;
            writer.begin_array()?;
            if safety == SafetyState::Carried {
                writer.begin_array()?;
                writer.begin_array()?;
                writer.begin_array()?;
                empty(writer)?;
                text(writer, budget, "positive")?;
                writer.end_array()?;
                writer.end_array()?;
                empty(writer)?;
                empty(writer)?;
                empty(writer)?;
                writer.end_array()?;
            }
            writer.end_array()?;
        }
        PolicyState::Bounded(BoundedState::Accept) => text(writer, budget, "accept")?,
        PolicyState::Bounded(BoundedState::Reject) => text(writer, budget, "reject")?,
        PolicyState::Bounded(BoundedState::Open { remaining }) => {
            text(writer, budget, "bounded")?;
            writer.begin_array()?;
            text(writer, budget, "origin")?;
            decimal(writer, budget, remaining)?;
            writer.end_array()?;
            writer.begin_array()?;
            writer.begin_array()?;
            empty(writer)?;
            empty(writer)?;
            writer.begin_array()?;
            writer.begin_array()?;
            empty(writer)?;
            text(writer, budget, "positive")?;
            let window = match kind {
                Kind::Bounded { kind, .. } => kind.window(),
                Kind::Safety => return Err(Error::Serialize("bounded policy kind".into())),
            };
            text(writer, budget, window)?;
            decimal(writer, budget, remaining)?;
            writer.end_array()?;
            writer.end_array()?;
            empty(writer)?;
            writer.end_array()?;
            writer.end_array()?;
        }
        PolicyState::Bounded(BoundedState::Idle) => {
            return Err(Error::Serialize("origin policy is not idle".into()))
        }
    }
    writer.end_array()
}
fn key(
    context: &PolicyContext<'_>,
    value: PolicyState,
    kind: Kind,
    budget: &Budget<'_>,
) -> Result<Vec<u8>, PolicyStateError> {
    let mut output = Output {
        budget,
        bytes: Vec::new(),
    };
    let mut provision = Provision(budget);
    let mut writer = Writer::with_reservation(
        &mut output,
        Limits::new(IdentityLimits::default().input_bytes),
        &mut provision,
    );
    let encoded = (|| {
        budget.event(1)?;
        writer.begin_array()?;
        text(&mut writer, budget, AUTOMATON_STATE_KEY_V1)?;
        writer.begin_array()?;
        writer.begin_array()?;
        project_node_key(
            context.node,
            &mut Identity {
                writer: &mut writer,
                budget,
            },
        )?;
        text(&mut writer, budget, context.claim.role().as_str())?;
        decimal(&mut writer, budget, context.claim.ordinal())?;
        writer.end_array()?;
        text(
            &mut writer,
            budget,
            TemporalProfile::InfiniteTrace.identity(),
        )?;
        text(&mut writer, budget, "infinite")?;
        writer.begin_array()?;
        writer.begin_array()?;
        project_node_key(
            context.parameter,
            &mut Identity {
                writer: &mut writer,
                budget,
            },
        )?;
        writer.begin_array()?;
        text(&mut writer, budget, "instance")?;
        text(&mut writer, budget, context.instance)?;
        writer.end_array()?;
        writer.end_array()?;
        writer.end_array()?;
        writer.end_array()?;
        state(&mut writer, budget, value, kind)?;
        writer.end_array()?;
        writer.finish().map(|_| ())
    })();
    if let Some(error) = budget.stopped.borrow_mut().take() {
        return Err(error);
    }
    encoded.map_err(|error| match error {
        Error::Limit(limit) => PolicyStateError::IdentityLimit { value: limit.bound },
        other => PolicyStateError::Encoding(other),
    })?;
    Ok(output.bytes)
}

impl StepPolicy {
    /// Select only the canonical, capture-free origin step-counting shapes.
    pub fn new(clause: &CheckedTemporalClause) -> Option<Self> {
        if clause.profile() != TemporalProfile::InfiniteTrace
            || clause.activation() != Activation::Origin
            || clause.captures().next().is_some()
            || !clause.fairness().is_empty()
        {
            return None;
        }
        if let Some(atom) = invariant_atom(clause) {
            return Some(Self {
                atom,
                kind: Kind::Safety,
            });
        }
        let (kind, upper, atom) = closure_shape(clause)?;
        Some(Self {
            atom,
            kind: Kind::Bounded { kind, upper },
        })
    }
    /// The actual checked holds atom read by this monitor.
    pub const fn atom(&self) -> FormulaNodeId {
        self.atom
    }
    /// Whether the next position still needs the atom's actual value.
    pub fn reads(&self, before: Option<PolicyState>) -> bool {
        !matches!(
            before,
            Some(PolicyState::Bounded(
                BoundedState::Accept | BoundedState::Reject | BoundedState::Open { remaining: 0 }
            ))
        )
    }
    /// Advance the shared safety or bounded control state by one real position.
    pub fn advance(
        &self,
        before: Option<PolicyState>,
        holds: bool,
        meter: &mut Meter,
    ) -> Result<PolicyState, PolicyStateError> {
        Budget {
            meter: RefCell::new(meter),
            stopped: RefCell::new(None),
        }
        .pay(Charge::new(ChargePoint::TemporalVisit))?;
        Ok(match (self.kind, before) {
            (Kind::Safety, None) => PolicyState::Safety(SafetyState::read(None, holds)),
            (Kind::Safety, Some(PolicyState::Safety(before))) => {
                PolicyState::Safety(SafetyState::read(Some(before), holds))
            }
            (Kind::Bounded { kind, upper }, None) => {
                PolicyState::Bounded(kind.decide(upper, holds))
            }
            (Kind::Bounded { kind, .. }, Some(PolicyState::Bounded(before))) => {
                PolicyState::Bounded(kind.step(before, true, holds))
            }
            _ => return Err(PolicyStateError::Invalid),
        })
    }
    /// A safety/deadline rejection excludes every continuation of this prefix.
    pub fn rejects(&self, value: PolicyState) -> bool {
        matches!(
            value,
            PolicyState::Safety(SafetyState::Rejected) | PolicyState::Bounded(BoundedState::Reject)
        ) || matches!(
            (self.kind, value),
            (
                Kind::Bounded {
                    kind: BoundedKind::Eventually,
                    ..
                },
                PolicyState::Bounded(BoundedState::Open { remaining: 0 })
            )
        )
    }
    /// Decide the actual final letter repeated forever, without false extension.
    pub fn terminal(
        &self,
        value: PolicyState,
        final_holds: bool,
        meter: &mut Meter,
    ) -> Result<bool, PolicyStateError> {
        Budget {
            meter: RefCell::new(meter),
            stopped: RefCell::new(None),
        }
        .pay(Charge::new(ChargePoint::TemporalVisit))?;
        Ok(match value {
            PolicyState::Safety(SafetyState::Carried)
            | PolicyState::Bounded(BoundedState::Accept) => true,
            PolicyState::Safety(SafetyState::Rejected)
            | PolicyState::Bounded(BoundedState::Reject | BoundedState::Idle) => false,
            PolicyState::Bounded(BoundedState::Open { remaining: 0 }) => matches!(
                self.kind,
                Kind::Bounded {
                    kind: BoundedKind::Always,
                    ..
                }
            ),
            PolicyState::Bounded(BoundedState::Open { .. }) => final_holds,
        })
    }
    /// Copy retained policy vectors on the original representation ledger.
    pub fn copy_states(
        monitors: &[Vec<u8>],
        meter: &mut Meter,
    ) -> Result<Vec<Vec<u8>>, PolicyStateError> {
        let budget = Budget {
            meter: RefCell::new(meter),
            stopped: RefCell::new(None),
        };
        let mut output = Vec::new();
        budget.growth(&mut output, monitors.len())?;
        for monitor in monitors {
            budget.work(1)?;
            let mut copy = Vec::new();
            budget.copy(&mut copy, monitor)?;
            output.push(copy);
        }
        Ok(output)
    }

    /// Encode the ordered vector of full canonical per-instance keys.
    pub fn encode(
        &self,
        contexts: &[PolicyContext<'_>],
        states: &[PolicyState],
        meter: &mut Meter,
    ) -> Result<Vec<u8>, PolicyStateError> {
        if contexts.len() != states.len() {
            return Err(PolicyStateError::Invalid);
        }
        let budget = Budget {
            meter: RefCell::new(meter),
            stopped: RefCell::new(None),
        };
        let mut output = Vec::new();
        budget.copy(
            &mut output,
            &u64::try_from(states.len())
                .map_err(|_| PolicyStateError::Invalid)?
                .to_le_bytes(),
        )?;
        for (context, value) in contexts.iter().zip(states) {
            let encoded = key(context, *value, self.kind, &budget)?;
            budget.copy(
                &mut output,
                &u64::try_from(encoded.len())
                    .map_err(|_| PolicyStateError::Invalid)?
                    .to_le_bytes(),
            )?;
            budget.copy(&mut output, &encoded)?;
        }
        Ok(output)
    }
    /// Strictly admit vector framing and every context/state; no local IDs exist.
    pub fn admit(
        &self,
        contexts: &[PolicyContext<'_>],
        bytes: &[u8],
        meter: &mut Meter,
    ) -> Result<Vec<PolicyState>, PolicyStateError> {
        let budget = Budget {
            meter: RefCell::new(meter),
            stopped: RefCell::new(None),
        };
        let mut input = bytes;
        if take_count(&mut input, &budget)? != contexts.len() {
            return Err(PolicyStateError::Invalid);
        }
        let mut states = Vec::new();
        for context in contexts {
            let count = take_count(&mut input, &budget)?;
            let octets = input.get(..count).ok_or(PolicyStateError::Invalid)?;
            input = input.get(count..).ok_or(PolicyStateError::Invalid)?;
            budget.work(octets.len())?;
            let document = quire_canonical::read_with_reservation(
                octets,
                IdentityLimits::default().input_bytes,
                &mut Provision(&budget),
            );
            if let Some(error) = budget.stopped.borrow_mut().take() {
                return Err(error);
            }
            let document = document.map_err(|error| match error {
                quire_canonical::ReadError::Limit(limit) => {
                    PolicyStateError::IdentityLimit { value: limit.bound }
                }
                _ => PolicyStateError::Invalid,
            })?;
            let Node::Array(mut members) = document.root().node() else {
                return Err(PolicyStateError::Invalid);
            };
            let body = members.nth(2).ok_or(PolicyStateError::Invalid)?;
            let value = self.read_state(body, &budget)?;
            let expected = key(context, value, self.kind, &budget)?;
            if expected.len() != octets.len() {
                return Err(PolicyStateError::Invalid);
            }
            for (left, right) in expected.iter().zip(octets) {
                budget.work(1)?;
                if left != right {
                    return Err(PolicyStateError::Invalid);
                }
            }
            budget.growth(&mut states, 1)?;
            states.push(value);
        }
        if !input.is_empty() {
            return Err(PolicyStateError::Invalid);
        }
        Ok(states)
    }
    fn read_state(
        &self,
        body: NodeRef<'_>,
        budget: &Budget<'_>,
    ) -> Result<PolicyState, PolicyStateError> {
        budget.work(1)?;
        let Node::Array(mut items) = body.node() else {
            return Err(PolicyStateError::Invalid);
        };
        match (self.kind, items.next().map(|tag| tag.node())) {
            (Kind::Safety, Some(Node::String("safety"))) => {
                let Some(Node::Array(branches)) = items.next().map(|value| value.node()) else {
                    return Err(PolicyStateError::Invalid);
                };
                Ok(PolicyState::Safety(if branches.len() == 0 {
                    SafetyState::Rejected
                } else {
                    SafetyState::Carried
                }))
            }
            (Kind::Bounded { .. }, Some(Node::String("accept"))) => {
                Ok(PolicyState::Bounded(BoundedState::Accept))
            }
            (Kind::Bounded { .. }, Some(Node::String("reject"))) => {
                Ok(PolicyState::Bounded(BoundedState::Reject))
            }
            (Kind::Bounded { upper, .. }, Some(Node::String("bounded"))) => {
                let Some(Node::Array(mut clock)) = items.next().map(|value| value.node()) else {
                    return Err(PolicyStateError::Invalid);
                };
                let Some(Node::String(distance)) = clock.nth(1).map(|value| value.node()) else {
                    return Err(PolicyStateError::Invalid);
                };
                budget.work(distance.len())?;
                let remaining = distance
                    .parse::<u64>()
                    .map_err(|_| PolicyStateError::Invalid)?;
                if remaining > upper {
                    return Err(PolicyStateError::Invalid);
                }
                Ok(PolicyState::Bounded(BoundedState::Open { remaining }))
            }
            _ => Err(PolicyStateError::Invalid),
        }
    }
}
fn take_count(input: &mut &[u8], budget: &Budget<'_>) -> Result<usize, PolicyStateError> {
    budget.work(input.len().min(8))?;
    let (head, rest) = input.split_at_checked(8).ok_or(PolicyStateError::Invalid)?;
    let bytes: [u8; 8] = head.try_into().map_err(|_| PolicyStateError::Invalid)?;
    *input = rest;
    usize::try_from(u64::from_le_bytes(bytes)).map_err(|_| PolicyStateError::Invalid)
}
