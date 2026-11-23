// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338 "Canonical translation state", the TP-2 alternatives: the canonical
//! key of a bounded monitor's state, and the translation of the two bounded
//! forms EN-1 certifies, `eventually[0,u] holds(A)` and `always[0,u]
//! holds(A)`, activated at the origin under a bounded profile or the
//! infinite-trace one, or activated `on each` (guarded or not) under a
//! bounded profile. Under the infinite-trace profile a terminal state
//! stutters for ever, each stutter a counted step that re-reads its letter,
//! so the window runs out on the stutter and no false extension closes it.
//!
//! The monitor's undecided state is `["bounded", clock, branches]` with one
//! branch `[[], [], windows, []]`. `on origin`, the clock is `["origin",
//! remaining]`, `remaining` the counted distance left of the root's horizon,
//! and the one window carries the root's open debt, `[[], "positive",
//! "deadline", remaining]` for `eventually` (the earliest open deadline) and
//! `[[], "positive", "extent", remaining]` for `always` (the furthest
//! required position). Both counters are measured from the letter just read.
//! `on each`, the clock is `["each"]`, with no remaining distance: the
//! deadline or extent lives in the window entry alone, and the branch holds
//! the one entry merged across every live activation, or none while no
//! activation is live ([`BoundedState::Idle`]). The decided states are the
//! absorbing `["accept"]` and `["reject"]`.
//!
//! A step first subtracts its counted distance: a step that counts leaves
//! the window once nothing remains, and closes the region before any letter
//! of the next distance is read (FSpec FR-091's closed boundary). An
//! undischarged `eventually` debt then rejects and a surviving `always`
//! obligation accepts (`on each`: discharges, and the monitor idles). A step
//! that does not count (ADR-027 PB-3) leaves the counter where it was and
//! its letter is still read. At a terminal state the false extension
//! supplies the counted false letters through the remaining horizon and
//! closes the last region.

use fsl_foundation::bound::TemporalUpper;
use fsl_foundation::IdentityLimits;
use fsl_semantics::check::{
    Activation, CheckedFormula, CheckedInterval, CheckedTemporalClause, FormulaNode, FormulaNodeId,
    TemporalProfile, UnaryTemporal,
};
use quire_canonical::{Node, NodeRef};

use crate::automaton::key::{
    ascending, canonical, canonical_state_key, decimal, exactly, AutomatonStateKey,
    ContextActivation, KeyRefusal, NotTranslated, StateContext, TranslationRefusal,
    AUTOMATON_STATE_KEY_V1,
};
use crate::simulation::model::encode::Enc;

/// Which operator the bounded root is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundedKind {
    /// `eventually[0,u] holds(A)`: a debt with a deadline.
    Eventually,
    /// `always[0,u] holds(A)`: an obligation with an extent.
    Always,
}

impl BoundedKind {
    pub(super) fn decide(self, remaining: u64, holds: bool) -> BoundedState {
        match (self, holds) {
            (BoundedKind::Eventually, true) => BoundedState::Accept,
            (BoundedKind::Always, false) => BoundedState::Reject,
            (BoundedKind::Eventually, false) | (BoundedKind::Always, true) => {
                BoundedState::Open { remaining }
            }
        }
    }
    pub(super) fn step(self, before: BoundedState, counted: bool, holds: bool) -> BoundedState {
        let BoundedState::Open { remaining } = before else {
            return before;
        };
        if !counted {
            return self.decide(remaining, holds);
        }
        match remaining.checked_sub(1) {
            Some(remaining) => self.decide(remaining, holds),
            // The region closed before the next distance was read.
            None => match self {
                BoundedKind::Eventually => BoundedState::Reject,
                BoundedKind::Always => BoundedState::Accept,
            },
        }
    }

    /// The window kind's spelling in a key.
    pub(super) const fn window(self) -> &'static str {
        match self {
            Self::Eventually => "deadline",
            Self::Always => "extent",
        }
    }
}

/// A state of a [`BoundedTranslation`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BoundedState {
    /// The root's debt is open, with `remaining` counted steps of its
    /// horizon left after the letter just read.
    Open {
        /// The counted distance left.
        remaining: u64,
    },
    /// An `on each` monitor with no live activation: no debt is open, the
    /// window list is empty, and the next activation position adds one.
    Idle,
    /// Decided in favour of the clause; absorbing.
    Accept,
    /// Decided against the clause; absorbing.
    Reject,
}

