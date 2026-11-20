# QSL-2094 final terminal-component gate

Independent SR-1503 code/Rust and SR-1504 scoped gap review passed without findings; both artifact validations exited 0. The exact supplied artifacts were committed unchanged in tested head `e66e09730504a1b226ce51d474c1a8a7b679e4f4`. Reviewed feature source remains `e10ecdc27f37b183f391f539a019108d7ccfde88`.

Before the gate, `/` had 210 GiB available. One existing owned target was reused. The gate ran once over the complete affected `emit::tests::terminal::` parent, including all existing and new children; the qualified filter includes no unrelated protocol-terminal module.

```sh
CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2094/target \
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_INCREMENTAL=0 cargo test --locked -p fsl-package --lib \
  emit::tests::terminal:: -- --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Actual result: **exit 0; 11 passed, 0 failed, 1 ignored; 1014 filtered out**. The 11 passes comprise six new active functions and five existing parent cases. The single ignored case is the strict QSL-2718 desired model-root identity assertion, excluded from coverage and not reprobed. There was no zero-test segment or overlapping filter. Cargo supplied its normal runtime environment; the unchanged warm test executable needed no feature-mode rebuild (`Finished test profile` in 0.24s).

The actual private receipt and captured shell exit are `/home/peter/dev/.agent/qsl-2094/logs/final-terminal.log` and `final-terminal.exit`. All gate processes exited. Known unchanged semantics/package warnings are retained; no green lint claim or known lint retry is made. No workspace, unrelated crate, proof, Kani, matrix, binary, additional review, or main-refresh gate ran.

Only receipt/body data changes follow the tested head. This slice leaves QSL-2718 and the parent's broader historical families open; it does not claim full QSL-2094 completion.
