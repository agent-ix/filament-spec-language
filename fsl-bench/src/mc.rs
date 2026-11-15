// SPDX-License-Identifier: AGPL-3.0-or-later
//! The model-check, sampling and key-encoding axes: the input generators
//! and the one function that runs each axis once.
//!
//! | Axis | Generator | Run | Size knob |
//! | --- | --- | --- | --- |
//! | `mc/state_graph` | [`ring_state_graph`] | [`run_state_graph`] | objects `n`, values `k` (`k^n` states) |
//! | `mc/safety` | [`ring_safety`] | [`run_safety`] | objects `n`, values `k` |
//! | `mc/liveness` | [`ring_liveness`] | [`run_liveness`] | objects `n`, values `k` |
//! | `mc/frame_candidates` | [`wide`] | [`run_frame_candidates`] | fields `m`, range `r` (`(r+1)^m` candidates) |
//! | `mc/observation` | [`observation`] | [`run_observation`] | unrelated records `R` |
//! | `reach/workers` | [`protocol`] | [`run_reach`] | parallel branches `b`, workers |
//! | `key/encode` | [`keys`] | [`run_encode`] | objects `n`, fields `f` |
//! | `en4/sample` | [`service`] | [`run_sample`] | indifference in ten-thousandths |
//!
//! Every generator builds its [`World`] through the layers' public API. A
//! refusal of a generated unit is a defect of the generator and is returned
//! as an [`AxisError`], so each axis's smallest-size test fails before
//! benchmark time. The runs never read the clock: the criterion benches and
//! the probe time them.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::num::NonZeroUsize;

use filament_exact::{Cancel, Meter};
use fsl_analyze::model_check::{
    check_model, check_state_graph, ModelCheckRequest, StateGraphItem, StateGraphRequest,
};
use fsl_analyze::statistical::{check_statistical, StatisticalRequest};
use fsl_eval::model_subject::{
    AtomStop, ModelInstance, ModelLetters, ProtocolSubject, RoleBinding,
};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::protocol::{ProtocolSystem, ResolvedMemory, Sc};
use fsl_eval::simulation::TransitionSystem;
use fsl_eval::temporal::AtomReading;
use fsl_foundation::selection::DefinitionRef;
use fsl_replay::statistical::{StatisticalLimits, StatisticalMethod, StatisticalOutcome};
use fsl_replay::{ModelCheckItem, ModelCheckLimits, ModelCheckOutcome};
use fsl_semantics::check::FormulaNodeId;

type Reads = Result<
    BTreeSet<fsl_semantics::check::footprint::SymbolicLocation>,
    fsl_semantics::check::footprint::FootprintRefusal,
>;

use crate::world::{Model, Operation, World, WorldError, BOOLEAN};

/// Why an axis did not run: the generator and a layer disagree.
#[derive(Debug)]
pub struct AxisError(pub String);

impl std::fmt::Display for AxisError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AxisError {}

impl From<WorldError> for AxisError {
    fn from(error: WorldError) -> Self {
        Self(error.0)
    }
}

fn fail(stage: &str, detail: impl std::fmt::Debug) -> AxisError {
    AxisError(format!("{stage}: {detail:?}"))
}

/// What one run of an axis did, in the engine's own counts. A count an axis
/// has no meaning for is zero.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Work {
    /// Distinct states the engine created.
    pub states: u64,
    /// Edges the engine explored.
    pub edges: u64,
    /// Samples the statistical engine counted.
    pub samples: u64,
    /// Bytes of one encoded state key, for `key/encode`.
    pub key_bytes: u64,
}

/// `value` as the count type the engines report.
fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn ints(name: &str, high: usize) -> Vec<(String, i64, i64)> {
    vec![(name.to_owned(), 0, i64::try_from(high).unwrap_or(i64::MAX))]
}

fn operation(name: &str, modifies: &[&str]) -> Operation {
    Operation {
        name: name.to_owned(),
        parameters: Vec::new(),
        modifies: modifies.iter().map(|field| (*field).to_owned()).collect(),
    }
}

