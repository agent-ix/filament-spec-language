# QSL-2093 focused validation

Worktree: `/home/peter/dev/.worktrees/luna-qsl-2093`.
Branch: `codex/qsl-2093-run-lifecycle-certificate-cases`.
Base: `1c9b02db9`.
One target: worktree `target/`.

Every Cargo command used `--locked --features checked-handoff --test it`, `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0` and the owned `CARGO_TARGET_DIR`. Tests used one thread and both required skips:

```text
--skip two_protocols_declaring_one_channel_name_keep_their_queues_apart
--skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Private logs and separately captured shell exits are in `/home/peter/dev/.agent/qsl-2093/logs/`. Free disk before the initial build was 209 GiB.

## Actual execution

| Receipt | Selection | Exit and result |
| --- | --- | --- |
| `focused-lifecycle.log/.exit` | `lifecycle_boundaries::`, excluding the large-declaration case | 101: five new tests passed, two new tests failed on authoring assumptions |
| `setting-corrected.log/.exit` | exact `lifecycle_boundaries::changing_one_compile_setting_preserves_other_bounds_and_emit_rejects_source_settings` | 0: one passed |
| `emission-corrected.log/.exit` | exact `lifecycle_boundaries::package_output_ceiling_is_inclusive_and_never_returns_a_truncated_artifact` | 0: one passed |
| `deadline-large.log/.exit` | exact `lifecycle_boundaries::a_deadline_from_another_thread_stops_two_hundred_thousand_declarations_at_the_next_charge` | 101: one setup failure at `s1.tokens`, before checking or cancellation |
| `deadline-provisioned.log/.exit` | same exact large-declaration test with its actual source provision | 101: the real checker returned `Cancelled(Deadline)` but observed 200,666 charges after Deadline was requested at charge 1,000 |

Seven distinct active functions have passing receipts. The original five passes were not rerun. Only the two changed failed active tests reran, each once. The large next-charge assertion is strict and ignored under QSL-2720; it has no passing result and is not coverage. No repeat of its reached implementation failure ran.

## Confined authoring corrections

The first setting case asked every field to retain `u64::MAX`. FR-506 explicitly makes `model.sequence_maximum` a `u32` whose largest bound is `u32::MAX`; this was a wrong test input, not a filed bug. The corrected test checks each field at zero, seven and its published prior bound, plus that exact maximum for the collection setting.

The first emission case assumed an unsupported top-level `format` member after the actual byte-limit and whole-byte equality assertions had already passed. The corrected test inspects the real emitted semantic graph, without inventing a wire member or probing an unrelated strict-reader feature.

The first large-declaration attempt supplied too few lexer tokens. Its corrected input provision derives source-byte, token and CST-node ceilings from the actual source size. No production hard limit, native stack size or parser rule changed. That corrected run reached the intended S3 cancellation assertion and exposed QSL-2720. The original setup failure is retained and is not evidence for that bug.

## Checks and remaining steps

`rustfmt --edition 2021 --check tests/it/lifecycle_boundaries.rs` exited 0. `git diff --check` exited 0. No unrelated module was formatted. Known unchanged root integration lint limitations are accepted by the lead; no lint retry or green Clippy claim is made.

No workspace gate, product binary, matrix, proof/Kani gate, baseline measurement, peer-result recheck or passing-case confidence rerun ran. Independent review and one complete affected `lifecycle_boundaries::` gate remain with the lead. Broader historical QSL-2093 families are unclaimed by this slice.