impl BoundedState {
    /// Whether the state rejects.
    pub const fn is_rejecting(self) -> bool {
        matches!(self, Self::Reject)
    }
}

/// The canonical translation of one bounded clause under one context
/// (FR-338): the monitor both EN-1 and the core's closure checker read.
#[derive(Clone, Debug)]
pub struct BoundedTranslation {
    kind: BoundedKind,
    /// Whether the clause reads the infinite trace, whose terminal state
    /// stutters forever instead of closing with false letters.
    stutters: bool,
    /// Whether the clause is `on each`: every activation position adds the
    /// root's window, merged into the one the monitor carries.
    each: bool,
    upper: u64,
    atom: FormulaNodeId,
    context: Enc,
    context_octets: Vec<u8>,
    max_bytes: u64,
}

impl BoundedTranslation {
    /// Translate an origin-activated bounded formula under its checked
    /// item context. Probabilistic events supply a formula rather than a
    /// temporal clause; the context owns their occurrence and binding.
    pub(super) fn formula(
        formula: &CheckedFormula,
        context: &StateContext,
    ) -> Result<Self, TranslationRefusal> {
        if !context.profile.is_bounded() || context.activation != ContextActivation::Origin {
            return Err(TranslationRefusal::NotTranslated(NotTranslated));
        }
        let shape = qualified_formula_shape(formula, context.profile);
        Self::in_profile(shape, context)
    }

    /// The translation of a bounded operator under a bounded or the
    /// infinite-trace profile; [`Self::formula`] admits the bounded ones
    /// alone.
    fn in_profile(
        shape: Option<(BoundedKind, u64, FormulaNodeId)>,
        context: &StateContext,
    ) -> Result<Self, TranslationRefusal> {
        let (kind, upper, atom) = shape.ok_or(TranslationRefusal::NotTranslated(NotTranslated))?;
        // A bounded clause is keyed under "origin" or "each", an
        // infinite-trace one under "infinite" (FR-338); an `on each` clause
        // with a `when` guard is keyed "each-when" under every profile.
        let infinite = context.profile == TemporalProfile::InfiniteTrace;
        let each = match (infinite, context.activation) {
            (true, ContextActivation::Infinite) | (false, ContextActivation::Origin) => false,
            (false, ContextActivation::Each | ContextActivation::EachWhen) => true,
            _ => return Err(TranslationRefusal::NotTranslated(NotTranslated)),
        };
        let max_bytes = IdentityLimits::default().input_bytes;
        let (context, context_octets) = context
            .encoded(max_bytes)
            .map_err(TranslationRefusal::Key)?;
        Ok(Self {
            kind,
            stutters: infinite,
            each,
            upper,
            atom,
            context,
            context_octets,
            max_bytes,
        })
    }

    /// The translation of `clause` under `context`.
    ///
    /// # Errors
    ///
    /// [`TranslationRefusal::NotTranslated`] when the clause is not
    /// `eventually[0,u] holds(A)` or `always[0,u] holds(A)` on origin under
    /// a step-measured bounded profile or the infinite-trace one, or `on
    /// each` under a bounded profile, or `context` is not its own;
    /// [`TranslationRefusal::Key`] when the context does not encode.
    pub fn clause(
        clause: &CheckedTemporalClause,
        context: &StateContext,
    ) -> Result<Self, TranslationRefusal> {
        closure_shape(clause).ok_or(TranslationRefusal::NotTranslated(NotTranslated))?;
        let activation = match clause.activation() {
            Activation::Origin if clause.profile() == TemporalProfile::InfiniteTrace => {
                ContextActivation::Infinite
            }
            Activation::Origin => ContextActivation::Origin,
            Activation::Each { guarded: true } => ContextActivation::EachWhen,
            Activation::Each { guarded: false } => ContextActivation::Each,
        };
        if context.profile != clause.profile() || context.activation != activation {
            return Err(TranslationRefusal::NotTranslated(NotTranslated));
        }
        Self::in_profile(bounded_formula_shape(clause.formula()), context)
    }

