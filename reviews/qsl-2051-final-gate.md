# QSL-2051: final catalog-source component gate

Reviewed and tested source head: `d71733b414ac6caf0cac66a7b84c4b25ba0cbc6d`.

Public PR: [#3198](https://github.com/agent-ix/filament-spec-language/pull/3198).

Public gate result: [comment 7042966002](https://github.com/agent-ix/filament-spec-language/pull/3198#issuecomment-7042966002).

Independent reviews SR-1534 and SR-1535 passed at that head; their artifact validations and individual marker YAML parses exited 0. Posted review readbacks: `a1eea58c` and `68b01d83`. The single final affected-component gate exited **0**, with **43 passed, 0 failed, 7 ignored**. The passing tests comprise 41 source cases and two runner controls. Test execution took 0.70 seconds.

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2051/target \
cargo test --locked --offline -p fsl-conformance --test it catalog_coverage:: -- \
  --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Receipt: `/home/peter/dev/.worktrees/luna-qsl-2051-final-catalog.log`.

`df -h /` showed 210G available before the gate. The worktree's existing target directory was reused. All owned processes exited.

The seven ignored tests retain strict desired admission assertions and do not count as passing coverage:

- Five composite Float64 equality regressions: IR-1320, with its STD-654 gate.
- Sequence indexing: IR-1491.
- Complete catalog qualification: QSL-2051, pending the remaining source adapters and node families.

QSL-2051 remains partial. The original authoring failures and failed-only correction receipts remain recorded in the PR body. This records-only update preserves the reviewed and tested source head above.