fn keyed(
    prefix: &str,
    count: usize,
    fields: &[(String, i64)],
) -> Vec<(String, Vec<(String, i64)>)> {
    (0..count)
        .map(|index| (format!("{prefix}{index}"), fields.to_vec()))
        .collect()
}

/// A declaration for a unit that declares nothing else.
const NOOP: &str = "function Noop using v(): Boolean pure { true }\n";

const T0: &str = "profile t0 = \"filament.temporal.infinite-trace/v1\";\n";

/// `objects` objects `c0 ..` of one `Cell` type, each holding `v: Int[0,
/// values-1]` that its one operation `step` advances modulo `values`: the
/// reachable graph is the `values^objects` product of independent rings.
/// `declarations` follow the contract; `symmetric` declares the population
/// symmetric.
pub fn ring(objects: usize, values: usize, symmetric: bool, declarations: &str) -> Model {
    let top = values.saturating_sub(1);
    Model {
        package: "bench/ring".to_owned(),
        object: "Cell".to_owned(),
        population: "cells".to_owned(),
        symmetric,
        integers: ints("Level", top),
        fields: vec![("v".to_owned(), "ix://bench/ring/Level".to_owned())],
        operations: vec![operation("step", &["v"])],
        profiles: T0.to_owned(),
        body: format!(
            "post Step using v on M::Cell::step {{ \
             (pre(self.v) < {top} and self.v = pre(self.v) + 1) or \
             (pre(self.v) = {top} and self.v = 0) }}\n{declarations}"
        ),
        objects: keyed("c", objects, &[("v".to_owned(), 0)]),
        unrelated: 0,
    }
}

/// A reader of one integer field of the object under an instance's key: the
/// letters of the model-check axes. It counts every atom it evaluates.
pub struct FieldLetters {
    population: filament_checked::key::DeclarationKey,
    field: filament_checked::key::DeclarationKey,
    holds: fn(i64) -> bool,
    evaluations: Cell<u64>,
}

impl FieldLetters {
    /// Letters reading `field` of `object` in `world` through `holds`.
    pub fn new(world: &World, object: &str, field: &str, holds: fn(i64) -> bool) -> Self {
        Self {
            population: world.population.clone(),
            field: world.key(&format!("{object}/{field}")),
            holds,
            evaluations: Cell::new(0),
        }
    }

    /// The atoms evaluated so far.
    pub fn evaluations(&self) -> u64 {
        self.evaluations.get()
    }
}

fn unobserved() -> AtomStop {
    AtomStop::Fault(filament_semantic_value::catalog::InternalFault::new(
        "fsl-bench",
        "the-letters-read-an-integer-field",
    ))
}

impl ModelLetters for FieldLetters {
    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        self.evaluations.set(self.evaluations.get() + 1);
        let value = state
            .state()
            .populations
            .get(&self.population)
            .and_then(|objects| objects.get(&instance.key))
            .and_then(|object| object.fields.get(&self.field));
        match value {
            Some(filament_exact::Value::Integer(integer)) => {
                let integer: i64 = integer.to_string().parse().map_err(|_| unobserved())?;
                Ok(if (self.holds)(integer) {
                    AtomReading::True
                } else {
                    AtomReading::False
                })
            }
            _ => Err(unobserved()),
        }
    }

    fn activation(
        &self,
        _instance: &ModelInstance,
        _state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Err(unobserved())
    }

    fn activation_reads(&self) -> Reads {
        Ok(BTreeSet::new())
    }

    fn reads(&self, _atom: FormulaNodeId) -> Reads {
        Ok(BTreeSet::new())
    }
}

/// The ring with the state-graph claim `Wraps`: the last value is reachable.
///
/// # Errors
///
/// [`AxisError`] when a layer refuses the generated unit.
pub fn ring_state_graph(
    objects: usize,
    values: usize,
    symmetric: bool,
) -> Result<World, AxisError> {
    let top = values.saturating_sub(1);
    let claim = format!("possible Wraps using v over (c: M::Cell) {{ c.v = {top} }}\n");
    Ok(World::build(&ring(objects, values, symmetric, &claim))?)
}