    /// Whether the clause reads the infinite trace: a terminal state then
    /// stutters, each stutter a counted step that re-reads its letter, and
    /// the monitor closes by no false extension.
    pub const fn stutters(&self) -> bool {
        self.stutters
    }

    /// The clause's `holds` atom.
    pub const fn atom(&self) -> FormulaNodeId {
        self.atom
    }

    /// The horizon: the last counted distance the clause reads.
    pub const fn horizon(&self) -> u64 {
        self.upper
    }

    /// The root operator.
    pub const fn kind(&self) -> BoundedKind {
        self.kind
    }

    /// The state after position 0, whose atom is `holds`.
    pub fn start(&self, holds: bool) -> BoundedState {
        self.decide(self.upper, holds)
    }

    /// The state after one more position from `before`, reached by a step
    /// that `counted` or not, whose atom is `holds`. A step that counts out
    /// of an exhausted window reads no letter, so `holds` is ignored there.
    pub fn step(&self, before: BoundedState, counted: bool, holds: bool) -> BoundedState {
        self.kind.step(before, counted, holds)
    }

    /// Whether the clause is `on each`: its states are read with
    /// [`Self::start_each`] and [`Self::step_each`].
    pub const fn each(&self) -> bool {
        self.each
    }

    /// The state of an `on each` clause after position 0, whose atom is
    /// `holds` and which `activates` when its `when` condition holds (always
    /// when unguarded).
    pub fn start_each(&self, holds: bool, activates: bool) -> BoundedState {
        self.read_each(None, holds, activates)
    }

    /// The state of an `on each` clause after one more position from
    /// `before`, reached by a step that `counted` or not. The step first
    /// subtracts its counted distance, and closes the window before any
    /// letter when none remains: an open `eventually` debt rejects and an
    /// `always` extent is discharged. The position's letter is then read
    /// against the entry that survives, and only afterwards does an
    /// activation at this position add its own window, which includes it
    /// (FR-338 "Guarded `on each` activation").
    pub fn step_each(
        &self,
        before: BoundedState,
        counted: bool,
        holds: bool,
        activates: bool,
    ) -> BoundedState {
        let open = match before {
            BoundedState::Open { remaining } if !counted => Some(remaining),
            BoundedState::Open { remaining } => match remaining.checked_sub(1) {
                Some(remaining) => Some(remaining),
                None => match self.kind {
                    BoundedKind::Eventually => return BoundedState::Reject,
                    BoundedKind::Always => None,
                },
            },
            BoundedState::Idle => None,
            decided @ (BoundedState::Accept | BoundedState::Reject) => return decided,
        };
        self.read_each(open, holds, activates)
    }

    /// The state after a letter whose atom is `holds` read against `open`,
    /// the remaining distance of the entry that survived the step, if any.
    fn read_each(&self, open: Option<u64>, holds: bool, activates: bool) -> BoundedState {
        let merged = |keep: fn(u64, u64) -> u64| match (open, activates) {
            (Some(older), true) => Some(keep(older, self.upper)),
            (None, true) => Some(self.upper),
            (older, false) => older,
        };
        let remaining = match self.kind {
            // A satisfying letter discharges every open occurrence, the new
            // one included (the position lies in every open window); an
            // unsatisfied activation keeps the earliest deadline.
            BoundedKind::Eventually if holds => None,
            BoundedKind::Eventually => merged(u64::min),
            // A failing letter inside any open or new window rejects; the
            // furthest extent is the one kept.
            BoundedKind::Always if !holds && (open.is_some() || activates) => {
                return BoundedState::Reject;
            }
            BoundedKind::Always => merged(u64::max),
        };
        remaining.map_or(BoundedState::Idle, |remaining| BoundedState::Open {
            remaining,
        })
    }

    /// The state the false extension of a terminal state `before` closes
    /// to: the counted false letters through the remaining horizon, then
    /// the closed boundary. An `on each` clause adds no activation at a
    /// false-extension position, so a surviving `always` extent discharges
    /// at its boundary into [`BoundedState::Idle`].
    pub fn terminal(&self, before: BoundedState) -> BoundedState {
        match (before, self.kind) {
            (BoundedState::Open { remaining: 0 }, BoundedKind::Always) if self.each => {
                BoundedState::Idle
            }
            (BoundedState::Open { remaining: 0 }, BoundedKind::Always) => BoundedState::Accept,
            (BoundedState::Open { .. }, _) => BoundedState::Reject,
            (decided, _) => decided,
        }
    }

