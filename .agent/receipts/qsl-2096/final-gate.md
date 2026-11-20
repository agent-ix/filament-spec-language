# QSL-2096 canonical-types gate receipt

Reviewed feature head: `246f1bc591daf951f62aeb7dc128326b8ca5133b`.
Gate head: `7ee14a013` (same feature plus unchanged independent PASS artifacts SR-1509 and SR-1510).

The one final affected component gate passed after independent review: exit 0,
9 passed, 0 failed, 0 ignored, 234 filtered out. All nine tests selected by
`canonical_types::` ran, including the existing shared-leaf Copy contracts and
the new Evaluation regression. Runtime was 23.81 seconds. Disk before the gate:
210 GiB available on `/`.

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2096/target cargo test --locked -p xtask --lib canonical_types:: -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Full local output: `/home/peter/dev/.worktrees/luna-qsl-2096-final-gate.log`.
Cargo supplied its test runtime environment; no direct executable was invoked.
Existing dependency dead-code warnings remain warnings. No workspace gate,
whole-xtask suite, lint retry, matrix, qualification proof or main refresh ran.

## Original failure and fix

The initial new test exited 101: `struct Evaluation;` in the workspace produced
no findings instead of an `Identifier` finding at `fsl-command/src/lib.rs:1`
with canonical definition `filament-eval/src/lib.rs:2`. The narrow identifier
condition alone still failed the same assertion because `filament-eval` was
missing from the shared-leaf scan list. Both actual failing receipts remain at
`luna-qsl-2096-red.log` and `luna-qsl-2096-green.log` beside the worktree.

Adding the fourth required shared leaf and its explicitly reserved Evaluation
identifier made the focused regression pass: exit 0, 1 passed, 242 filtered.
It exercises three violating/admitted pairs: a private struct, a nested enum,
and an alias. Exact typed findings and both loci, gate refusal code and exit 1,
then renamed declarations with direct owner re-export and gate acceptance are
asserted. The successful focused receipt is `luna-qsl-2096-green-fixed.log`.
Changed-file rustfmt passed. The initial package fmt check reported unrelated
existing formatting; those files were preserved.

## Scope

QSL-2721 owns this concrete gate defect and blocks QSL-2096. This delivers only
FR-273-AC-1's gate-reporting facet and the explicitly named Evaluation behavior.
It does not claim full FR-273, full QSL-2096, any actual deletion/rename, or
generalized shared-leaf identifier conformance. The broader FR-272 wording
versus the existing TC-747 Copy distinction remains documented and unreconciled.
QSL-2455's facade-accessor work remains outside this change. Independent review
and this component gate are complete; PR opening and real-number review posting
remain with the lead.