/// `mc/state_graph`: FR-167 `check_state_graph` over the ring, with a
/// symmetry group over every object when `symmetric`. The phase 0 walks are
/// off, so the run is the exploration.
///
/// # Errors
///
/// [`AxisError`] when the run does not complete the graph.
pub fn run_state_graph(world: &World, symmetric: bool) -> Result<Work, AxisError> {
    let claim = world
        .package
        .graph()
        .state_graph_claims()
        .iter()
        .find(|claim| claim.name() == "Wraps")
        .ok_or_else(|| fail("claim", "Wraps is not checked"))?;
    let mut reductions = fsl_route::reductions::Reductions::default();
    if symmetric {
        let keys: Vec<String> = world
            .universes
            .iter()
            .flat_map(|universe| universe.keys.clone())
            .collect();
        reductions
            .symmetry
            .push(fsl_route::reductions::SymmetryDeclaration {
                population: world.population.clone(),
                classes: vec![keys],
            });
    }
    let limits = ModelCheckLimits {
        witness_samples: 0,
        ..ModelCheckLimits::default()
    };
    let run = check_state_graph(
        StateGraphRequest {
            subject: world.subject(),
            temporal_clock_selections: Vec::new(),
            items: vec![StateGraphItem::new(claim)],
            reductions,
            deadlock_freedom: false,
            max_depth: None,
            limits,
            seed: None,
        },
        &Cancel::new(),
    )
    .map_err(|refusal| fail("check_state_graph", refusal))?;
    if !run.graph.open.is_empty() {
        return Err(fail(
            "check_state_graph",
            "the graph was not fully expanded",
        ));
    }
    Ok(Work {
        states: run.graph.stats.nodes,
        edges: run.graph.stats.edges,
        ..Work::default()
    })
}

/// The ring with the safety clause `Bounded` over `c`.
///
/// # Errors
///
/// [`AxisError`] when a layer refuses the generated unit.
pub fn ring_safety(objects: usize, values: usize) -> Result<World, AxisError> {
    let top = values.saturating_sub(1);
    let clause = format!(
        "temporal Bounded using t0 over (c: M::Cell) clock \"model-steps\" on origin \
         {{ always holds(c.v <= {top}) }}\n"
    );
    Ok(World::build(&ring(objects, values, false, &clause))?)
}

/// What a `check_model` run returns, as counts and the atoms evaluated.
fn model_work(outcome: &ModelCheckOutcome, atoms: u64) -> Result<(Work, u64), AxisError> {
    match outcome {
        ModelCheckOutcome::Holds { stats, .. } => Ok((
            Work {
                states: stats.product_states,
                edges: stats.product_edges,
                ..Work::default()
            },
            atoms,
        )),
        other => Err(fail("check_model", other)),
    }
}

