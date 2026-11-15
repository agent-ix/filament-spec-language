// SPDX-License-Identifier: AGPL-3.0-or-later
//! The workload counters (feature `counters`, off by default): what an
//! axis's input makes the engines do, counted at the public seams so that a
//! change to a generator cannot silently change the workload a benchmark
//! times.
//!
//! The qualified core forbids a mutable global (FR-284), so the counters are
//! not incremented inside `fsl-eval`. They are read where a public seam
//! shows them:
//!
//! - [`fingerprint`] drives a breadth-first search over a
//!   [`ModelSystem`] through its [`TransitionSystem`] seam:
//!   `expansions` is the calls of `successors`, `edges` and
//!   `generated_candidates` are the successor tuples it returned, `states`
//!   the distinct canonical keys and `canonical_encodes` the calls of
//!   `ModelSystem::state_key`.
//! - `atom_evaluations` is the `holds` atoms the bench's own
//!   [`FieldLetters`](crate::mc::FieldLetters) evaluated in a `check_model`
//!   run.
//!
//! A candidate an expansion generates and a contract rejects is not visible
//! at the seam, so `generated_candidates` equals `edges` until the engine
//! reports it. A counter an axis cannot observe is zero.
//!
//! These counters fingerprint the generator, not the engine run a bench
//! times. `expansions`, `edges`, `generated_candidates`, `canonical_encodes`
//! and `states` come from the bench's own search over the plain model, so
//! they do not move when an engine change makes the engine expand, encode or
//! generate less, and they cannot show that improvement. For `mc/liveness`
//! at 2x2 the search counts 4 states and 8 edges while `check_model` explores
//! 12 product states and 28 edges. A before and after of engine work needs
//! the engine to report it.

use std::collections::{BTreeSet, VecDeque};

use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::TransitionSystem;

use crate::mc::{AxisError, Work};
use crate::world::World;

/// The counted workload of one axis input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counters {
    /// Distinct states.
    pub states: u64,
    /// Edges: successor tuples returned over all expansions.
    pub edges: u64,
    /// Candidate successors the system generated and handed back.
    pub generated_candidates: u64,
    /// Canonical state-key encodings.
    pub canonical_encodes: u64,
    /// `holds` atoms evaluated.
    pub atom_evaluations: u64,
    /// States expanded.
    pub expansions: u64,
}

impl Counters {
    /// The counters of an engine's own [`Work`]: states and edges only.
    pub fn of_work(work: Work) -> Self {
        Self {
            states: work.states,
            edges: work.edges,
            ..Self::default()
        }
    }
}

fn fail(detail: impl std::fmt::Debug) -> AxisError {
    AxisError(format!("fingerprint: {detail:?}"))
}

/// The counters of the full reachable graph of `world`'s model system, with
/// `atom_evaluations` carried from the run that read the letters.
///
/// # Errors
///
/// [`AxisError`] when the system does not admit, an expansion stops or a key
/// has no encoding.
pub fn fingerprint(world: &World, atom_evaluations: u64) -> Result<Counters, AxisError> {
    let system = ModelSystem::new(world.simulation()).map_err(fail)?;
    let mut counters = Counters {
        atom_evaluations,
        ..Counters::default()
    };
    let mut seen = BTreeSet::new();
    let mut pending = VecDeque::new();
    for state in system.initial() {
        visit(&system, state, &mut seen, &mut pending, &mut counters)?;
    }
    while let Some(state) = pending.pop_front() {
        let expansion = system.successors(&state).map_err(fail)?;
        counters.expansions += 1;
        let returned = u64::try_from(expansion.successors.len()).unwrap_or(u64::MAX);
        counters.edges += returned;
        counters.generated_candidates += returned;
        for (_, next) in expansion.successors {
            visit(&system, next, &mut seen, &mut pending, &mut counters)?;
        }
    }
    Ok(counters)
}

/// Encode `state`; queue it when its key is new.
fn visit(
    system: &ModelSystem<'_>,
    state: ModelSimState,
    seen: &mut BTreeSet<Vec<u8>>,
    pending: &mut VecDeque<ModelSimState>,
    counters: &mut Counters,
) -> Result<(), AxisError> {
    let (bytes, _) = system.state_key(&state).map_err(fail)?;
    counters.canonical_encodes += 1;
    if seen.insert(bytes) {
        counters.states += 1;
        pending.push_back(state);
    }
    Ok(())
}
