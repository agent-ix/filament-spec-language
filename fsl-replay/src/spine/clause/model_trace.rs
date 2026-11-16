// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330 `TraceInput::Model`: a clause run over a model-arm behaviour.
//!
//! The behaviour is an initial-state snapshot, the universes (the snapshots
//! whose complete populations are the simulated populations), and the steps
//! of its prefix and loop. The run admits the initial state, re-executes the
//! steps over the model system the package and universes define, closes and
//! fairness-checks the loop, and evaluates the clause over the reconstructed
//! states on the one evaluator a replayed counterexample uses (FR-329
//! `Lasso::Model`).

use std::collections::{BTreeMap, BTreeSet};

use filament_checked::key::DeclarationKey;
use filament_exact::{ObjectId, ObjectReference, ValueType};
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_eval::temporal::TemporalPosition;
use fsl_foundation::witness::{ObservationIdentity, RuntimeValuePath, ValuePathSubject};
use fsl_semantics::check::TemporalClauseNode;
use fsl_semantics::model::observation::population_universe_for;
use serde::Deserialize;

use super::temporal::{observed_disposition, refused, OverObject, TraceInput};
use super::{ClauseDisposition, ClauseRunUsage, CompiledRun};
use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::{CounterexampleKind, Lasso, TemporalCounterexample};
use crate::model_check::replay::evaluate_model_trace;
use crate::spine::TemporalVerdict;

/// The per-expansion candidate bound of a model-arm run: the request carries
/// none, so it is the one a model check defaults to.
const EXPANSION: ExpansionLimits = ExpansionLimits {
    max_candidates: 1_000_000,
};

/// What a model-arm run reports besides its disposition.
pub(super) struct ModelRun {
    pub(super) disposition: ClauseDisposition,
    pub(super) usage: ClauseRunUsage,
    pub(super) decisive: Option<crate::TemporalDecisive>,
}

/// The snapshot members the universes are read from.
#[derive(Deserialize)]
struct SnapshotRead {
    model: ModelRead,
    #[serde(default)]
    populations: Vec<PopulationRead>,
}

#[derive(Deserialize)]
struct ModelRead {
    identity: String,
}

#[derive(Deserialize)]
struct PopulationRead {
    population: String,
    #[serde(default)]
    complete: bool,
    #[serde(default)]
    objects: Vec<ObjectRead>,
}

#[derive(Deserialize)]
struct ObjectRead {
    key: String,
}

/// FSpec FR-520's universes: one per complete population the universe
/// snapshots declare, its keys the union of their objects' keys in reference
/// order.
fn universes(run: &CompiledRun<'_>, documents: &[crate::DocumentRef]) -> Vec<PopulationUniverse> {
    let mut found: BTreeMap<DeclarationKey, (Vec<String>, BTreeSet<String>)> = BTreeMap::new();
    for bytes in documents
        .iter()
        .filter_map(|reference| run.provisions.snapshots.get(reference))
    {
        let Ok(read) = serde_json::from_slice::<SnapshotRead>(bytes) else {
            continue;
        };
        for population in read
            .populations
            .iter()
            .filter(|population| population.complete)
        {
            let (keys, seen) = found
                .entry(DeclarationKey {
                    package: read.model.identity.clone(),
                    node: population.population.clone(),
                })
                .or_default();
            for object in &population.objects {
                if seen.insert(object.key.clone()) {
                    keys.push(object.key.clone());
                }
            }
        }
    }
    found
        .into_iter()
        .map(|(population, (keys, _))| PopulationUniverse { population, keys })
        .collect()
}