fn check_item(world: &World, item: ModelCheckItem<'_>) -> Result<ModelCheckOutcome, AxisError> {
    check_model(
        ModelCheckRequest {
            temporal_clock_selections: &[],
            subject: world.subject(),
            item,
            max_depth: None,
            limits: ModelCheckLimits::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
    .map_err(|refusal| fail("check_model", refusal))
}

fn first_clause(world: &World) -> Result<&fsl_semantics::check::CheckedTemporalClause, AxisError> {
    world
        .package
        .graph()
        .temporal_clauses()
        .first()
        .map(|node| node.clause())
        .ok_or_else(|| fail("clause", "the unit declares no temporal clause"))
}

/// `mc/safety`: `check_model` of the invariant clause `Bounded`, or, with
/// `deadlock`, of the deadlock-freedom item, over the ring. Returns the work
/// and the atoms evaluated.
///
/// # Errors
///
/// [`AxisError`] when the item is not decided `Holds`.
pub fn run_safety(world: &World, deadlock: bool) -> Result<(Work, u64), AxisError> {
    if deadlock {
        let outcome = check_item(world, ModelCheckItem::DeadlockFreedom)?;
        return model_work(&outcome, 0);
    }
    let clause = first_clause(world)?;
    let letters = FieldLetters::new(world, "Cell", "v", |value| value >= 0);
    let outcome = check_item(
        world,
        ModelCheckItem::Clause {
            clause,
            letters: &letters,
        },
    )?;
    model_work(&outcome, letters.evaluations())
}

/// The ring with the weak-fair liveness clause `Returns`.
///
/// # Errors
///
/// [`AxisError`] when a layer refuses the generated unit.
pub fn ring_liveness(objects: usize, values: usize) -> Result<World, AxisError> {
    let clause = "temporal Returns using t0 over (c: M::Cell) clock \"model-steps\" on origin \
         { fair weak each M::Cell::step; always eventually holds(c.v = 0) }\n";
    Ok(World::build(&ring(objects, values, false, clause))?)
}

/// `mc/liveness`: `check_model` of `always eventually` under weak fairness
/// over the ring. Returns the work and the atoms evaluated.
///
/// # Errors
///
/// [`AxisError`] when the clause is not decided `Holds`.
pub fn run_liveness(world: &World) -> Result<(Work, u64), AxisError> {
    let clause = first_clause(world)?;
    let letters = FieldLetters::new(world, "Cell", "v", |value| value == 0);
    let outcome = check_item(
        world,
        ModelCheckItem::Clause {
            clause,
            letters: &letters,
        },
    )?;
    model_work(&outcome, letters.evaluations())
}

/// One object whose operation `scramble` writes `fields` fields `f0 ..` each
/// over `Int[0, range]`, with no contract: every expansion generates
/// `(range+1)^fields` candidates and all of them are successors.
///
/// # Errors
///
/// [`AxisError`] when a layer refuses the generated unit.
pub fn wide(fields: usize, range: usize) -> Result<World, AxisError> {
    let names: Vec<String> = (0..fields).map(|index| format!("f{index}")).collect();
    let zeros: Vec<(String, i64)> = names.iter().map(|name| (name.clone(), 0)).collect();
    let model = Model {
        package: "bench/wide".to_owned(),
        object: "Cell".to_owned(),
        population: "cells".to_owned(),
        symmetric: false,
        integers: ints("Digit", range),
        fields: names
            .iter()
            .map(|name| (name.clone(), "ix://bench/wide/Digit".to_owned()))
            .collect(),
        operations: vec![Operation {
            name: "scramble".to_owned(),
            parameters: Vec::new(),
            modifies: names,
        }],
        profiles: String::new(),
        body: NOOP.to_owned(),
        objects: keyed("c", 1, &zeros),
        unrelated: 0,
    };
    Ok(World::build(&model)?)
}

/// `mc/frame_candidates`: deadlock-freedom over [`wide`].
///
/// # Errors
///
/// [`AxisError`] when the item is not decided `Holds`.
pub fn run_frame_candidates(world: &World) -> Result<Work, AxisError> {
    let outcome = check_item(world, ModelCheckItem::DeadlockFreedom)?;
    model_work(&outcome, 0).map(|(work, _)| work)
}

/// A two-state ring (two values, one object) in a domain package that also
/// declares `unrelated` object types with a population each, none read by
/// the model: the state count is fixed and only the package grows.
///
/// # Errors
///
/// [`AxisError`] when a layer refuses the generated unit.
pub fn observation(unrelated: usize) -> Result<World, AxisError> {
    let mut model = ring(1, 2, false, "");
    model.package = "bench/observation".to_owned();
    model.fields = vec![("v".to_owned(), "ix://bench/observation/Level".to_owned())];
    model.unrelated = unrelated;
    Ok(World::build(&model)?)
}

/// `mc/observation`: deadlock-freedom over [`observation`], which builds the
/// system and observes the snapshot inside the run.
///
/// # Errors
///
/// [`AxisError`] when the item is not decided `Holds`.
pub fn run_observation(world: &World) -> Result<Work, AxisError> {
    run_frame_candidates(world)
}

/// `branches` parallel attempts of `inc` on one `Cell` whose `v` counts to
/// `branches`, joined `all`: the protocol's reachable states are the
/// interleavings of the branches.
///
/// # Errors
///
/// [`AxisError`] when a layer refuses the generated unit.
pub fn protocol(branches: usize) -> Result<World, AxisError> {
    let attempt = |index: usize| {
        format!(
            "branch b{index} attempt A{index} by w on M::Cell::inc contracts [IncPre, IncPost] \
             as (a{index}: Boolean) {{ true }};"
        )
    };
    let names: Vec<String> = (0..branches).map(|index| format!("b{index}")).collect();
    let branch_text: String = (0..branches).map(attempt).collect::<Vec<_>>().join(" ");
    let body = format!(
        "pre IncPre using v on M::Cell::inc {{ self.v < {branches} }}\n\
         post IncPost using v on M::Cell::inc {{ self.v - pre(self.v) = 1 }}\n\
         protocol P using pp over (k: M::Cell) on origin {{\n\
         role w on M::Cell;\n\
         run sequence Main {{ parallel Both {{ {branch_text} }} join all [{}]; }}\n\
         finish Done as (d: Boolean) {{ d }};\n}}\n",
        names.join(", ")
    );
    let model = Model {
        package: "bench/protocol".to_owned(),
        object: "Cell".to_owned(),
        population: "cells".to_owned(),
        symmetric: false,
        integers: ints("Level", branches),
        fields: vec![("v".to_owned(), "ix://bench/protocol/Level".to_owned())],
        operations: vec![operation("inc", &["v"])],
        profiles: "profile pp = \"filament.protocol.finite-global/v1\";\n".to_owned(),
        body,
        objects: keyed("c", 1, &[("v".to_owned(), 0)]),
        unrelated: 0,
    };
    Ok(World::build(&model)?)
}

/// `reach/workers`: every reachable state of protocol `P` under `Sc`, each
/// breadth-first level expanded on up to `workers` threads.
///
/// # Errors
///
/// [`AxisError`] when the protocol system does not build or the search
/// refuses.
pub fn run_reach(world: &World, workers: NonZeroUsize) -> Result<Work, AxisError> {
    let checked = world
        .package
        .graph()
        .protocol("P")
        .ok_or_else(|| fail("protocol", "P is not checked"))?;
    let memory = ResolvedMemory::resolve(checked, &[]).map_err(|e| fail("memory", e))?;
    let subject = ProtocolSubject {
        model: world.subject(),
        protocol: "P".to_owned(),
        over: ModelInstance {
            population: world.population.clone(),
            key: "c0".to_owned(),
        },
        roles: vec![RoleBinding {
            role: "w".to_owned(),
            model: "bench/protocol".to_owned(),
        }],
        memory,
    };
    let model = ModelSystem::new(subject.model.simulation.clone())
        .map_err(|failure| fail("ModelSystem::new", failure))?;
    let system = ProtocolSystem::new(model, &subject, Sc)
        .map_err(|refusal| fail("ProtocolSystem::new", refusal))?;
    let reached = system
        .reachable(workers)
        .map_err(|refusal| fail("reachable", refusal))?;
    let edges: usize = reached.states.iter().map(|state| state.steps.len()).sum();
    Ok(Work {
        states: count(reached.states.len()),
        edges: count(edges),
        ..Work::default()
    })
}

/// `objects` objects `c0 ..` each of `fields` integer fields `f0 ..` over
/// `Int[0, 9]`, and no operation: the initial state is the one state keyed.
///
/// # Errors
///
/// [`AxisError`] when a layer refuses the generated unit.
pub fn keys(objects: usize, fields: usize) -> Result<World, AxisError> {
    let names: Vec<String> = (0..fields).map(|index| format!("f{index}")).collect();
    let values: Vec<(String, i64)> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), i64::try_from(index % 10).unwrap_or(0)))
        .collect();
    let model = Model {
        package: "bench/keys".to_owned(),
        object: "Cell".to_owned(),
        population: "cells".to_owned(),
        symmetric: false,
        integers: ints("Digit", 9),
        fields: names
            .iter()
            .map(|name| (name.clone(), "ix://bench/keys/Digit".to_owned()))
            .collect(),
        operations: Vec::new(),
        profiles: String::new(),
        body: NOOP.to_owned(),
        objects: keyed("c", objects, &values),
        unrelated: 0,
    };
    Ok(World::build(&model)?)
}