    /// The state with `remaining` counted steps left after a letter whose
    /// atom is `holds`.
    fn decide(&self, remaining: u64, holds: bool) -> BoundedState {
        self.kind.decide(remaining, holds)
    }

    /// The canonical key of `state` under this translation's context.
    ///
    /// # Errors
    ///
    /// [`KeyRefusal::Encoding`] when the key exceeds the identity byte
    /// budget; no truncated key is produced. [`KeyRefusal::Unreachable`] for
    /// [`BoundedState::Idle`] of a clause that is not `on each`.
    pub fn key(&self, state: BoundedState) -> Result<AutomatonStateKey, KeyRefusal> {
        if state == BoundedState::Idle && !self.each {
            return Err(KeyRefusal::Unreachable);
        }
        canonical_state_key(&self.context, self.state_enc(state), self.max_bytes)
            .map_err(|_| KeyRefusal::Encoding)
    }

    fn state_enc(&self, state: BoundedState) -> Enc {
        match state {
            BoundedState::Accept => Enc::Array(vec![Enc::text("accept")]),
            BoundedState::Reject => Enc::Array(vec![Enc::text("reject")]),
            BoundedState::Idle => self.open_enc(None),
            BoundedState::Open { remaining } => self.open_enc(Some(remaining)),
        }
    }

    /// `["bounded", clock, [branch]]` for the one window `remaining` names,
    /// or for none. An `on each` clock is `["each"]` with no remaining
    /// distance: the deadline or extent lives in the window entry alone.
    fn open_enc(&self, remaining: Option<u64>) -> Enc {
        let distance = |remaining: u64| Enc::text(remaining.to_string());
        let windows = remaining
            .map(|remaining| {
                Enc::Array(vec![
                    Enc::Array(Vec::new()),
                    Enc::text("positive"),
                    Enc::text(self.kind.window()),
                    distance(remaining),
                ])
            })
            .into_iter()
            .collect();
        let branch = Enc::Array(vec![
            Enc::Array(Vec::new()),
            Enc::Array(Vec::new()),
            Enc::Array(windows),
            Enc::Array(Vec::new()),
        ]);
        let clock = if self.each {
            Enc::Array(vec![Enc::text("each")])
        } else {
            Enc::Array(vec![Enc::text("origin"), distance(remaining.unwrap_or(0))])
        };
        Enc::Array(vec![Enc::text("bounded"), clock, Enc::Array(vec![branch])])
    }

    /// Admit `octets` as a key of this translation's context, refusing every
    /// FR-338 admission negative before the key is used for membership.
    ///
    /// # Errors
    ///
    /// The first [`KeyRefusal`] the octets meet.
    pub fn admit(&self, octets: &[u8]) -> Result<(AutomatonStateKey, BoundedState), KeyRefusal> {
        let document =
            quire_canonical::read(octets, self.max_bytes).map_err(|_| KeyRefusal::Malformed)?;
        if canonical(document.root(), self.max_bytes)? != octets {
            return Err(KeyRefusal::NotCanonical);
        }
        let Node::Array(items) = document.root().node() else {
            return Err(KeyRefusal::Shape);
        };
        let [discriminator, context, state] = exactly(items).ok_or(KeyRefusal::Shape)?;
        if !matches!(discriminator.node(), Node::String(AUTOMATON_STATE_KEY_V1)) {
            return Err(KeyRefusal::Discriminator);
        }
        if canonical(context, self.max_bytes)? != self.context_octets {
            return Err(KeyRefusal::ForeignContext);
        }
        let state = self.state(state)?;
        Ok((AutomatonStateKey(octets.to_vec()), state))
    }

