# Population admission budget final gate

PR #3184; partial runtime-resource slice of QSL-2097.

Independent reviews SR-1517 and SR-1518 passed with no findings. Their supplied artifacts are committed unchanged alongside this receipt.

Tested head: `a5295485a1d74463a7f765d9ab9339e312521e6c`.

Free disk before the gate: 211G.

```sh
env CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2097/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p fsl-semantics --features test-support --test it model_population:: -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Actual exit: **0**. Result: **69 passed, 0 failed, 0 ignored, 1643 filtered out**. The complete `model_population::` component ran once after independent review, using the warm owned target.

The two new cases are:

- `model_population::duplicate_records_use_member_capacity_before_a_foreign_tail_is_inspected` — NFR-012-AC-8; duplicate document records consume raw member capacity and a denied tail is not inspected.
- `model_population::subset_scans_charge_the_candidate_width_before_a_semantic_verdict` — NFR-012-AC-9; weighted subset scans stop before an unfunded semantic verdict and complete at exactly twelve work units.

The other 67 passing cases are existing component coverage. No production code changed. Broader historical NFR families and previously assigned pending defects remain unclaimed by this slice.

Author checks previously passed each new qualified case separately. The initial author invocation omitted `test-support` and ran zero tests; that receipt is invalid evidence. Four warnings in unchanged library code were emitted by compilation; no green lint claim is made. No unrelated suite, proof, Kani, matrix or shared binary check was run.

This receipt adds documentation only after the tested head; the reviewed test code remains unchanged.
