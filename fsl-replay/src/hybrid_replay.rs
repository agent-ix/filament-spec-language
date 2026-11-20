// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-450 (ADR-026 HY-2): a `HybridTrace` replayed against the claim's
//! checked hybrid model (FR-447), by FSpec FR-193 "Trace replay".
//!
//! [`static_replay`] checks every trace in exact rational interval
//! arithmetic within the trace's tolerance and settles no value.
//! [`exact_replay`] decides a point trace through modes with polynomial
//! flows exactly, with no floating-point step: each dwell's solution is a
//! vector of polynomials in time with exact rational coefficients, every atom
//! along a dwell a rational function of time whose sign FSL decides by exact
//! real root isolation, and the claim is read over the replayed behaviour by
//! FSpec FR-193 "Hybrid claims". A trace it reproduces is the item's one
//! refutation path (ADR-026 HY-2).
//!
//! Both read the checked package only: the claim's `CheckedHybridClaim`, its
//! `CheckedHybridModel` and the checked bodies of the functions its terms
//! call, by the indices the trace carries. Neither reads source.

mod certificate;
mod class;
mod eval;
mod exact;
mod expand;
mod flow;
mod interval;
mod poly;
mod poly2;
mod real;
mod section;
mod signal;
mod static_replay;
mod trace;

#[cfg(test)]
mod tests;

use filament_exact::LimitKind;
use fsl_semantics::check::{
    CheckedFunctionRef, CheckedGraph, CheckedHybridClaim, CheckedHybridModel, HybridModelIndex,
};

pub use certificate::{
    check_certificate, read_certificate, Acceptance, CertificateOutcome, CertificateRefusal,
    Decisive, Expect, HybridCertificate, ProofCoverage, RefusalCode, Rejection,
    Rule as HybridCertificateRule,
};
pub use class::polynomial_class_gap;
pub use expand::{ExpansionGap, TermLocus};
pub use fsl_foundation::diagnostic::JsonPointer;
pub use real::{ExactReal, Root};
pub use trace::{
    admit, ExactInterval, HybridTrace, TraceBefore, TraceFault, TraceJump, TraceRefusal, TraceState,
};

/// The outcome of static replay (FR-450 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StaticReplay {
    /// No check is definitely false.
    Consistent,
    /// The first check, in trace order, that is definitely false.
    Inconsistent {
        /// The RFC 6901 pointer of the member the check reads.
        at: JsonPointer,
        /// Which check.
        check: StaticCheck,
    },
}

/// FSpec FR-193's static checks 1 to 5.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StaticCheck {
    /// 1: `start` meets an initial entry of its mode.
    Initial,
    /// 2: a state meets its mode's invariant.
    Invariant,
    /// 3: no time runs backwards and a jump takes no time.
    Time,
    /// 4: the source invariant, the guard and the reset across a jump.
    Jump,
    /// 5: a lasso's last state meets the state it returns to.
    Loop,
    /// 6: the finite end of the final dwell.
    End,
}

impl StaticCheck {
    /// The check's number on the stored record's `replay` member.
    pub const fn number(self) -> u8 {
        match self {
            Self::Initial => 1,
            Self::Invariant => 2,
            Self::Time => 3,
            Self::Jump => 4,
            Self::Loop => 5,
            Self::End => 6,
        }
    }
}

/// The replay outcomes kept with a trace on an `inconclusive` record: FSpec
/// FR-193's `replay` member, `static` and, when exact replay ran and did not
/// reproduce, `exact`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayRecord {
    /// The static replay outcome.
    pub static_outcome: StaticReplay,
    /// The exact replay outcome, when exact replay ran and did not
    /// reproduce the trace.
    pub exact: Option<ExactRecord>,
}

/// An exact replay outcome that settles no value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactRecord {
    /// The trace is not eligible for exact replay.
    NotExact,
    /// The first exact check that fails.
    NotReproduced {
        /// The RFC 6901 pointer of the member the check reads.
        at: JsonPointer,
        /// Which check.
        check: ExactCheck,
    },
}

/// The outcome of exact replay (FR-450 "Outputs").
#[derive(Clone, Debug, PartialEq)]
pub enum ExactReplay {
    /// Not a point trace, a mode the trace dwells in without a polynomial
    /// flow, or an expression the replay evaluates that calls a recursive
    /// function.
    NotExact,
    /// The trace reproduces a violation.
    Reproduced(Box<HybridCounterexample>),
    /// The first exact check, in trace order, that fails.
    NotReproduced {
        /// The RFC 6901 pointer of the member the check reads; the empty
        /// pointer, the whole trace, for check 5.
        at: JsonPointer,
        /// Which check.
        check: ExactCheck,
    },
}