    /// The bounded state `body` encodes. `#[string_edge]`: reads an imported
    /// key's state tag by its wire spelling.
    #[ix_trace_rs::string_edge]
    fn state(&self, body: NodeRef<'_>) -> Result<BoundedState, KeyRefusal> {
        let Node::Array(mut items) = body.node() else {
            return Err(KeyRefusal::Members);
        };
        let tag = match items.next().map(|tag| tag.node()) {
            Some(Node::String(tag)) => tag,
            _ => return Err(KeyRefusal::Members),
        };
        match tag {
            "accept" | "reject" => {
                if items.next().is_some() {
                    return Err(KeyRefusal::Members);
                }
                Ok(if tag == "accept" {
                    BoundedState::Accept
                } else {
                    BoundedState::Reject
                })
            }
            "bounded" => {
                let [clock, branches] = exactly(items).ok_or(KeyRefusal::Members)?;
                self.open(clock, branches)
            }
            "safety" | "buchi" => Err(KeyRefusal::WrongForm(tag.to_owned())),
            other => Err(KeyRefusal::UnknownTag(other.to_owned())),
        }
    }

    /// The clock and its one branch. An `on origin` clock is `["origin",
    /// remaining]` and its branch carries the root's window with that same
    /// remaining distance; an `on each` clock is `["each"]` and its branch
    /// carries the one merged window, or none. `#[string_edge]`: reads an
    /// imported key's clock and window spellings.
    #[ix_trace_rs::string_edge]
    fn open(&self, clock: NodeRef<'_>, branches: NodeRef<'_>) -> Result<BoundedState, KeyRefusal> {
        let Node::Array(mut clock) = clock.node() else {
            return Err(KeyRefusal::Members);
        };
        let mode = match clock.next().map(|mode| mode.node()) {
            Some(Node::String(mode)) => mode,
            _ => return Err(KeyRefusal::Members),
        };
        let (own, other) = if self.each {
            ("each", "origin")
        } else {
            ("origin", "each")
        };
        // The clock of the other activation is another context's.
        if mode == other {
            return Err(KeyRefusal::ForeignContext);
        }
        if mode != own {
            return Err(KeyRefusal::Members);
        }
        let remaining = if self.each {
            // A stray remaining distance in an `on each` clock is no member.
            if clock.next().is_some() {
                return Err(KeyRefusal::Members);
            }
            None
        } else {
            let remaining = match clock.next().map(|remaining| remaining.node()) {
                Some(Node::String(text)) => decimal(text)?,
                _ => return Err(KeyRefusal::Members),
            };
            if clock.next().is_some() {
                return Err(KeyRefusal::Members);
            }
            if remaining > self.upper {
                return Err(KeyRefusal::Unreachable);
            }
            Some(remaining)
        };
        let Node::Array(branches) = branches.node() else {
            return Err(KeyRefusal::Members);
        };
        let mut previous: Option<Vec<u8>> = None;
        let mut count = 0_usize;
        let mut window = None;
        for branch in branches {
            ascending(&mut previous, canonical(branch, self.max_bytes)?)?;
            count += 1;
            window = self.configuration(branch, remaining)?;
        }
        if count != 1 {
            // No branch is false, and the clause has one root debt.
            return Err(KeyRefusal::Unreachable);
        }
        Ok(match (remaining, window) {
            (Some(remaining), _) => BoundedState::Open { remaining },
            (None, Some(remaining)) => BoundedState::Open { remaining },
            (None, None) => BoundedState::Idle,
        })
    }

