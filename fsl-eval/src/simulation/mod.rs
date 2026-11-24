// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-101 (implementing FSpec FR-181): finite exploration and seeded
//! sampling of a caller-supplied transition system, in canonical order, with
//! typed state keys and the pinned `filament.simulation.sampler/v1` generator.
//!
//! This module is the exploration engine only. It knows nothing about FSL's
//! own models, formal semantics or admitted packages: a caller implements
//! `TransitionSystem` over whatever state it has, and the engine walks it --
//! exhaustively by deterministic breadth-first search (`explore_request`,
//! producing an `Outcome`), or by seeded sampling over the same successor
//! relation (`sample_request`, producing a `Trace`). `replay` re-runs a
//! `Trace` against a `TransitionSystem`, independently of the engine.
//!
//! `explore_request`, `sample_request` and `sample_request_observed` are the
//! only public entries that explore or sample: `explore`, `sample` and
//! `Sampler` are `pub(crate)`, so no caller skips the requires-bound check or
//! the sampler's `GeneratorMismatch` check. `weighted_choice` is the generator's one
//! selection function (FR-188): it reads draws at a choice point and runs no
//! transition system. `sample_probabilistic` samples a probabilistic model
//! under a workload (FR-188), over FR-187's step structure.

mod choice;
mod discovery;
mod expansion;
mod explore;
mod footprint;
mod frontier;
mod key;
pub mod model;
mod not_simulated;
mod order;
mod progress;
pub mod protocol;
mod reduction;
mod sample;
mod sampled;
mod subject;
mod trace;

pub use choice::{ChoicePoint, TotalAboveDrawRange, Weights, WeightsRefusal, weighted_choice};
pub use discovery::{
    Coverage, Discovered, Enumeration, EnumerationBounds, EnumerationEnd, EnumerationRequest,
    FrontierPosition, enumerate_request, sampled_coverage,
};
pub use expansion::{Expansion, ExpansionLimit, ExpansionStop, StateFindings};
pub use explore::{
    CANCELLED_STOP, Exploration, Limits, Outcome, Stats, TransitionSystem, explore_request,
};
pub use footprint::{Footprint, Location, any_affects, independent};
pub use frontier::{Frontier, Limit};
pub use key::{EncodingRefusal, StateKey};
pub use not_simulated::{BOUND_POSITIONS, ClockReferenceDomain, NotSimulated, RequiresBound, Root};
pub use progress::{FairLoop, ZeroProgress, ZeroProgressError};
pub use reduction::{
    ConstraintValue, OfferedReductions, Permutation, SymmetryClass, SymmetryGroup,
};
pub use sample::{
    ObservedSample, ObserverDecision, SampleExpansion, SampleObservation, SamplePrefix,
    SampleRequest, SampleSuccessorRef, SampleSuccessors, sample_request, sample_request_observed,
    sample_request_until,
};
pub use sampled::{
    ProbabilisticSampler, SampleStop, SampledBehaviour, SampledDisagreement, SampledPosition,
    SampledRecord, SampledReplay, SampledReplayRefusal, SampledStep, StepAt, TimedDelayRefusal,
    draw_timed_delay, replay_sampled, sample_probabilistic,
};
pub use subject::{
    Bounded, ConstraintError, Cut, InitialStateRefusal, SubjectSystem, SubjectTransition,
};
pub use trace::{ReplayError, SampleProvenance, Step, StopReason, Trace, replay};
