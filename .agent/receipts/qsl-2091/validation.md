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
  zero ignored, 2,209 filtered. Compile 10m14s; runtime 1.21s. Review found its
  reduction fixture selected invariants as constraints. Those reduction results
  are not normative constraint coverage; its failure is retracted as QSL-2713
  evidence. The unaffected certificate and exact-probability results remain valid.
- Changed exact name
  `reduction::completion_boundaries::an_initial_boundary_never_becomes_a_deadlock_or_a_safety_stutter`:
  exit 0, one passed. The strict liveness portion is separate and ignored.
- Strengthened exact name
  `reduction::completion_boundaries::the_twentieth_stored_state_completes_the_constrained_cube`:
  exit 0, one passed, including the independent 24-edge assertion.
- Strengthened exact name
  `exact_route::completion_boundaries::thresholds_one_decimal_unit_around_the_exact_probability_remain_distinct`:
  exit 0, one passed, including the explicit checked `exact_only` assertions.

Those subsequent commands selected only actually changed qualified names with
`--exact`. Their invariant-based reduction inputs are superseded below; they
are not counted as constraint coverage. Exact-probability receipts remain valid.

Private original logs and saved exit files are under
`/home/peter/dev/.agent/qsl-2091/logs/`: `focused`,
`initial-boundary-corrected`,
`the_twentieth_stored_state_completes_the_constrained_cube-strengthened`, and
`thresholds_one_decimal_unit_around_the_exact_probability_remain_distinct-strengthened`.

## FND-001 fixture correction

Only the new reduction child declares native `constraint Low` and
`constraint Started`; inherited `count`/`COUNT` and parent fixtures are unchanged.
The real `World::new` route parses and checks the new source under Linked mode.
The following actually changed new qualified names each ran with `--exact`:

- `reduction::completion_boundaries::the_twentieth_stored_state_completes_the_constrained_cube`:
  exit 0, one pass; `actual-constraint-cube.log/.exit`.
- `reduction::completion_boundaries::an_initial_boundary_never_becomes_a_deadlock_or_a_safety_stutter`:
  exit 0, one pass; `actual-an_initial_boundary_never_becomes_a_deadlock_or_a_safety_stutter.log/.exit`.
- `reduction::completion_boundaries::a_false_initial_letter_refutes_before_the_initial_constraint_cuts`:
  exit 0, one pass; `actual-a_false_initial_letter_refutes_before_the_initial_constraint_cuts.log/.exit`.
- `reduction::completion_boundaries::a_liveness_initial_boundary_never_gains_a_stutter_lasso`:
  the changed actual-constraint input ran once with `--exact --ignored`;
  exit 101, zero passed, one failed on `ReducedLiveness`;
  `actual-constraint-liveness.log/.exit`. This first correctly grounded input
  replaces the original withdrawn QSL-2713 evidence. No exploration is claimed.

Five unaffected certificate/exact functions and three corrected actual-constraint
functions have focused passing receipts: eight distinct new active functions.
One strict desired liveness oracle remains ignored and excluded from coverage.
No unchanged certificate/exact test or invalid old reduction fixture was rerun
in the fix round. The original nonpassing records remain intact.

Leaf `rustfmt --edition 2021 --check` and `git diff --check` exited 0.
The Rust implementation/testing checklist was applied to the changed tests.
Same-reviewer SR-1497/SR-1498 disposition is PASS after the actual-constraint
fixture correction. The single final affected-component gate passed 60 tests,
with zero failures and one strict QSL-2713 ignored case. Exact commands,
tested head and per-parent exit receipts are in `final-gate.md`.
No author self-approval replaces the lead's independent review.
