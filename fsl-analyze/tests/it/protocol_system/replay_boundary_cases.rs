// SPDX-License-Identifier: AGPL-3.0-or-later
//! Protocol replay checks the complete blocked record and subject inputs.

use super::*;

#[trace("FR-217-AC-1", "FR-217-AC-3")]
#[test]
fn replay_rederives_each_blocked_thread_location_and_reason() {
    let case = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let outcome = case.deadlock("Fill");
    let witness = violated(&outcome);
    assert_eq!(witness.blocked, fill_blocked());
    assert_eq!(witness.blocked.len(), 2);
    let mut changed = Vec::new();
    for index in 0..witness.blocked.len() {
        let mut missing = witness.clone();
        missing.blocked.remove(index);
        changed.push(missing);
        let mut location = witness.clone();
        location.blocked[index].rest_point.node = Some("Main/Other".to_owned());
        changed.push(location);
        let mut reason = witness.clone();
        reason.blocked[index].cause = WaitCause::Precondition {
            clause: "OtherPrecondition".to_owned(),
        };
        changed.push(reason);
    }
    for (index, changed) in changed.into_iter().enumerate() {
        let result = case
            .replay("Fill", changed)
            .expect("the steps remain executable");
        assert!(
            matches!(result.settlement, ModelTraceSettlement::Inconclusive(_)),
            "changed blocked field {index}: {result:?}"
        );
    }
    let original = case
        .replay("Fill", witness)
        .expect("the original record replays");
    assert!(matches!(
        original.settlement,
        ModelTraceSettlement::Reproduced { .. }
    ));
}

#[trace("FR-217")]
#[test]
fn an_untimed_protocol_refuses_delay_on_each_step_and_at_the_endpoint() {
    let case = Case::new(&fill_unit("self.v = 0", ""), &[0]);
    let witness = violated(&case.deadlock("Fill"));
    assert_eq!(witness.prefix.len(), 2);
    for index in 0..witness.prefix.len() {
        let mut changed = witness.clone();
        changed.prefix[index].delay = Some(crate::zone_search::exact(0, 1));
        let error = case
            .replay("Fill", changed)
            .expect_err("untimed delays must not be ignored");
        let ReplayRefusal::ModelTrace(refusal) = error else {
            panic!("delay at {index}: {error:?}");
        };
        assert!(
            matches!(*refusal, ModelTraceRefusal::UntimedDelay { step: Some(step) } if step == index),
            "{refusal:?}"
        );
        assert_eq!(refusal.code().as_str(), "invalid_runtime_input");
        assert_eq!(refusal.cause(), "invalid-value");
    }
    let mut changed = witness;
    changed.final_delay = Some(crate::zone_search::exact(0, 1));
    let error = case
        .replay("Fill", changed)
        .expect_err("an untimed endpoint has no delay");
    let ReplayRefusal::ModelTrace(refusal) = error else {
        panic!("endpoint delay: {error:?}");
    };
    assert_eq!(*refusal, ModelTraceRefusal::UntimedDelay { step: None });
}

#[trace("FR-217-AC-1")]
#[test]
fn changing_the_initial_snapshot_invalidates_the_first_recorded_protocol_state() {
    let source = fill_unit("self.v = 0", "");
    let first = Case::new(&source, &[0]);
    let witness = violated(&first.deadlock("Fill"));
    let changed = Case::new(&source, &[1]);
    let error = changed
        .replay("Fill", witness)
        .expect_err("the first post-state binds its snapshot");
    let ReplayRefusal::ModelTrace(refusal) = error else {
        panic!("snapshot mismatch: {error:?}");
    };
    assert!(
        matches!(*refusal, ModelTraceRefusal::PostState { step: 0, .. }),
        "{refusal:?}"
    );
    assert_eq!(refusal.code().as_str(), "stale_dependency");
    assert_eq!(refusal.cause(), "content-mismatch");
}
