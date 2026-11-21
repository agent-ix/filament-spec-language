# QSL-2748 focused checks

Implementation head: `718966c9f78684676baf4d14042d34dc06a53597`.

Both focused tests passed, one test per command:

- `cargo test --locked -p fsl-analyze --test it delay_fractions_clip_crossing_steps_and_idle_tails -- --exact statistical_model::delay_fractions_clip_crossing_steps_and_idle_tails --test-threads=1`: exit 0, 1 passed.
- `cargo test --locked -p fsl-analyze --test it probability_from_source::bounded_delay_fraction_from_source_clips_the_first_leaving_step -- --exact --test-threads=1`: exit 0, 1 passed.

The source test initially failed because its assertion expected a string decision instead of the structured decision object. The production result was already measured/accepted with estimate 1/1. After correcting that assertion and adding the exact estimate assertion, the same focused test passed. The original failure receipt was retained.

Touched-file formatting passed. No aggregate gate, whole-suite test, matrix, performance or diagnostic sweep was run. Independent code/Rust review SR-1544 and scoped gap analysis SR-1545 both passed on the implementation head; their artifacts accompany this summary. Review artifacts and this summary are the only subsequent repository changes. No further tests, builds, lint, gates or reviews were run for that artifact commit.
