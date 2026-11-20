// SPDX-License-Identifier: AGPL-3.0-or-later
//! Initial-state evidence is independently authenticated before path replay.

use super::*;

#[trace("FR-202-AC-10")]
#[test]
fn a_genuine_terminal_key_and_state_cannot_replace_the_witness_initial_state() {
    let world = World::new(&prob_world::link(CLAIMS));
    let original = evidence(&world, "TooMuch");
    assert_eq!(
        mass(replay(
            &world,
            &original,
            &ExactProbLimits::default(),
            &Cancel::new()
        )),
        ratio(9, 10)
    );
    let terminal = original.paths[0]
        .steps
        .last()
        .expect("a real target-reaching path");
    assert_ne!(terminal.post_key, original.initial_key);
    let mut key_only = original.clone();
    key_only.initial_key = terminal.post_key.clone();
    let mut state_and_key = original.clone();
    state_and_key.initial = terminal.post.clone();
    state_and_key.initial_key = terminal.post_key.clone();
    for changed in [key_only, state_and_key] {
        assert!(
            matches!(
                replay(
                    &world,
                    &changed,
                    &ExactProbLimits::default(),
                    &Cancel::new()
                ),
                Err(ProbabilityWitnessError::Refused)
            ),
            "a valid state from this model still is not the requested initial state"
        );
    }
}