/// FR-330 "Admit" and "Evaluate" for `TraceInput::Model`.
pub(super) fn run_model(
    run: &CompiledRun<'_>,
    node: &TemporalClauseNode,
    trace: &TraceInput,
    over: Option<&OverObject>,
) -> ModelRun {
    let TraceInput::Model {
        initial,
        universes: universe_documents,
        prefix,
        repeat,
        terminal_stutter,
    } = trace
    else {
        return fault("model-run-has-a-model-trace");
    };
    let stopped = |disposition| ModelRun {
        disposition,
        usage: ClauseRunUsage::default(),
        decisive: None,
    };
    let Some(over) = over else {
        return stopped(refused(
            "invalid-value",
            BTreeMap::from([("member", "over".to_owned())]),
        ));
    };
    let clause = node.clause();
    let (_, over_type) = clause.over();
    let ValueType::Reference(effective) = over_type else {
        return stopped(refused("invalid-value", BTreeMap::new()));
    };
    let request = ModelSimulationRequest {
        package: run.package,
        packages: run.packages,
        normalization: run.model_limits,
        snapshots: run.provisions.snapshots,
        initial: vec![initial.clone()],
        universes: universes(run, universe_documents),
        observation: run.observation_limits,
        expansion: EXPANSION,
        meter: run.accounting,
        emit: fsl_package::EmitLimits::new(u64::MAX),
    };
    let system = match ModelSystem::new(request) {
        Ok(system) => system,
        Err(failure) => return stopped(ClauseDisposition::Admit(failure)),
    };
    let instance = match system.instances(over_type) {
        Ok(instances) => instances
            .into_iter()
            .find(|instance| instance.key == over.key),
        Err(_) => return fault("model-instances-are-listed"),
    };
    let Some(instance) = instance else {
        return stopped(refused(
            "wrong-role-mapping",
            BTreeMap::from([
                ("population", over.population.clone()),
                ("object", over.key.clone()),
            ]),
        ));
    };
    let letters = match system.clause_letters(node) {
        Ok(letters) => letters,
        Err(_) => return fault("model-clause-has-letters"),
    };
    // The loop's steps follow the prefix's; a terminal stutter loop lists no
    // step of its own (ADR-018 SM-4).
    let entry = prefix.len();
    let mut steps = prefix.clone();
    steps.extend(repeat.iter().cloned());
    let lasso = if *terminal_stutter {
        Some(Lasso {
            entry: steps.len(),
            terminal_stutter: true,
        })
    } else if repeat.is_empty() {
        None
    } else {
        Some(Lasso {
            entry,
            terminal_stutter: false,
        })
    };
    let payload = TemporalCounterexample {
        initial_state: 0,
        prefix: steps,
        over_binding: Some(instance.clone()),
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
    };
    let evaluated = match evaluate_model_trace(&system, &payload, clause, &letters, run.accounting)
    {
        Ok(Some(evaluated)) => evaluated,
        Ok(None) => {
            return stopped(ClauseDisposition::Temporal(TemporalVerdict::Pending));
        }
        Err(refusal) => return stopped(replay_refusal(refusal)),
    };
    let (disposition, _) =
        observed_disposition(None, evaluated.outcome, run.package.graph(), run.sources);
    let decisive = match (&disposition, evaluated.decisive) {
        (ClauseDisposition::Temporal(TemporalVerdict::Violated { .. }), Some(Ok(Some(atom)))) => {
            Some(decisive_record(
                run,
                &system,
                &instance,
                effective,
                atom.position,
            ))
        }
        _ => None,
    };
    ModelRun {
        disposition,
        usage: ClauseRunUsage::default(),
        decisive: decisive.flatten(),
    }
}

/// The decisive record of a violated model-arm run: the atom is `false` at
/// the position, read over the state the steps reconstructed. A model state
/// has no observation document, so the path is rooted at the over object
/// under the position's state.
fn decisive_record(
    run: &CompiledRun<'_>,
    _system: &ModelSystem<'_>,
    instance: &ModelInstance,
    effective: &filament_exact::EffectiveId,
    position: TemporalPosition,
) -> Option<crate::TemporalDecisive> {
    let universe = population_universe_for(
        run.package.graph().model_inputs(),
        run.packages,
        run.model_limits,
        effective.clone(),
    )
    .ok()?;
    let reference = ObjectReference::new(
        universe,
        effective.clone(),
        ObjectId::new(&instance.key).ok()?,
    );
    Some(
        crate::TemporalDecisive::undecided(RuntimeValuePath {
            observation: ObservationIdentity {
                authority: "model-state".to_owned(),
                identity: format!("position-{}", position.index()),
            },
            subject: ValuePathSubject::Object(reference),
            steps: Vec::new(),
        })
        .at_position(position.index()),
    )
}

fn fault(invariant: &'static str) -> ModelRun {
    ModelRun {
        disposition: ClauseDisposition::EvaluateFault(
            filament_semantic_value::catalog::InternalFault::new("temporal", invariant),
        ),
        usage: ClauseRunUsage::default(),
        decisive: None,
    }
}

/// The disposition of a refused model-arm re-execution: the subject's own
/// admission failure, or `invalid_runtime_input`/`invalid-value` naming why.
fn replay_refusal(refusal: ReplayRefusal) -> ClauseDisposition {
    match refusal {
        ReplayRefusal::Admission(failure) => ClauseDisposition::Admit(failure),
        other => refused(
            "invalid-value",
            BTreeMap::from([("reason", other.to_string())]),
        ),
    }
}
