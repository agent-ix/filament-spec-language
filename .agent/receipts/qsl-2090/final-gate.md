# QSL-2090 final affected-component gate

Independent SR-1499 code/Rust and SR-1500 scoped gap reviews passed with no
findings. Their latest artifacts were copied unchanged into `reviews/` at
`2835b14096555c07168b10b3e4e6c7eae51ca85f`, the head tested below. Reviewed
feature code is `7ebffebe932cbf9a111b5156d1a912e5a9fe07e6`; subsequent changes
are review artifacts and validation receipts only.

The two complete affected components ran once, through Cargo with its actual
runtime environment. Every child case in each component was selected. The
groups do not overlap. Both commands used this worktree's warm `target/`,
jobs 2, dev/test debug 0, incremental 0, one thread, and the two prescribed
skips. Disk before the gate had 207 GiB available.

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2090/target cargo test --locked --features checked-handoff --test it lowering_intake_boundaries:: -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2090/target cargo test --locked -p fsl-analyze --test it source_model_boundaries:: -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

| Complete affected component | Passed | Failed | Pending ignored | Exit |
|---|---:|---:|---:|---:|
| Root `lowering_intake_boundaries::` | 19 | 0 | 1 | 0 |
| Analysis `source_model_boundaries::` | 1 | 0 | 3 | 0 |
| Total | 20 | 0 | 4 | 0 |

All 20 passing functions are new. The one QSL-2709 widening case and three
QSL-2714 bounded-operator cases remain strict, pending, and excluded from
passing coverage. They were not reprobed. Original authoring failures and
their corrections remain in the authoring receipts. No full-workspace gate,
lint retry, proof gate, matrix, shared-binary build, main refresh, artificial
runtime environment, or extra review was run.

Private actual logs and individual exits are
`/home/peter/dev/.agent/qsl-2090/logs/final-{root,analyze}.{log,exit}`;
`final-exits.tsv` records both exits and `final-gate-head.txt` the tested head.
All actual processes exited before this receipt was finalized.
