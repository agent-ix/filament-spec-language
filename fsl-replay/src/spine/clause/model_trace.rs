// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330 `TraceInput::Model`: a clause run over a model-arm behaviour.
//!
//! The behaviour is an initial-state snapshot, the universes (the snapshots
//! whose complete populations are the simulated populations), and the steps
//! of its prefix and loop. The run admits the initial state and the
//! universes, re-executes the steps over the model system the package and
//! universes define, closes and fairness-checks the loop, and evaluates the
//! clause over the reconstructed states on the one evaluator a replayed
//! counterexample uses (FR-329 `Lasso::Model`).

use std::collections::{BTreeMap, BTreeSet};

use filament_checked::key::DeclarationKey;
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_eval::simulation::{ExpansionLimit, ExpansionStop};
use fsl_eval::temporal::DecisiveStop;
use fsl_eval::value::ObservationIdentity;
use fsl_semantics::check::{FormulaNode, TemporalClauseNode};
use fsl_semantics::model::observation::{admit_trace_snapshot, AdmissionUsage};
use serde::Deserialize;

use super::temporal::{decisive_over, observed_disposition, refused, OverObject, TraceInput};
use super::trace_refusal::ClauseSite;
use super::{ClauseDisposition, ClauseRunUsage, CompiledRun};
use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::{CounterexampleKind, Lasso, TemporalCounterexample};
use crate::model_check::limits::{meter_bound, ModelCheckLimit, ReachedLimit};
use crate::model_check::replay::{evaluate_model_trace, ModelTraceEnd, ModelTraceRun};
use crate::spine::TemporalVerdict;

/// The per-expansion candidate bound of a model-arm run. No requirement
/// specifies it: FR-255 gives `expansion.max_candidates` no default and
/// FR-641 gives the run request no member to set it (W5 spec ruling
/// pending), so the run fixes it here. A stop at the bound is reported as
/// `ExpansionLimitReached`, never as a verdict.
const MAX_CANDIDATES: u64 = 1_000_000;

