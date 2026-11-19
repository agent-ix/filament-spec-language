// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2277 (FR-640-AC-12): compile a two-model hyper clause with one
//! event-position clock row per trace, then settle a replayed counterexample.

use filament_contract_model::ClockConfiguration;
use filament_exact::Cancel;
use fsl_eval::hyper::{CheckedHyperLetters, CheckedStepMatch, ParameterBinding};
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::TransitionSystem;
use fsl_package::{emit_checked_with_temporal_selections, TemporalClockSelection};
use fsl_replay::{
    settle_hyper_check, HyperCounterexample, HyperCounterexampleKind, HyperOutcome, HyperReplay,
    HyperStep, HyperTrace, ModelCheckLimits, ModelStep, ModelTraceSettlement, RunStats,
    TerminalValue,
};
use ix_trace_rs::trace;

use crate::vault_world::{key, other_key, World, UNLIMITED};

fn step(system: &ModelSystem<'_>) -> ModelStep {
    let initial = system
        .initial()
        .into_iter()
        .next()
        .expect("an initial state");
    let successor = system
        .successors(&initial)
        .expect("initial state expands")
        .successors
        .into_iter()
        .find(|(transition, _)| {
            transition.operation().contains("/Vault/step\"")
                && transition.arguments()[0].contains("\"value\":\"0\"")
        })
        .expect("step(0) is enabled");
    ModelStep {
        transition: successor.0,
        post_state: system
            .state_key(&successor.1)
            .expect("post-state key")
            .0
            .into(),
        delay: None,
    }
}

fn trace(variable: &str, alias: &str, step: ModelStep) -> HyperTrace {
    HyperTrace {
        variable: variable.to_owned(),
        alias: alias.to_owned(),
        initial: 0,
        steps: vec![HyperStep::Model(step)],
        results: Vec::new(),
        marks: Vec::new(),
    }
}

/// Compile emits independent event-position rows for both model identities;
/// the violation settles only after replay validates the two model traces.
#[trace("TC-2243", "FR-640-AC-12", "FSpec-TC-1680", "FSpec-FR-322-AC-453")]
#[test]
fn a_two_model_event_position_hyper_clause_emits_rows_and_replays_its_verdict() {
    let declaration = "hyper Saves using ep over behaviours (d: V::Vault, e: N::Vault) \
         forall trace s of V forall trace n of N \
         { always holds(d.l @ s != e.l @ n) }";
    let world = World::two_aliases(declaration, &crate::vault_world::SECURE);
    let clause = world.clause();
    let graph = world.package().graph();
    let source = graph.source().clone();
    let selections: Vec<_> = clause
        .quantifiers()
        .iter()
        .map(|quantifier| {
            let mut selection = TemporalClockSelection::new(
                source.clone(),
                clause.span(),
                ClockConfiguration::EventPosition {
                    sequence_authority: match quantifier.variable.as_str() {
                        "s" => "test/vault",
                        "n" => "test/vault-other",
                        other => panic!("unexpected trace variable {other}"),
                    }
                    .into(),
                },
            );
            selection.trace = Some(quantifier.variable.as_str().into());
            selection
        })
        .collect();
    let emitted = emit_checked_with_temporal_selections(
        world.package(),
        &selections,
        fsl_package::EmitLimits::fixture(),
    )
    .expect("the two-model hyper clause compiles");
    let occurrence = clause
        .assembled_occurrence()
        .expect("the clause is assembled");
    let rows = emitted.temporal_binding_rows();
    let s = rows
        .get_for_trace(occurrence.node(), occurrence.origin().ordinal(), "s")
        .expect("trace s has its own clock row");
    let n = rows
        .get_for_trace(occurrence.node(), occurrence.origin().ordinal(), "n")
        .expect("trace n has its own clock row");
    assert_eq!(s.trace.as_deref(), Some("s"));
    assert_eq!(n.trace.as_deref(), Some("n"));
    assert_eq!(s.clock_name, "model-steps");
    assert_eq!(n.clock_name, "model-steps");
    assert_eq!(
        s.clock,
        ClockConfiguration::EventPosition {
            sequence_authority: "test/vault".into()
        }
    );
    assert_eq!(
        n.clock,
        ClockConfiguration::EventPosition {
            sequence_authority: "test/vault-other".into()
        }
    );

    let subject_s = world.subject();
    let subject_n = world.subject_n();
    let system_s = ModelSystem::new(subject_s.simulation).expect("V admits");
    let system_n = ModelSystem::new(subject_n.simulation).expect("N admits");
    let systems = vec![&system_s, &system_n];
    let letters = CheckedHyperLetters::new(world.package(), clause, systems.clone())
        .expect("the clause letters admit both model systems");
    let matcher = CheckedStepMatch::new(world.package(), clause);
    let instance = [
        ParameterBinding {
            parameter: "d".to_owned(),
            instance: ModelInstance {
                population: key("vaults"),
                key: "v".to_owned(),
            },
        },
        ParameterBinding {
            parameter: "e".to_owned(),
            instance: ModelInstance {
                population: other_key("vaults"),
                key: "m".to_owned(),
            },
        },
    ];
    let counterexample = HyperCounterexample {
        kind: HyperCounterexampleKind::Lockstep { loop_entry: 0 },
        instance: vec![("d".to_owned(), "v".to_owned()), ("e".to_owned(), "m".to_owned())],
        traces: vec![
            trace("s", "V", step(&system_s)),
            trace("n", "N", step(&system_n)),
        ],
        trace_position: 0,
    };
    let cancel = Cancel::new();
    let replay = HyperReplay {
        clause,
        systems,
        instance: &instance,
        letters: &letters,
        matcher: &matcher,
        meter: UNLIMITED,
        max_witness_set: ModelCheckLimits::default().max_witness_set,
        cancel: &cancel,
    };
    let replay_result = fsl_replay::replay_hyper_trace_tuple(&replay, &counterexample)
        .expect("the counterexample replays against both model traces");
    assert!(matches!(
        replay_result,
        ModelTraceSettlement::Reproduced { .. }
    ));
    let outcome = HyperOutcome::Violated {
        counterexample: Box::new(counterexample),
        stats: RunStats::default(),
    };
    let record = settle_hyper_check(
        &outcome,
        &ModelCheckLimits::default(),
        &[],
        |counterexample| fsl_replay::replay_hyper_trace_tuple(&replay, counterexample),
    )
    .expect("the replayed run settles");
    assert_eq!(record.value, TerminalValue::Refuted);
}
