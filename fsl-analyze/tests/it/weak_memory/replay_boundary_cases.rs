// SPDX-License-Identifier: AGPL-3.0-or-later
//! Replay recomputes weak-memory evidence under the caller's bounds.

use super::*;

fn returned(
    case: &Case,
    selection: &[MemorySelection],
) -> TemporalCounterexample<ProtocolTransition> {
    match case.claim(selection, None) {
        ModelCheckOutcome::Violated { counterexample, .. } => *counterexample,
        other => panic!("store buffering must expose both-zero: {other:?}"),
    }
}

fn replay_bounded(
    case: &Case,
    selection: &[MemorySelection],
    payload: TemporalCounterexample<ProtocolTransition>,
    store_buffer: Option<u64>,
    messages: Option<u64>,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let unit = unit_text_with(case.profiles, &case.body).into_bytes();
    let node = &case.checked.graph().temporal_clauses()[0];
    let clause = node.identity().clone();
    let occurrence = OccurrenceKey::new(clause.clone(), node.claim().clone());
    let mut subject = case.trace_subject(selection);
    subject.max_store_buffer = store_buffer;
    subject.max_messages = messages;
    replay_protocol_trace(
        request(
            &case.checked,
            &[],
            &document(),
            case.documents.snapshots.iter(),
            &unit,
        ),
        &subject,
        &envelope_at(payload, clause, occurrence),
        case.letters,
        ReplayLimits::default(),
    )
}

fn assert_refutation(result: ModelTraceReplayResult) {
    assert!(
        matches!(
            result.settlement,
            ModelTraceSettlement::Reproduced {
                value: fsl_replay::ModelTraceValue::Truth(false),
                ..
            }
        ),
        "{result:?}"
    );
}

#[trace("FR-227-AC-5")]
#[test]
fn one_store_per_thread_replays_at_the_exact_tso_buffer_bound() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let selection = [select("SB::Both", MemoryModelName::Tso)];
    let witness = returned(&case, &selection);
    assert_eq!(witness.prefix.len(), 4);
    for bound in [None, Some(1), Some(2)] {
        assert_refutation(
            replay_bounded(&case, &selection, witness.clone(), bound, None)
                .unwrap_or_else(|error| panic!("buffer {bound:?}: {error:?}")),
        );
    }
}

#[trace("FR-227-AC-5", "FR-225-AC-3")]
#[test]
fn ra_replay_counts_the_initial_message_against_its_bound() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let selection = [select("SB::Both", MemoryModelName::Ra)];
    let witness = returned(&case, &selection);
    for bound in [None, Some(2)] {
        assert_refutation(
            replay_bounded(&case, &selection, witness.clone(), None, bound)
                .unwrap_or_else(|error| panic!("messages {bound:?}: {error:?}")),
        );
    }
    let error = replay_bounded(&case, &selection, witness, None, Some(1))
        .expect_err("the first store would add a second message");
    assert_eq!(error.code().as_str(), "invalid_runtime_input", "{error:?}");
    let ReplayRefusal::ModelTrace(refusal) = error else {
        panic!("the replay must name its invalid trace");
    };
    assert_eq!(refusal.cause(), "invalid-value", "{refusal:?}");
}

#[trace("FR-227-AC-1")]
#[test]
fn every_weak_step_requires_its_own_carried_memory_component() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let selection = [select("SB::Both", MemoryModelName::Tso)];
    let witness = returned(&case, &selection);
    assert_eq!(witness.prefix.len(), 4);
    for index in 0..witness.prefix.len() {
        assert!(witness.prefix[index]
            .transition
            .memory_component()
            .is_some());
        let mut missing = witness.clone();
        missing.prefix[index].transition = missing.prefix[index]
            .transition
            .clone()
            .carrying_memory(None);
        let error = case
            .replay(&selection, missing)
            .expect_err("a component is required");
        let ReplayRefusal::ModelTrace(refusal) = error else {
            panic!("missing component {index}: {error:?}");
        };
        assert!(
            matches!(&*refusal, ModelTraceRefusal::MemoryComponent { step, recorded: None, replayed: Some(_) } if *step == index),
            "component {index}: {refusal:?}"
        );
        assert_eq!(refusal.code().as_str(), "stale_dependency");
        assert_eq!(refusal.cause(), "content-mismatch");
    }
}
