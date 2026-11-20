# QSL-2092 authoring validation

All ten new active functions passed through focused commands. No aggregate gate
has run; one independent review and one final affected-component gate remain
with the lead. No passing function was rerun after an unrelated failure.

Each command used the worktree's single target directory, jobs2, dev/test
debug0, incremental0, `cargo test --locked -p fsl-analyze --test it`, one test
thread and both prescribed user skips:

```text
--skip two_protocols_declaring_one_channel_name_keep_their_queues_apart
--skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

| Filter | Actual result | Receipt |
| --- | --- | --- |
| `weak_memory::replay_boundary_cases::` | 1 passed, 2 failed; exit101 | `weak-memory-focused.log/.exit` |
| Same filter, skipping the already-passed RA function | 2 passed; exit0 | `weak-memory-corrected.log/.exit` |
| `protocol_system::replay_boundary_cases::` | 3 passed; exit0 | `protocol-focused.log/.exit` |
| `zone_replay::replay_boundary_cases::` | 3 passed, 1 failed; exit101 | `timed-focused.log/.exit` |
| Exact corrected legal-fractional-trace function | 1 passed; exit0 | `timed-corrected.log/.exit` |

Original failures are preserved. The two initial weak-memory failures were an
authoring assumption about the witness length: the trace also contains its
fork. The corrected tests require a nonempty witness and retain all intended
buffer and per-component assertions. The initial fractional-timeout fixture
tried to pass its own client deadline. Its replacement executes a legal reply
at time 3/2 before timeout and asserts that a nonviolating trace is not a
refutation. Only the changed failed functions reran.

The first owned target compile finished in 9m50s. The final compiled integration
artifact is `target/debug/deps/it-5333e4e3dd3e085d`; a later direct executable
gate must retain Cargo's required runtime environment. Disk before the initial
build had 209 GiB free. Leaf rustfmt check and changed-file whitespace check
both exited0.

Known unchanged semantics, package, evaluator, command and integration-helper
warnings remain visible in the logs. The lead accepted the existing lint
limitations; no lint retry, suppression or green Clippy claim is made. No
matrix, product binary, proof/Kani, benchmark or main-refresh check ran.

Full commands, original logs and captured shell exits are retained outside the
repository at `/home/peter/dev/.agent/qsl-2092/logs/`. All authoring processes
exited before handoff. The clean worktree and warm target remain for the lead's
single reviewed gate and subsequent cleanup.