/// The expansion bound of a model-arm run.
const EXPANSION: ExpansionLimits = ExpansionLimits {
    max_candidates: MAX_CANDIDATES,
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
/// order. `None` when an admitted snapshot is not there to read.
fn universes(
    run: &CompiledRun<'_>,
    documents: &[crate::DocumentRef],
) -> Option<Vec<PopulationUniverse>> {
    let mut found: BTreeMap<DeclarationKey, (Vec<String>, BTreeSet<String>)> = BTreeMap::new();
    for reference in documents {
        let bytes = run.provisions.snapshots.get(reference)?;
        let read = serde_json::from_slice::<SnapshotRead>(bytes).ok()?;
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
    Some(
        found
            .into_iter()
            .map(|(population, (keys, _))| PopulationUniverse { population, keys })
            .collect(),
    )
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
    let mut admitted = AdmissionUsage::default();
    let stopped = |disposition, admitted: AdmissionUsage| ModelRun {
        disposition,
        usage: ClauseRunUsage {
            admission_consumed: admitted,
            ..ClauseRunUsage::default()
        },
        decisive: None,
    };
    let Some(over) = over else {
        return stopped(
            refused(
                "invalid-value",
                BTreeMap::from([("member", "over".to_owned())]),
            ),
            admitted,
        );
    };
    let clause = node.clause();
    let (_, over_type) = clause.over();
    // FR-645: a model behaviour whose loop holds no step is refused, as a
    // lasso with an empty loop is (FR-329-AC-3).
    if repeat.is_empty() && !*terminal_stutter {
        let site = ClauseSite {
            package: run.package,
            node,
        };
        return stopped(
            ClauseDisposition::TraceRefused(Box::new(site.empty_loop())),
            admitted,
        );
    }
    // FR-330: the universe snapshots are observations of the trace, admitted
    // by FR-106 before the behaviour is simulated over their populations.
    let graph = run.package.graph();
    for document in universe_documents {
        match admit_trace_snapshot(
            graph.model_inputs(),
            graph.scope().types(),
            run.packages,
            run.model_limits,
            run.provisions.snapshots,
            document,
            run.observation_limits,
            &[],
        ) {
            Ok((_, usage)) => {
                admitted.document_bytes =
                    admitted.document_bytes.saturating_add(usage.document_bytes);
                admitted.objects = admitted.objects.saturating_add(usage.objects);
                admitted.values = admitted.values.saturating_add(usage.values);
            }
            Err(failure) => return stopped(ClauseDisposition::Admit(failure), admitted),
        }
    }
    let Some(universes) = universes(run, universe_documents) else {
        return fault("admitted-universe-snapshot-reads");
    };
    let request = ModelSimulationRequest {
        package: run.package,
        packages: run.packages,
        normalization: run.model_limits,
        snapshots: run.provisions.snapshots,
        initial: vec![initial.clone()],
        universes,
        observation: run.observation_limits,
        expansion: EXPANSION,
        meter: run.accounting,
        emit: fsl_package::EmitLimits::new(u64::MAX),
    };
    let system = match ModelSystem::new(request) {
        Ok(system) => system,
        Err(failure) => return stopped(ClauseDisposition::Admit(failure), admitted),
    };
    let instance = match system.instances(over_type) {
        Ok(instances) => instances
            .into_iter()
            .find(|instance| instance.key == over.key),
        Err(_) => return fault("model-instances-are-listed"),
    };
    let Some(instance) = instance else {
        return stopped(
            refused(
                "wrong-role-mapping",
                BTreeMap::from([
                    ("population", over.population.clone()),
                    ("object", over.key.clone()),
                ]),
            ),
            admitted,
        );
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
    let lasso = Lasso {
        entry: if *terminal_stutter {
            steps.len()
        } else {
            entry
        },
        terminal_stutter: *terminal_stutter,
    };
    let payload = TemporalCounterexample {
        initial_state: 0,
        prefix: steps,
        over_binding: Some(instance.clone()),
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso: Some(lasso),
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
    };
    let evaluated = match evaluate_model_trace(&system, &payload, clause, &letters, run.accounting)
    {
        Ok(ModelTraceEnd::Evaluated(evaluated)) => *evaluated,
        Ok(ModelTraceEnd::Stopped(stop)) => {
            return stopped(expansion_stop(&stop, run.accounting), admitted)
        }
        Err(refusal) => return stopped(replay_refusal(refusal), admitted),
    };
    let ModelTraceRun {
        outcome,
        decisive,
        observation,
        mut meter,
        ..
    } = evaluated;
    let (disposition, _) = observed_disposition(None, outcome, run.package.graph(), run.sources);
    let violated = matches!(
        disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Violated { .. })
    );
    let mut decisive_record = None;
    let mut disposition = disposition;
    if violated {
        match decisive {
            Some(Ok(Some(atom))) => {
                let holds = matches!(
                    clause.formula().nodes().get(atom.atom.index()),
                    Some(FormulaNode::Holds { .. })
                );
                if let (true, Some(observed)) = (holds, observation) {
                    match decisive_over(
                        run,
                        node,
                        atom,
                        &observed.over,
                        &observed.environment,
                        ObservationIdentity {
                            authority: observed.identity.authority,
                            identity: observed.identity.identity,
                        },
                        &mut meter,
                    ) {
                        Ok(record) => decisive_record = record,
                        Err(stopped_at) => disposition = *stopped_at,
                    }
                }
            }
            Some(Err(DecisiveStop::Incomplete(incomplete))) => {
                disposition = ClauseDisposition::Temporal(TemporalVerdict::Incomplete(incomplete));
            }
            Some(Err(DecisiveStop::Fault(internal))) => {
                disposition = ClauseDisposition::EvaluateFault(internal);
            }
            Some(Ok(None)) | None => {}
        }
    }
    ModelRun {
        disposition,
        usage: ClauseRunUsage::from_meter(&meter, admitted),
        decisive: decisive_record,
    }
}

/// The disposition of an expansion that stopped before the behaviour was
/// replayed (FR-203, FR-126): the bound it reached, or the `accounting.*`
/// counter, and its value in the run. A stop that names no bound is a fault,
/// never a completed run.
fn expansion_stop(
    stop: &ExpansionStop,
    accounting: filament_exact::ScalarLimits,
) -> ClauseDisposition {
    let reached = match stop.limit {
        Some(ExpansionLimit::MaxCandidates) => ReachedLimit {
            limit: ModelCheckLimit::MaxCandidates,
            value: MAX_CANDIDATES,
        },
        Some(ExpansionLimit::EvaluationMeter(counter)) => ReachedLimit {
            limit: ModelCheckLimit::EvaluationMeter { counter },
            value: meter_bound(&accounting, counter),
        },
        None => {
            return ClauseDisposition::EvaluateFault(
                filament_semantic_value::catalog::InternalFault::new(
                    "temporal",
                    "model-expansion-stops-at-a-named-limit",
                ),
            )
        }
    };
    ClauseDisposition::Temporal(TemporalVerdict::ExpansionLimitReached(reached))
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

#[cfg(test)]
mod tests {
    use super::*;
    use filament_semantic_value::catalog::CatalogCode;

    /// FR-255: the candidate bound is reported as `max_candidates`, raised by
    /// `expansion.max_candidates`, with the value the run ran under.
    #[test]
    fn a_stop_at_the_candidate_bound_names_max_candidates() {
        let stop = ExpansionStop {
            cause: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
            limit: Some(ExpansionLimit::MaxCandidates),
        };
        let ClauseDisposition::Temporal(TemporalVerdict::ExpansionLimitReached(reached)) =
            expansion_stop(&stop, crate::AccountingLimits::default().0)
        else {
            panic!("a stop at the candidate bound is a limit");
        };
        assert_eq!(reached.limit.as_str(), "max_candidates");
        assert_eq!(reached.limit.setting(), "expansion.max_candidates");
        assert_eq!(reached.value, MAX_CANDIDATES);
    }

    /// A stop that names no bound is a fault, never a completed run.
    #[test]
    fn a_stop_with_no_named_limit_is_a_fault() {
        let stop = ExpansionStop {
            cause: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
            limit: None,
        };
        assert!(matches!(
            expansion_stop(&stop, crate::AccountingLimits::default().0),
            ClauseDisposition::EvaluateFault(_)
        ));
    }
}