/// `key/encode`: the canonical state key (`ModelSystem::state_key`) of the
/// initial state, once.
///
/// # Errors
///
/// [`AxisError`] when the subject has no initial state or the key has no
/// encoding.
pub fn run_encode(system: &ModelSystem<'_>) -> Result<Work, AxisError> {
    let initial = system.initial();
    let state = initial
        .first()
        .ok_or_else(|| fail("initial", "the subject has no initial state"))?;
    let (bytes, _) = system
        .state_key(state)
        .map_err(|refusal| fail("state_key", refusal))?;
    Ok(Work {
        states: 1,
        key_bytes: count(bytes.len()),
        ..Work::default()
    })
}

/// The `Service` model of the statistical engine's own tests: one server
/// that is requested, attempted with a random duration and outcome, and
/// reset, with the claim `P95` of the duration from Busy to Done within ten
/// steps at indifference `epsilon_ten_thousandths / 10000`.
///
/// # Errors
///
/// [`AxisError`] when a layer refuses the generated unit.
pub fn service(epsilon_ten_thousandths: u32) -> Result<World, AxisError> {
    const PACKAGE: &str = "bench/service";
    let int = |name: &str| format!("ix://{PACKAGE}/{name}");
    let epsilon = format!(
        "{}.{:04}",
        epsilon_ten_thousandths / 10_000,
        epsilon_ten_thousandths % 10_000
    );
    let body = format!(
        "pre Idle using v on M::Server::request {{ self.phase = 0 }}\n\
         post Started using v on M::Server::request {{ self.phase = 1 and self.attempts = 0 }}\n\
         pre Busy using v on M::Server::attempt {{ self.phase = 1 }}\n\
         post Attempted using v on M::Server::attempt {{ \
            self.attempts - pre(self.attempts) = 1 and \
            ((ok and self.phase = 2) or \
             (not ok and pre(self.attempts) = 1 and self.phase = 2) or \
             (not ok and pre(self.attempts) = 0 and self.phase = 1)) }}\n\
         pre Done using v on M::Server::reset {{ self.phase = 2 }}\n\
         post Reset using v on M::Server::reset {{ self.phase = 0 }}\n\
         random Server::attempt(d) ~ {{ 1: 90, 3: 7, 12: 3 }};\n\
         random Server::attempt(ok) ~ {{ true: 98, false: 2 }};\n\
         reward duration on Server::attempt = d;\n\
         workload Steady on M {{ weight Server::request = 1; weight Server::attempt = 1; \
            weight Server::reset = 1; }}\n\
         probabilistic P95 using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference {epsilon} \
         {{ quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
         until holds(s.phase = 2) within 10) <= 5 }}\n"
    );
    let model = Model {
        package: PACKAGE.to_owned(),
        object: "Server".to_owned(),
        population: "servers".to_owned(),
        symmetric: false,
        integers: vec![
            ("Phase".to_owned(), 0, 2),
            ("Attempts".to_owned(), 0, 2),
            ("Millis".to_owned(), 1, 12),
        ],
        fields: vec![
            ("phase".to_owned(), int("Phase")),
            ("attempts".to_owned(), int("Attempts")),
        ],
        operations: vec![
            operation("request", &["phase", "attempts"]),
            Operation {
                name: "attempt".to_owned(),
                parameters: vec![
                    ("d".to_owned(), int("Millis")),
                    ("ok".to_owned(), BOOLEAN.to_owned()),
                ],
                modifies: vec!["phase".to_owned(), "attempts".to_owned()],
            },
            operation("reset", &["phase"]),
        ],
        profiles: "profile ev = \"filament.temporal.event-position.false-extension/v1\";\n\
                   profile inf = \"filament.temporal.infinite-trace/v1\";\n"
            .to_owned(),
        body,
        objects: vec![(
            "s".to_owned(),
            vec![("phase".to_owned(), 0), ("attempts".to_owned(), 0)],
        )],
        unrelated: 0,
    };
    Ok(World::build(&model)?)
}

