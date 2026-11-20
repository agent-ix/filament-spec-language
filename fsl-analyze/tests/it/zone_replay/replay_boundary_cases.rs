// SPDX-License-Identifier: AGPL-3.0-or-later
//! Timed replay preserves exact delays and locates malformed evidence.

use super::*;

fn returned(world: &World) -> TemporalCounterexample {
    violated(claim(world, ZoneCheckLimits::default(), &Cancel::new()))
}

#[trace("FR-237")]
#[test]
fn a_timed_witness_requires_a_delay_on_each_transition() {
    let world = rpc(3);
    let witness = returned(&world);
    assert_eq!(operations(&witness), ["send", "timeout", "reply"]);
    for index in 0..witness.prefix.len() {
        let mut changed = witness.clone();
        changed.prefix[index].delay = None;
        let (refusal, code) = super::refusal(replay(&world, &changed));
        assert_eq!(
            refusal,
            ModelTraceRefusal::MissingDelay { step: Some(index) }
        );
        assert_eq!(code, Code::InvalidRuntimeInput);
    }
}

#[trace("FR-237-AC-1")]
#[test]
fn rationally_equal_delays_reproduce_the_same_exact_trace_position() {
    let world = rpc(3);
    let mut witness = returned(&world);
    assert_eq!(delays(&witness), [whole(0), whole(3), whole(0)]);
    for (step, delay) in witness
        .prefix
        .iter_mut()
        .zip([exact(0, 17), exact(6, 2), exact(0, 31)])
    {
        step.delay = Some(delay);
    }
    assert_eq!(
        replay(&world, &witness).expect("equivalent rational delays reproduce"),
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: Some(3),
        }
    );
}

#[trace("FR-237-AC-2")]
#[test]
fn rational_delays_on_opposite_sides_of_the_guard_name_distinct_refusals() {
    let world = rpc(3);
    let witness = returned(&world);
    let mut before = witness.clone();
    before.prefix[1].delay = Some(exact(2_999_999, 1_000_000));
    let (refusal, code) = super::refusal(replay(&world, &before));
    assert!(
        matches!(refusal, ModelTraceRefusal::GuardFails { step: 1, ref guards, .. } if guards == &["Timeout"]),
        "{refusal:?}"
    );
    assert_eq!(code, Code::InvalidRuntimeInput);
    let mut after = witness;
    after.prefix[1].delay = Some(exact(3_000_001, 1_000_000));
    let (refusal, code) = super::refusal(replay(&world, &after));
    assert_eq!(
        refusal,
        ModelTraceRefusal::DelayNotAdmissible {
            step: Some(1),
            delay: exact(3_000_001, 1_000_000),
            bar: DelayBar::TimeInvariant("ServerDeadline".to_owned()),
        }
    );
    assert_eq!(code, Code::InvalidRuntimeInput);
}

#[trace("FR-237")]
#[test]
fn a_formula_witness_cannot_gain_a_final_delay_even_when_it_is_zero() {
    let world = rpc(3);
    let witness = returned(&world);
    assert_eq!(witness.kind, CounterexampleKind::Formula);
    assert!(witness.final_delay.is_none());
    for delay in [whole(0), exact(1, 2)] {
        let mut changed = witness.clone();
        changed.final_delay = Some(delay);
        let (refusal, code) = super::refusal(replay(&world, &changed));
        assert_eq!(refusal, ModelTraceRefusal::StrayFinalDelay);
        assert_eq!(code, Code::InvalidRuntimeInput);
    }
}