/// FSpec FR-193's exact checks 1 to 5.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExactCheck {
    /// 1: `start` satisfies an initial entry.
    Initial,
    /// 2: a dwell's solution reaches the jump's `before` and keeps the
    /// mode's invariant.
    Dwell,
    /// 3: the guard, the jump's time, the reset and the target invariant.
    Jump,
    /// 4: a lasso returns to the state it names after a positive dwell.
    Loop,
    /// 5: the claim is false on the replayed behaviour.
    Claim,
}

impl ExactCheck {
    /// The check's number on the stored record's `replay` member.
    pub const fn number(self) -> u8 {
        match self {
            Self::Initial => 1,
            Self::Dwell => 2,
            Self::Jump => 3,
            Self::Loop => 4,
            Self::Claim => 5,
        }
    }
}

/// A reproduced hybrid trace (FSpec FR-331 `hybrid-counterexample`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridCounterexample {
    /// What the trace reproduces.
    pub kind: HybridCounterexampleKind,
    /// The trace.
    pub trace: HybridTrace,
}

impl HybridCounterexample {
    /// The counterexample in FSpec FR-331's `hybrid-counterexample` wire form
    /// over `model`, of either kind (FR-750): kind `formula` writes
    /// `"formula"`; kind `undefined-evaluation` writes FSpec FR-364's
    /// envelope `{"type", "where": {"position", "time", "locus"}, "cause"}`.
    /// `time` is a point interval when the instant is rational and otherwise
    /// the isolating interval of the instant's root. `locate` spells the
    /// FSpec FR-331 `Locus` of the term node an [`ExpressionLocus`] names in
    /// the checked package; it is called for an `undefined-evaluation`
    /// counterexample only, and the caller that cannot locate the term does
    /// not write the counterexample.
    pub fn to_wire(
        &self,
        model: &CheckedHybridModel,
        locate: &dyn Fn(&ExpressionLocus) -> serde_json::Value,
    ) -> serde_json::Value {
        let kind = match &self.kind {
            HybridCounterexampleKind::Formula => serde_json::json!("formula"),
            HybridCounterexampleKind::UndefinedEvaluation { at, cause } => {
                let (lower, upper) = if let Some(interval) = &at.time_interval {
                    (&interval.lower, &interval.upper)
                } else {
                    match &at.time {
                        ExactReal::Rational(point) => (point, point),
                        ExactReal::Root(root) => (root.lower(), root.upper()),
                    }
                };
                serde_json::json!({
                    "type": "undefined-evaluation",
                    "where": {
                        "position": at.state,
                        "time": {
                            "lower": trace::rational_wire(lower),
                            "upper": trace::rational_wire(upper),
                        },
                        "locus": locate(&at.locus),
                    },
                    "cause": cause.spelling(),
                })
            }
        };
        serde_json::json!({
            "type": "hybrid-counterexample",
            "kind": kind,
            "trace": self.trace.to_wire(model),
        })
    }

    pub(crate) fn measured_bytes(&self) -> usize {
        fn rational(value: &filament_exact::Rational) -> usize {
            value.numerator().to_string().len() + value.denominator().to_string().len()
        }
        fn interval(value: &ExactInterval) -> usize {
            rational(&value.lower) + rational(&value.upper)
        }
        fn state(value: &TraceState) -> usize {
            interval(&value.time) + value.enclosure.iter().map(interval).sum::<usize>() + 8
        }
        let trace = &self.trace;
        state(&trace.start)
            + trace
                .jumps
                .iter()
                .map(|jump| {
                    8 + jump.before.as_ref().map_or(0, |before| {
                        interval(&before.time)
                            + before.enclosure.iter().map(interval).sum::<usize>()
                    }) + state(&jump.state)
                })
                .sum::<usize>()
            + trace.delta.as_ref().map_or(0, rational)
            + trace.loop_to.map_or(0, |_| 8)
            + trace.end.as_ref().map_or(0, |end| {
                interval(&end.time) + end.enclosure.iter().map(interval).sum::<usize>()
            })
    }
}

/// What a reproduced trace shows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridCounterexampleKind {
    /// The claim is false on the replayed behaviour.
    Formula,
    /// An expression the replay evaluates has no value at the decisive
    /// instant.
    UndefinedEvaluation {
        /// The instant and the expression: FSpec FR-193's `where`.
        at: Box<UndefinedInstant>,
        /// Why it has no value.
        cause: UndefinedCause,
    },
}

