// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-751: the core's check of a hybrid provider's certificate by FSpec
//! FR-652's rule tables, in exact rational interval arithmetic. The reader
//! ([`read_certificate`]) turns the member into a [`HybridCertificate`] or a
//! [`CertificateRefusal`]; [`check_certificate`] applies the rules of the
//! certificate's form rule by rule in table order, each over every element
//! it applies to, and reports the first failing rule and the pointer of the
//! box or step it failed at.

use std::collections::BTreeSet;

use filament_exact::Rational;
use fsl_semantics::check::{CheckedGraph, CheckedHybridClaim, CheckedHybridModel};

use super::{
    ExactInterval, HybridCounterexampleKind, HybridReplayMeter, HybridReplayRefusal, HybridTrace,
};
use crate::hybrid::BoundDimension;

mod claim;
mod flowpipe;
mod inductive;
mod machine;
mod reader;
mod wire;
mod witness;

pub use reader::{
    box_advance, box_hull, box_tube, box_within, Fired, HybridBox, InitialHull, ModelReader,
    ReadStop,
};
pub use wire::{read_certificate, CertificateRefusal, Expect, HybridCertificate, RefusalCode};

use machine::{Machine, Stop};

/// FSpec FR-652 "Rule names": the 17 spellings of a rejection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Rule {
    /// A box lacks a variable, names an unknown one or has `lower > upper`.
    BoxShape,
    /// `inductive-enclosure` rule (a): the initial states lie in the boxes.
    A,
    /// `inductive-enclosure` rule (b): each box is forward invariant.
    B,
    /// `inductive-enclosure` rule (c): each transition's post image lies in its target's box.
    C,
    /// `inductive-enclosure` rule (d): the claim holds over every box.
    D,
    /// `flowpipe`: the first boxes cover the initial states.
    Initial,
    /// `flowpipe`: a step's a-priori box encloses its tube.
    Picard,
    /// `flowpipe`: a step's box contains its predecessor's advance.
    Successor,
    /// `flowpipe`: a transition that can fire has its jump step.
    Jump,
    /// `flowpipe`: the steps cover the entry's horizon, jumps and exhaustive dimensions.
    Coverage,
    /// `flowpipe`: the claim is definitely true over the readings.
    Claim,
    /// `validated-witness`: the first box contains the trace's start.
    WitnessStart,
    /// `validated-witness`: a step satisfies the Picard condition.
    WitnessPicard,
    /// `validated-witness`: the flow is decided uniquely over each step.
    WitnessUnique,
    /// `validated-witness`: a jump step takes its transition definitely.
    WitnessGuard,
    /// `validated-witness`: a decisive reading shows the violation.
    WitnessViolation,
    /// The item's work limit stopped the check.
    WorkLimit,
}

impl Rule {
    /// The rule's spelling.
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::BoxShape => "box-shape",
            Self::A => "a",
            Self::B => "b",
            Self::C => "c",
            Self::D => "d",
            Self::Initial => "initial",
            Self::Picard => "picard",
            Self::Successor => "successor",
            Self::Jump => "jump",
            Self::Coverage => "coverage",
            Self::Claim => "claim",
            Self::WitnessStart => "witness-start",
            Self::WitnessPicard => "witness-picard",
            Self::WitnessUnique => "witness-unique",
            Self::WitnessGuard => "witness-guard",
            Self::WitnessViolation => "witness-violation",
            Self::WorkLimit => "work-limit",
        }
    }
}

/// The first failing rule and the pointer it failed at, rooted at
/// `/certificate`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rejection {
    /// The rule.
    pub rule: Rule,
    /// The RFC 6901 pointer.
    pub at: String,
}

/// The decisive reading of an accepted witness (FSpec FR-652 "Readings and
/// the decisive box").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decisive {
    /// The trace state (dwell) whose position the reading is at.
    pub position: usize,
    /// The reading's instants: a point for a `box` reading, the step's
    /// `time` for an a-priori box reading.
    pub time: ExactInterval,
    /// The box the reading reads.
    pub enclosure: Vec<ExactInterval>,
    /// What the reading shows.
    pub kind: HybridCounterexampleKind,
}

/// What an accepted certificate establishes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Acceptance {
    /// A proof's enclosure.
    Proof,
    /// A witness, with its decisive reading.
    Witness(Box<Decisive>),
}

/// The outcome of checking a certificate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertificateOutcome {
    /// Every rule of the form passes.
    Accepted(Acceptance),
    /// The first failing rule.
    Rejected(Rejection),
}

/// A `bounded-solver` proof's `horizon`, `jumps` and `exhaustive`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofCoverage {
    /// The covered duration in seconds.
    pub horizon: Rational,
    /// The covered jump count.
    pub jumps: u64,
    /// The dimensions established complete.
    pub exhaustive: BTreeSet<BoundDimension>,
}

/// A rule's failure: a rejection, or a fault of the checked model.
pub(super) enum Fail {
    Reject(Rule, String),
    Fault(HybridReplayRefusal),
}

pub(super) fn reject<T>(rule: Rule, at: impl Into<String>) -> Result<T, Fail> {
    Err(Fail::Reject(rule, at.into()))
}

/// A read's stop as the failure of the element at `pointer`.
pub(super) trait At<T> {
    fn at(self, pointer: &str) -> Result<T, Fail>;
}

