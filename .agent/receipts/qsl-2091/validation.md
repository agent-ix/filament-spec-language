# QSL-2091 focused validation

One worktree target, jobs 2, dev/test debug 0, incremental 0. Free disk before
the first compile: 209 GiB. All test commands use one thread and both prescribed
skips. No workspace, proof, Kani, mutation, product binary, main-move or matrix
check ran. Known unchanged analyze lint blockers were accepted by the lead;
they were not reproduced, suppressed or represented as green lint.

## Commands and actual receipts

From the worktree root, the common Cargo prefix is:

```sh
CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 \
  CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo test --locked -p fsl-analyze --test it
```

The common harness suffix is:

```text
-- --test-threads=1
--skip two_protocols_declaring_one_channel_name_keep_their_queues_apart
--skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

- Initial filter `completion_boundaries::`: exit 101, seven passed, one failed,
  zero ignored, 2,209 filtered. Compile 10m14s; runtime 1.21s. The failure is the
  reached QSL-2713 liveness admission defect, not a passing coverage claim.
- Changed exact name
  `reduction::completion_boundaries::an_initial_boundary_never_becomes_a_deadlock_or_a_safety_stutter`:
  exit 0, one passed. The strict liveness portion is separate and ignored.
- Strengthened exact name
  `reduction::completion_boundaries::the_twentieth_stored_state_completes_the_constrained_cube`:
  exit 0, one passed, including the independent 24-edge assertion.
- Strengthened exact name
  `exact_route::completion_boundaries::thresholds_one_decimal_unit_around_the_exact_probability_remain_distinct`:
  exit 0, one passed, including the explicit checked `exact_only` assertions.

Each subsequent command selected only that actually changed qualified name
and used `--exact`. Eight distinct new active functions therefore have accepted
focused passing receipts. The two strengthened tests are not counted twice.
The strict new QSL-2713 test is not coverage and was not reprobed.

Private original logs and saved exit files are under
`/home/peter/dev/.agent/qsl-2091/logs/`: `focused`,
`initial-boundary-corrected`,
`the_twentieth_stored_state_completes_the_constrained_cube-strengthened`, and
`thresholds_one_decimal_unit_around_the_exact_probability_remain_distinct-strengthened`.

Leaf `rustfmt --edition 2021 --check` and `git diff --check` exited 0.
The Rust implementation/testing checklist was applied to the changed tests.
Independent review and one final affected-component gate remain pending.
No author self-approval replaces the lead's independent review.