/// The instant of a behaviour at which an expression has no value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndefinedInstant {
    /// The trace state whose dwell, or whose own point, holds the instant:
    /// 0 for `start`, `k` for the state of `jumps[k - 1]`.
    pub state: usize,
    /// The instant's time stamp, in seconds. When the expression has no
    /// value on an open stretch of instants only, this is the stretch's
    /// start, the infimum of those instants.
    pub time: ExactReal,
    /// The provider's claimed rational interval for this instant, when read
    /// from the wire. Exact replay retains the claim for parity checks.
    pub time_interval: Option<ExactInterval>,
    /// The expression.
    pub locus: ExpressionLocus,
}

/// An expression of the model or the claim, and a place inside it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpressionLocus {
    /// Which expression.
    pub site: ExpressionSite,
    /// Where in it.
    pub term: TermLocus,
}

/// One expression of a checked hybrid model or claim.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExpressionSite {
    /// The condition of `initial[entry]`.
    Initial {
        /// The entry's position.
        entry: usize,
    },
    /// A mode's invariant.
    Invariant {
        /// The mode's position.
        mode: usize,
    },
    /// A mode's rate of a variable.
    Rate {
        /// The mode's position.
        mode: usize,
        /// The variable's position.
        variable: usize,
    },
    /// A transition's guard.
    Guard {
        /// The source mode's position.
        mode: usize,
        /// The transition's position in the mode.
        transition: usize,
    },
    /// A transition's reset constraint.
    Reset {
        /// The source mode's position.
        mode: usize,
        /// The transition's position in the mode.
        transition: usize,
        /// The constraint's position in the reset.
        constraint: usize,
    },
    /// The claim's `on each ... when` guard.
    Activation,
    /// A capture's initializer.
    Capture {
        /// The capture's position.
        capture: usize,
    },
    /// A `holds(...)` atom of the claim's formula, by its position in the
    /// formula's pre-order.
    ClaimAtom {
        /// The atom's position.
        atom: usize,
    },
}

/// Why an expression has no value (FSpec FR-193 "Continuous values and
/// expressions").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UndefinedCause {
    /// A division whose divisor is 0.
    DivisionByZero,
    /// A call's argument outside its parameter's value range.
    OutsideParameterRange {
        /// The called function.
        function: CheckedFunctionRef,
        /// The parameter's position.
        parameter: usize,
    },
    /// A call's result outside its result type's value range.
    OutsideResultRange {
        /// The called function.
        function: CheckedFunctionRef,
    },
    /// A capture's value outside its type's value range.
    OutsideCaptureRange {
        /// The capture's position.
        capture: usize,
    },
}

impl UndefinedCause {
    /// The FSpec FR-193 catalogued hybrid undefined reason, one-to-one over
    /// the variants: `division-by-zero`, `outside-parameter-range`,
    /// `outside-result-range` and `outside-capture-range`.
    pub const fn spelling(&self) -> &'static str {
        match self {
            Self::DivisionByZero => "division-by-zero",
            Self::OutsideParameterRange { .. } => "outside-parameter-range",
            Self::OutsideResultRange { .. } => "outside-result-range",
            Self::OutsideCaptureRange { .. } => "outside-capture-range",
        }
    }
}

/// A claim form exact replay does not decide.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ExactGap {
    /// An `on each` claim under `model-time` whose obligations read the
    /// activation position's state, where a claim atom has no value over an
    /// open cell of activation instants at an instant that moves with the
    /// activation instant along a curve other than `x = tau + c` (`c`
    /// rational): the infimum of the instant over the cell is a limit of a
    /// root curve. It is the only remaining trigger. An instant with no value
    /// that is the same for every activation instant of the cell, rational or
    /// not, and one that moves along `x = tau + c` (rational or irrational
    /// cell start), are decided. Remaining work: QSL-2706.
    #[error("the decisive activation instant of an `on each` claim is not isolated")]
    ActivationInfimum,
    /// A fairness constraint reads the enabledness of a transition whose
    /// reset leaves two or more variables free that the target mode's
    /// invariant couples in one condition. Remaining work: QSL-2639.
    #[error("fairness reads a reset whose free variables the target invariant couples")]
    CoupledReset {
        /// The source mode's position.
        mode: usize,
        /// The transition's position in the mode.
        transition: usize,
    },
    /// A recursive call reached exact evaluation.
    #[error("a recursive call reached exact evaluation")]
    RecursiveCall,
    /// A Boolean where a number is read, or a number where a Boolean is.
    #[error("a term of the wrong kind reached exact evaluation")]
    IllKindedTerm,
}