/// `en4/sample`: `check_statistical` of `P95` by the Okamoto method. The
/// work's `samples` is the counted samples.
///
/// # Errors
///
/// [`AxisError`] when the claim does not complete.
pub fn run_sample(world: &World) -> Result<Work, AxisError> {
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("P95")
        .ok_or_else(|| fail("claim", "P95 is not checked"))?;
    let sampler = DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1")
        .map_err(|e| fail("sampler", e))?;
    let outcome = check_statistical(
        StatisticalRequest {
            subject: world.subject(),
            claim,
            method: StatisticalMethod::Okamoto,
            seed: 7,
            sampler,
            limits: StatisticalLimits::default(),
            max_witnesses: 0,
            symmetry: Vec::new(),
            timed_sampling: fsl_replay::statistical::TimedSampling::default(),
        },
        &Cancel::new(),
    )
    .map_err(|refusal| fail("check_statistical", refusal))?;
    match outcome {
        StatisticalOutcome::Completed(verdict) => Ok(Work {
            samples: verdict.basis.samples,
            ..Work::default()
        }),
        other => Err(fail("check_statistical", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn work(states: u64, edges: u64) -> Work {
        Work {
            states,
            edges,
            ..Work::default()
        }
    }

    #[test]
    fn state_graph_smallest_size_explores_the_ring_product() {
        let world = ring_state_graph(2, 2, false).expect("the ring builds");
        assert_eq!(run_state_graph(&world, false).expect("runs"), work(4, 8));
    }

    /// The run collapses the claim's per-key instances to one orbit
    /// representative and quotients by the subgroup that fixes it
    /// (`common_stabiliser` in `state_graph.rs`), so three interchangeable
    /// rings of three values give `k * C(k + n - 2, n - 1)` = 3 * 6 = 18 states,
    /// not the 10 orbits of the full group; two objects fix one and reduce
    /// nothing. `ModelSystem::canonicalise` under the full group reaches 10.
    #[test]
    fn state_graph_symmetry_quotients_the_ring_product() {
        let world = ring_state_graph(3, 3, true).expect("the ring builds");
        assert_eq!(run_state_graph(&world, false).expect("runs"), work(27, 81));
        assert_eq!(run_state_graph(&world, true).expect("runs"), work(18, 54));
    }

    #[test]
    fn safety_smallest_size_holds_over_the_ring_product() {
        let world = ring_safety(2, 2).expect("the ring builds");
        assert_eq!(run_safety(&world, false).expect("runs"), (work(4, 8), 18));
        assert_eq!(run_safety(&world, true).expect("runs"), (work(4, 8), 0));
    }

    #[test]
    fn liveness_smallest_size_holds_under_weak_fairness() {
        let world = ring_liveness(2, 2).expect("the ring builds");
        assert_eq!(run_liveness(&world).expect("runs"), (work(12, 28), 8));
    }

    #[test]
    fn frame_candidates_smallest_size_generates_every_assignment() {
        // Two fields over Int[0, 1]: four candidate states, each the
        // successor of every state.
        let world = wide(2, 1).expect("the model builds");
        assert_eq!(run_frame_candidates(&world).expect("runs"), work(4, 16));
    }

    #[test]
    fn observation_smallest_size_keeps_the_state_count_fixed() {
        for records in [1, 8] {
            let world = observation(records).expect("the model builds");
            assert_eq!(run_observation(&world).expect("runs"), work(2, 2));
        }
    }

    #[test]
    fn reach_smallest_size_is_the_same_graph_at_every_worker_count() {
        let world = protocol(2).expect("the protocol builds");
        for workers in [1, 2, 4] {
            let limit = NonZeroUsize::new(workers).expect("nonzero");
            assert_eq!(run_reach(&world, limit).expect("runs"), work(7, 7));
        }
    }

    #[test]
    fn encode_smallest_size_keys_one_state() {
        let world = keys(1, 1).expect("the state builds");
        let system = ModelSystem::new(world.simulation()).expect("the subject admits");
        assert_eq!(
            run_encode(&system).expect("runs"),
            Work {
                states: 1,
                key_bytes: 845,
                ..Work::default()
            }
        );
        let larger = keys(2, 1).expect("the state builds");
        let system = ModelSystem::new(larger.simulation()).expect("the subject admits");
        assert!(run_encode(&system).expect("runs").key_bytes > 845);
    }

    #[test]
    fn sample_smallest_size_draws_the_okamoto_sample_count() {
        let world = service(400).expect("the model builds");
        let drawn = run_sample(&world).expect("runs").samples;
        assert_eq!(drawn, 1_440);
    }
}

#[cfg(all(test, feature = "counters"))]
mod counter_tests {
    use super::tests::work;
    use super::*;
    use crate::counters::{fingerprint, Counters};

    fn counters(states: u64, edges: u64, encodes: u64, atoms: u64, expansions: u64) -> Counters {
        Counters {
            states,
            edges,
            generated_candidates: edges,
            canonical_encodes: encodes,
            atom_evaluations: atoms,
            expansions,
        }
    }

    #[test]
    fn smallest_size_counter_totals_are_fixed() {
        let ring = ring_state_graph(2, 2, false).expect("builds");
        assert_eq!(
            fingerprint(&ring, 0).expect("counts"),
            counters(4, 8, 9, 0, 4)
        );
        let symmetric = ring_state_graph(3, 3, true).expect("builds");
        assert_eq!(
            Counters::of_work(run_state_graph(&symmetric, true).expect("runs")),
            Counters::of_work(work(18, 54))
        );
        let safety = ring_safety(2, 2).expect("builds");
        let (_, atoms) = run_safety(&safety, false).expect("runs");
        assert_eq!(
            fingerprint(&safety, atoms).expect("counts"),
            counters(4, 8, 9, 18, 4)
        );
        let liveness = ring_liveness(2, 2).expect("builds");
        let (_, atoms) = run_liveness(&liveness).expect("runs");
        assert_eq!(
            fingerprint(&liveness, atoms).expect("counts"),
            counters(4, 8, 9, 8, 4)
        );
        let wide = wide(2, 1).expect("builds");
        assert_eq!(
            fingerprint(&wide, 0).expect("counts"),
            counters(4, 16, 17, 0, 4)
        );
        let observation = observation(1).expect("builds");
        assert_eq!(
            fingerprint(&observation, 0).expect("counts"),
            counters(2, 2, 3, 0, 2)
        );
        let protocol = protocol(2).expect("builds");
        let reached = run_reach(&protocol, NonZeroUsize::MIN).expect("runs");
        assert_eq!(Counters::of_work(reached), Counters::of_work(work(7, 7)));
        let keyed = keys(1, 1).expect("builds");
        assert_eq!(
            fingerprint(&keyed, 0).expect("counts"),
            counters(1, 0, 1, 0, 1)
        );
        let service = service(400).expect("builds");
        assert_eq!(
            fingerprint(&service, 0).expect("counts"),
            counters(7, 17, 18, 0, 7)
        );
    }
}