    /// One elementary configuration `[obligations, past, windows,
    /// acceptance]`: the root's one window, or for an `on each` clock the
    /// one merged window or none, and nothing else. The window's distance,
    /// when `on each` carries one.
    #[ix_trace_rs::string_edge]
    fn configuration(
        &self,
        branch: NodeRef<'_>,
        remaining: Option<u64>,
    ) -> Result<Option<u64>, KeyRefusal> {
        let Node::Array(members) = branch.node() else {
            return Err(KeyRefusal::Members);
        };
        let [obligations, past, windows, acceptance] =
            exactly(members).ok_or(KeyRefusal::Members)?;
        // The root's debt is stored in its window, not as an obligation, and
        // the clause has no past or least-fixed-point occurrence an entry
        // could name: a finite debt on a model subject needs no acceptance
        // set, and on a protocol subject a carry is not a member.
        for fact in [obligations, past, acceptance] {
            let Node::Array(mut entries) = fact.node() else {
                return Err(KeyRefusal::Members);
            };
            if entries.next().is_some() {
                return Err(KeyRefusal::Unresolved);
            }
        }
        let Node::Array(windows) = windows.node() else {
            return Err(KeyRefusal::Members);
        };
        let windows: Vec<NodeRef<'_>> = windows.collect();
        let window = match windows.as_slice() {
            [window] => window,
            // Only an `on each` monitor with no live activation has none.
            [] if remaining.is_none() => return Ok(None),
            _ => return Err(KeyRefusal::Unresolved),
        };
        let Node::Array(entry) = window.node() else {
            return Err(KeyRefusal::Members);
        };
        let [path, polarity, kind, distance] = exactly(entry).ok_or(KeyRefusal::Members)?;
        let Node::Array(mut path) = path.node() else {
            return Err(KeyRefusal::Members);
        };
        if path.next().is_some() {
            return Err(KeyRefusal::Unresolved);
        }
        match polarity.node() {
            Node::String("positive") => {}
            Node::String("negative") => return Err(KeyRefusal::Unresolved),
            _ => return Err(KeyRefusal::Members),
        }
        match kind.node() {
            Node::String(spelling) if spelling == self.kind.window() => {}
            Node::String(_) => return Err(KeyRefusal::Unresolved),
            _ => return Err(KeyRefusal::Members),
        }
        let distance = match distance.node() {
            Node::String(text) => decimal(text)?,
            _ => return Err(KeyRefusal::Members),
        };
        match remaining {
            Some(remaining) if distance != remaining => Err(KeyRefusal::Unreachable),
            None if distance > self.upper => Err(KeyRefusal::Unreachable),
            _ => Ok(Some(distance)),
        }
    }
}

/// The root operator, upper bound and `holds` atom of a clause this module
/// translates: `eventually[0,u] holds(A)` or `always[0,u] holds(A)` on origin
/// under a step-measured bounded profile.
pub fn bounded_shape(clause: &CheckedTemporalClause) -> Option<(BoundedKind, u64, FormulaNodeId)> {
    if !clause.profile().is_bounded() || !matches!(clause.activation(), Activation::Origin) {
        return None;
    }
    bounded_formula_shape(clause.formula())
}

/// The shape of a clause whose closure certificate this module keys: that of
/// [`bounded_shape`], the same operators on origin under the infinite-trace
/// profile (FR-125), and the same operators under `on each`, guarded or not,
/// under a step-measured bounded profile (FR-338 "Guarded `on each`
/// activation").
pub fn closure_shape(clause: &CheckedTemporalClause) -> Option<(BoundedKind, u64, FormulaNodeId)> {
    let profile = clause.profile();
    let admitted = match clause.activation() {
        Activation::Origin => profile.is_bounded() || profile == TemporalProfile::InfiniteTrace,
        Activation::Each { .. } => profile.is_bounded(),
    };
    if !admitted {
        return None;
    }
    bounded_formula_shape(clause.formula())
}

pub(super) fn bounded_formula_shape(
    formula: &CheckedFormula,
) -> Option<(BoundedKind, u64, FormulaNodeId)> {
    let FormulaNode::Unary {
        operator,
        interval: Some(CheckedInterval::Steps(interval)),
        operand,
    } = formula.nodes().get(formula.root().index())?
    else {
        return None;
    };
    let kind = match operator {
        UnaryTemporal::Eventually => BoundedKind::Eventually,
        UnaryTemporal::Always => BoundedKind::Always,
        UnaryTemporal::Once | UnaryTemporal::Historically => return None,
    };
    let TemporalUpper::Finite(upper) = interval.upper() else {
        return None;
    };
    if interval.lower() != 0 {
        return None;
    }
    matches!(
        formula.nodes().get(operand.index()),
        Some(FormulaNode::Holds { .. })
    )
    .then_some((kind, upper, *operand))
}

/// The shape the qualified probability path reads: [`bounded_formula_shape`],
/// or, under the event-position false-extension profile, a root `holds(P)`
/// with no temporal operator as `eventually[0,0] holds(P)` (FR-201). The
/// checked formula itself is never rewritten.
pub(super) fn qualified_formula_shape(
    formula: &CheckedFormula,
    profile: TemporalProfile,
) -> Option<(BoundedKind, u64, FormulaNodeId)> {
    if profile == TemporalProfile::EventPositionFalseExtension {
        let root = formula.root();
        if matches!(
            formula.nodes().get(root.index()),
            Some(FormulaNode::Holds { .. })
        ) {
            return Some((BoundedKind::Eventually, 0, root));
        }
    }
    bounded_formula_shape(formula)
}
