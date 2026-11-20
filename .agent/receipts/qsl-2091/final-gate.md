# QSL-2091 final component gate

Reviewed source correction: `d84a5f48b4226f68b12d3741dbe7748db35c3421`.
Latest SR-1497/SR-1498 PASS artifacts committed unchanged at
`ed72e3d92dace2c905265ac2df958dcd5cd75777`, the head tested below.
Same-reviewer disposition comments: `997a9b64` / `50049dc5`.

Free disk before this gate: 207 GiB. The original owned target was reused,
with jobs2, dev/test debug0, incremental0 and one test thread. All actual
processes exited. No main refresh, unrelated component, lint retry, new
review, workspace/proof/Kani gate, product binary or matrix ran.

The single complete affected-component gate ran these three parents once:

| Component filter | Passed | Failed | Ignored | Exit |
| --- | ---: | ---: | ---: | ---: |
| `refinement_certificate::` | 25 | 0 | 0 | 0 |
| `reduction::` | 27 | 0 | 1 | 0 |
| `exact_route::` | 8 | 0 | 0 | 0 |
| Total | 60 | 0 | 1 | 0 |

The eight new active functions are included. The only ignored function is
`reduction::completion_boundaries::a_liveness_initial_boundary_never_gains_a_stutter_lasso`,
strictly pending QSL-2713 and excluded from coverage. Its actual-constraint
source was established in the fix round; no pending failure was reprobed in
this gate. Historical invariant-based constraint inputs remain withdrawn
as evidence for the new required behavior.

Each command, from the worktree root, used:

```sh
CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 \
  CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo test --locked -p fsl-analyze --test it '<component>::' -- \
    --test-threads=1 \
    --skip timed_refinement_certificate:: \
    --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
    --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

The explicit `timed_refinement_certificate::` exclusion prevents Cargo's
substring filter from including that unrelated parent. The three selected
parents do not overlap; each includes its existing cases and new child.
Original commands/logs/exits are retained outside the repo in
`/home/peter/dev/.agent/qsl-2091/logs/final-{refinement_certificate,reduction,exact_route}.log`,
`final-exits.tsv` and `final-tested-head.txt`.

Only data receipts changed after the tested head. Broader QSL-2091 historical
families remain unclaimed; this gate does not establish full ticket completion.