impl<T> At<T> for Result<T, Stop> {
    fn at(self, pointer: &str) -> Result<T, Fail> {
        self.map_err(|stop| match stop {
            Stop::WorkLimit => Fail::Reject(Rule::WorkLimit, pointer.to_owned()),
            Stop::Fault(fault) => Fail::Fault(fault),
        })
    }
}

/// The pointer of `/certificate/<kind>/<k>`.
pub(super) fn step_pointer(k: usize) -> String {
    format!("/certificate/steps/{k}")
}

/// Check `certificate` against the claim's model (FR-751).
///
/// `coverage` is the `bounded-solver` proof's `horizon`, `jumps` and
/// `exhaustive` for a flowpipe; `trace` is the trace a witness accompanies.
/// Each evaluation of one expression over one box is charged to `meter`; a
/// check it stops is `Rejected { rule: work-limit }`.
///
/// # Errors
///
/// [`HybridReplayRefusal`] for a fault of the checked model: a claim whose
/// model the package lacks or a call whose function cannot be expanded.
pub fn check_certificate(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    certificate: &HybridCertificate,
    coverage: Option<&ProofCoverage>,
    trace: Option<&HybridTrace>,
    meter: &mut HybridReplayMeter,
) -> Result<CertificateOutcome, HybridReplayRefusal> {
    check_with(graph, claim, certificate, coverage, trace, meter, true)
}

/// Every rule of `flowpipe` except the containment half of rule `initial`
/// (the first box covering the initial states), for a provider that wants to
/// know whether a sub-box of the initial states still meets the claim's
/// violation. It settles nothing: a core settlement uses
/// [`check_certificate`] only.
///
/// # Errors
///
/// As [`check_certificate`].
#[doc(hidden)]
pub fn probe_flowpipe_claim(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    certificate: &HybridCertificate,
    coverage: &ProofCoverage,
    meter: &mut HybridReplayMeter,
) -> Result<ProbeOutcome, HybridReplayRefusal> {
    Ok(match check_with(graph, claim, certificate, Some(coverage), None, meter, false)? {
        CertificateOutcome::Rejected(Rejection {
            rule: Rule::Claim, ..
        }) => ProbeOutcome::ClaimRejected,
        _ => ProbeOutcome::NotRejected,
    })
}

/// What [`probe_flowpipe_claim`] found. It is never a proof: the initial
/// containment was not checked.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbeOutcome {
    /// The claim rule rejects the flowpipe.
    ClaimRejected,
    /// It does not (accepted by the checked rules, or another rule rejects).
    NotRejected,
}

fn check_with(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    certificate: &HybridCertificate,
    coverage: Option<&ProofCoverage>,
    trace: Option<&HybridTrace>,
    meter: &mut HybridReplayMeter,
    initial_containment: bool,
) -> Result<CertificateOutcome, HybridReplayRefusal> {
    let model: &CheckedHybridModel = graph
        .hybrid_models()
        .get(claim.model.get())
        .ok_or(HybridReplayRefusal::UnknownModel(claim.model))?;
    let mut machine = Machine::new(graph, meter);
    let mx = match machine.expand_model(model) {
        Ok(mx) => mx,
        Err(Stop::WorkLimit) => {
            return Ok(CertificateOutcome::Rejected(Rejection {
                rule: Rule::WorkLimit,
                at: "/certificate".to_owned(),
            }))
        }
        Err(Stop::Fault(fault)) => return Err(fault),
    };
    let result = match &certificate.body {
        wire::Body::Inductive(boxes) => {
            inductive::check(&mut machine, &mx, model, claim, boxes).map(|()| Acceptance::Proof)
        }
        wire::Body::Flowpipe(steps) => {
            flowpipe::check(
                &mut machine,
                &mx,
                model,
                claim,
                steps,
                coverage,
                initial_containment,
            )
            .map(|()| Acceptance::Proof)
        }
        wire::Body::Witness(dwells) => match trace {
            Some(trace) => witness::check(&mut machine, &mx, model, claim, dwells, trace)
                .map(|decisive| Acceptance::Witness(Box::new(decisive))),
            None => reject(Rule::WitnessStart, "/certificate/dwells/0/0"),
        },
    };
    match result {
        Ok(acceptance) => Ok(CertificateOutcome::Accepted(acceptance)),
        Err(Fail::Reject(rule, at)) => Ok(CertificateOutcome::Rejected(Rejection { rule, at })),
        Err(Fail::Fault(fault)) => Err(fault),
    }
}

/// The variable names of `model`, in declaration order.
pub(super) fn names(model: &CheckedHybridModel) -> Vec<&str> {
    model
        .variables
        .iter()
        .map(|variable| variable.name.as_str())
        .collect()
}

/// A wire box as a typed box when its keys are exactly `names` and every
/// interval has `lower <= upper` (rule `box-shape`).
pub(super) fn shaped(bx: &wire::WireBox, names: &[&str]) -> Option<machine::Bx> {
    if bx.0.len() != names.len() {
        return None;
    }
    names
        .iter()
        .map(|name| {
            let (_, interval) = bx.0.iter().find(|(key, _)| key == name)?;
            (interval.lower <= interval.upper).then(|| super::interval::Interval {
                lower: Some(interval.lower.clone()),
                upper: Some(interval.upper.clone()),
            })
        })
        .collect()
}
