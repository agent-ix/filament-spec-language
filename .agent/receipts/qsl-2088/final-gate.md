# QSL-2088 final affected-component gate

Reviewed feature code: `0d04807b257efac20dbd1503095727a7c1a8ba9b`. Independent SR-1495/1496 PASS, no findings; both direct validations exited0. The unchanged artifacts were committed at `d0241cd10`. No production, dependency or feature changes followed the builds below. Disk before this gate:208GiB free.

The gate reused the actual owned Cargo-produced artifacts whose code matches the reviewed affected components. The first two artifact paths were retained from the original `--locked` three-package authoring build; the evaluator path is from the corrected final-source build. Builds used the same worktree target, jobs2, debug0 and incremental0. The gate did not rebuild unchanged code or change Cargo feature modes.

| Component | Owned artifact | Selection | Actual exit | Passed | Failed | Ignored |
|---|---|---|---|---:|---:|---:|
| fsl-semantics integration | target/debug/deps/it-984447d185c6990f | timed_model:: | 0 | 28 | 0 | 1 |
| fsl-eval integration | target/debug/deps/it-39415a6a019d3b7e | protocol_slots:: | 101 | 96 | 2 | 0 |
| fsl-analyze integration | target/debug/deps/it-322fa4e17f986720 | monitor_plan:: | 0 | 23 | 0 | 0 |

Every selection was the complete affected parent, with no overlap or zero-test segment. `monitor_plan_vectors::` is another parent and does not match `monitor_plan::`. Every invocation used:

```text
--test-threads=1
--skip two_protocols_declaring_one_channel_name_keep_their_queues_apart
--skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

The original aggregate is147 passed,2 failed,1 inherited ignored. All12 newly authored cases passed in this gate; other passes include inherited parent cases and an embedded carrier test. The ignored measured clock-recursion case remains QSL-2144 and is not coverage.

## Confined failure disposition

One failure was gate setup: `protocol_slots::acceptance::state_compensation::the_evaluation_crates_depend_on_no_io_crate` expects Cargo's runtime `CARGO` variable. Direct-artifact execution had omitted it. Only that failed qualified name was executed again with `CARGO=/home/peter/.cargo/bin/cargo`, `--exact` and the same harness flags. It passed1/failed0/ignored0, exit0. No code changed and no passing component or case was rerun.

The other failure is an actual existing reader-parity defect, QSL-2712 (blocks QSL-2088): `protocol_slots::acceptance::bare_and_explicit_dereference_reads_keep_their_charge_logs_through_the_reader` cannot obtain its emitted package's evaluation view. It receives `UnsupportedView / MissingLocationProjection` for `protocol Refunds capture order initializer`, an expression/deref application, and `/checked_term_data/protocol_locations/25/location/origin` (`rows:1`, `visits:0`, `invalid_package/invalid-value`). Current FR-390-AC-19 requires reader parity, and explicitly disallows treating unsupported as passing parity. The original receipt is retained; no repro, production fix, altered assertion or duplicate bug was introduced. No per-loop-iteration success is inferred.

The final accepted composition is148 distinct passes, one strict owner-pending failure (QSL-2712, excluded from successful coverage), and one inherited ignored test (QSL-2144). This is not a wholly green gate. The original147/2/1 receipt and separate1/0/0 correction remain the evidence. Remaining reader-parity scope stays gated; all12 new behavior tests passed. This receipt makes no claim that every historical ticket criterion or runtime-consumer axis is complete.

Known unchanged semantics/analyze lint limitations were accepted and not retried. Actual compile warnings remain recorded; no green lint claim, suppression, whole workspace/crate sweep, matrix, proof gate, binary build, repeated review, main refresh or confidence rebuild occurred. All actual test processes exited before handoff.
