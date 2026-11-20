# QSL-2052 final affected-component gate

Reviewed and tested source: `fa2abc9f6ff2f147a76295266ca6d88e644c7ca9`.
PR: https://github.com/agent-ix/filament-spec-language/pull/3197.

The one final qualification-source component gate exited **0**:
**37 passed, 0 failed, 0 ignored**, 1480 unrelated tests filtered out,
0.19 seconds of test execution. The worktree had 210G free before the gate.

```sh
env -u FSL_FULL_SOURCE_QUALIFICATION \
  CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2052/target \
  CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 \
  CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo test --locked --offline -p fsl-eval --test it \
  qualification_vectors_from_source:: -- --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Actual final log: `/home/peter/dev/.worktrees/luna-qsl-2052-final-gate.log`.
Original compile failure, adapter failures, failed-only repairs and the
full-qualification negative control remain in the sibling focused logs.
The expected-negative full-qualification run was not repeated for this gate.

The tests sample source outcomes from **35 of 171 published rows**.
**136 rows have no adapter**, and **21 sampled rows have additional authored
obligations not qualified here**. Deliberately wire/backend-only inputs are
not relabelled as missing FSL features. Full qualification remains incomplete;
QSL-2052 has no Done or full-acceptance claim.

SR-1532 and SR-1533 independently passed at the tested source head. Their
artifact validations and individual marker YAML parses exited 0; tracker
readbacks are `c006d587` and `0611eeca`.

The earlier scoped Clippy run exited 101 on **12 unchanged fsl-eval library
errors**, before reaching the new test target. That limitation remains;
no lint retry, suppression or unrelated fix was made. Exact owned-file
formatting and whitespace checks passed during authoring.

The following commit adds only this receipt and the independently validated
review records. It preserves the reviewed and tested source unchanged.