/// Why a reader cannot replay a trace.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HybridReplayRefusal {
    /// The trace breaks FSpec FR-193 "Hybrid trace":
    /// `invalid_result`/`invalid-value` at its member.
    #[error(transparent)]
    Trace(#[from] TraceRefusal),
    /// The claim names no model of the package.
    #[error("the claim names no hybrid model of the package: {0:?}")]
    UnknownModel(HybridModelIndex),
    /// A called function's checked body cannot be expanded.
    #[error(transparent)]
    Expansion(#[from] ExpansionGap),
    /// Exact replay does not decide this claim form.
    #[error(transparent)]
    Exact(#[from] ExactGap),
    /// The caller's accounting work limit denied the next replay step.
    #[error("hybrid replay exhausted the caller's accounting counter {counter:?}")]
    ResourceExhausted {
        /// The accounting counter whose bound was reached.
        counter: LimitKind,
    },
}

/// Caller-supplied work budget shared by static and exact hybrid replay.
/// One admitted step consumes one `accounting.work_units` unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridReplayMeter {
    /// The caller's configured accounting bound.
    pub work_units: u64,
    /// Successfully charged work units.
    pub consumed: u64,
}

impl HybridReplayMeter {
    /// Create a replay meter from the caller's `accounting.work_units` bound.
    pub const fn new(work_units: u64) -> Self {
        Self {
            work_units,
            consumed: 0,
        }
    }

    /// The words of operand a step works on that one work unit pays for.
    const WORDS_PER_UNIT: u64 = 16;

    /// The number of admitted work units.
    pub const fn consumed(&self) -> u64 {
        self.consumed
    }

    fn charge(&mut self) -> Result<(), HybridReplayRefusal> {
        self.charge_by(1)
    }

    /// Charge one step that works on `weight` words of operand (one per
    /// coefficient and per 64 bits of the largest), before it is taken: a
    /// unit per [`Self::WORDS_PER_UNIT`] words, at least one.
    fn charge_by(&mut self, weight: u64) -> Result<(), HybridReplayRefusal> {
        let units = weight.div_ceil(Self::WORDS_PER_UNIT).max(1);
        match self.consumed.checked_add(units) {
            Some(next) if next <= self.work_units => {
                self.consumed = next;
                Ok(())
            }
            _ => Err(HybridReplayRefusal::ResourceExhausted {
                counter: LimitKind::WorkUnits,
            }),
        }
    }
}

fn model_of<'g>(
    graph: &'g CheckedGraph,
    claim: &CheckedHybridClaim,
) -> Result<&'g CheckedHybridModel, HybridReplayRefusal> {
    graph
        .hybrid_models()
        .get(claim.model.get())
        .ok_or(HybridReplayRefusal::UnknownModel(claim.model))
}

/// FR-450 "Static replay": `trace` against `claim`'s model in `graph`, after
/// the FSpec FR-193 "Hybrid trace" refusals ([`admit`]).
pub fn static_replay(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    trace: &HybridTrace,
    meter: &mut HybridReplayMeter,
) -> Result<StaticReplay, HybridReplayRefusal> {
    let model = model_of(graph, claim)?;
    admit(model, trace)?;
    let mut expander = expand::Expander::new(graph);
    static_replay::replay(model, &mut expander, trace, meter)
}

/// FR-450 "Exact replay": `trace` against `claim` and its model in `graph`,
/// after the FSpec FR-193 "Hybrid trace" refusals ([`admit`]).
pub fn exact_replay(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    trace: &HybridTrace,
    meter: &mut HybridReplayMeter,
) -> Result<ExactReplay, HybridReplayRefusal> {
    let model = model_of(graph, claim)?;
    admit(model, trace)?;
    let mut expander = expand::Expander::new(graph);
    match exact::replay(model, claim, &mut expander, trace, meter) {
        // FR-750 "Replay refusals": a recursive call is not a refusal. Exact
        // replay returns `NotExact` for it, as for a trace that is not a
        // point trace or that dwells in a mode without a polynomial flow.
        //
        // TEMPORARY (QSL-2706 for `ActivationInfimum`, QSL-2639 for
        // `CoupledReset`): both are gaps FR-750 deletes, because exact
        // replay decides those claims (the infimum of a curved instant with
        // no value; quantifier elimination over several coupled free
        // variables). Until then they also settle `NotExact`, the only
        // honest answer short of a verdict.
        Err(HybridReplayRefusal::Exact(
            ExactGap::RecursiveCall | ExactGap::ActivationInfimum | ExactGap::CoupledReset { .. },
        )) => Ok(ExactReplay::NotExact),
        other => other,
    }
}
