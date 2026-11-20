# QSL-2048: finite-window source gate

PR: https://github.com/agent-ix/filament-spec-language/pull/3192

Tested source: `14ea5cc2def156ce458fc9b0dc2ff584fda27a6f`. Independent code, gap and specification reviews SR-1529, SR-1530 and SR-1531 passed before this gate. Later commits contain review artifacts and this receipt only.

The one final affected-component gate passed:

| Selection | Result | Exit |
| --- | --- | --- |
| `cargo test --locked --offline -p fsl-conformance --test it fspec_finite_window::` | 4 passed, 0 failed, 0 ignored | 0 |
| `cargo test --locked --offline -p xtask --lib conformance::tests::` | 5 passed, 0 failed, 0 ignored | 0 |

Both commands used `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0`, the worktree's own target directory, `--test-threads=1 --nocapture`, and the required skips `two_protocols_declaring_one_channel_name_keep_their_queues_apart` and `a_recovery_predicate_over_the_recovery_view_is_evaluated`. Disk availability before the gate was 204 GiB.

The source runner printed `conformance: 46 finite-window source windows matched`. It checked the standard's independently authored outcomes, activation positions, deadlines, evidence reasons/positions and specified settlement basis/support. Those 46 cases include true, false, pending and indeterminate outcomes. The additional focused source tests cover altered expectations, missing published input and the source-atom/counter-cover adapter seam. The xtask component tests cover required selections, evidence summaries, failed child processes, missing evidence and zero-vector rejection; synthetic Cargo output is routing evidence only.

This is **partial QSL-2048 delivery**: 77 direct finite-window case ids are accounted for, with 46 executed and 31 named as lacking source adapters. State-core's abstract judgments and proposed snapshot/invocation contracts lack the authoritative executable model binding; their semantic expectations remain unqualified. No default success is claimed for either missing scope.

Before this final gate, the explicit `FSPEC_FULL_QUALIFICATION=1` control exited 101 as required, naming all 31 missing adapters before executing any source case. That expected negative result was preserved and was not rerun in the gate.

Direct confined requirement validation passed with `quire validate --scope . spec/functional/FR-350-run-the-conformance-corpus-against-fsl.md`, exit 0. The repository's canonical npm wrapper separately stopped with helper error 127 / Make exit 2 because its Node launcher was absent from PATH; that wrapper is not claimed to have passed. The earlier explicit SpecReview-module invocation was unsuitable for an FR document and is not validation evidence. Scoped Clippy and exact leaf formatting passed during authoring.

Original authoring failures were retained: duplicate facade re-export E0252, an incorrect `ScalarLimits::default()` assumption, and seven fixture-adapter mismatches. The duplicate export was removed without changing the public exports; explicit limits, holds-atom labels and counter-cover interpretation were corrected. Only the affected source examples were rerun during iteration. The final gate above is the first complete post-review execution of the affected components.
