# QSL-2092 final affected-component gate

SR-1501 code/Rust and SR-1502 scoped gap review passed with no findings. Their
validated artifacts were committed unchanged at `defe1125a`; reviewed feature
head was `583d0de02eed477339b887c7e84db972d1a0953d`.

The single final gate ran all existing and new tests in the three affected
integration parents, with nonoverlapping qualified filters. The warm target
was reused; disk before the gate had 209 GiB free. Cargo launched the test
executable with its required runtime environment. Each command used jobs2,
dev/test debug0, incremental0 and one test thread.

```sh
CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2092/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p fsl-analyze --test it weak_memory:: -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated

CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2092/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p fsl-analyze --test it protocol_system:: -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated

CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2092/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p fsl-analyze --test it zone_replay:: -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

| Original segment | Passed | Failed | Ignored | Exit | Receipt |
| --- | ---: | ---: | ---: | ---: | --- |
| `weak_memory::` | 46 | 1 | 0 | 101 | `final-weak-memory.log/.exit` |
| `protocol_system::` | 35 | 2 | 0 | 101 | `final-protocol.log/.exit` |
| `zone_replay::` | 26 | 0 | 0 | 0 | `final-timed.log/.exit` |

The original gate is **107 passed, 3 failed, 0 ignored**. All ten new tests
passed in those segments. The original failures and exits remain preserved.

## Confined fixture corrections

Two existing protocol tests expected runtime Undefined from bare remainder
expressions in Linked code, so S3 refused them before their assertions. Their
fixtures now call `q(x) = 6 mod x`, admitted through the existing
`fill::package_with_mode` Kernel-function seam; the protocol clauses retain
their Linked checking semantics. Numerator, divisor expressions, all outcome
assertions and the separate exhaustion fixture remain unchanged. The change
is confined to test fixture code at `998cf6688`.

Only the two failed qualified names reran:

```sh
CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2092/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p fsl-analyze --test it protocol_system::an_undefined_contract_or_constraint_ends_an_undefined_evaluation -- --exact --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated

CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2092/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p fsl-analyze --test it protocol_system::an_undefined_or_exhausted_predicate_is_never_read_as_false -- --exact --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Each ran **1 passed, 0 failed, 0 ignored, exit0**. Receipts are
`final-contract-corrected.log/.exit` and `final-predicate-corrected.log/.exit`.
No passing parent or name reran.

## Remaining strict failure

[QSL-2717](https://linear.app/agent-ix/issue/QSL-2717) owns the existing
`weak_memory::multi_location_read_modify_write_is_locked_under_tso_and_ra`
failure. Current FR-902-AC-2 requires the two-location locked access shape.
The first TSO iteration instead refuses in independent strict package
consumption with `InvalidSemanticGraph`, pointer `/semantic_graph/nodes/85`,
checked node `SB/run/Both/L/Bx`. No model exploration or RA iteration was
reached. No emitter-versus-reader root cause is assigned.

One complete bounded owning-project lookup found no prior owner; the actual
bug was filed in the package API project and blocks QSL-2092. Its original
assertion is unchanged, it was not reprobed, and it is excluded from accepted
coverage. It is distinct from widened-result QSL-2709 and capture-projection
QSL-2712.

The accepted composition is **109 passing tests, 1 strict pending failure,
0 ignored**. This combines the original passing segments and two confined
corrections; it is not a claim that the original gate was green or that a
second complete gate ran. The library-core timed seam and broader historical
scope limitations in `scope.md` remain explicit.

No production code, known-failing lint, proof/Kani, matrix, product binary,
main-refresh or repeated review gate ran. Every test and Cargo process exited.
Full commands, logs and shell exits remain at
`/home/peter/dev/.agent/qsl-2092/logs/`.
