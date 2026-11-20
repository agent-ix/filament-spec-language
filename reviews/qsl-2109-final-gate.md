# QSL-2109 affected-component gate receipt

The complete affected-component run started at reviewed feature head
`128108371d85b0b500aa170f6bae366c8c3b2c8c`. Only failed names were rerun
after oracle corrections. Passing segments were retained. Full acceptance
remains blocked by QSL-2729: the strict sequence-row regression is pending,
ignored with its real owner, and untagged. Its failure was not converted into
an expected reader refusal.

All commands used the owned `target`, jobs 2, development/test debug 0,
incremental 0 and one test thread. Free space before the gate was 206 GiB.
The only explicit skips were
`two_protocols_declaring_one_channel_name_keep_their_queues_apart` and
`a_recovery_predicate_over_the_recovery_view_is_evaluated`.

| Original component | Passed | Failed | Ignored | Exit | Local receipt |
|---|---:|---:|---:|---:|---|
| `fsl-package --lib emit::tests::admission_corpus::` | 32 | 2 | 1 | 101 | `luna-qsl-2109-final-admission.log` |
| Root `--features checked-handoff --test it checked_handoff::` | 42 | 4 | 0 | 101 | `luna-qsl-2109-final-handoff.log` |
| `xtask --lib ci_feature_lane_tests::` | 7 | 3 | 0 | 101 | `luna-qsl-2109-final-ci-lanes.log` |
| `xtask --lib ignored_reason_tests::` | 3 | 0 | 0 | 0 | `luna-qsl-2109-final-ignore.log` |
| `make seam-probe-capability` | 1 | 0 | 0 | 0 | `luna-qsl-2109-final-seam.log` |

The capability target executed the real ignored compiler test, including its
successful normal build and applicability-match probe failure. No other
ignored behavior case was executed.

At oracle-fix head `237040cc276de832dcf3002873a128f70c592fc8`:

| Failed-only correction | Passed | Failed | Exit | Local receipt |
|---|---:|---:|---:|---|
| Admission classifier and positive population/sequence rows | 1 | 1 | 101 | `luna-qsl-2109-final-admission-fixes.log` |
| Four root handoff cases | 3 | 1 | 101 | `luna-qsl-2109-final-handoff-fixes.log` |
| Three CI-lane cases | 3 | 0 | 0 | `luna-qsl-2109-final-ci-lanes-fixes.log` |

The population row passed its strict read before the sequence row failed at
`/semantic_graph/nodes/17/node_id` with `InvalidPackage` /
`InvalidDerivedShape`. QSL-2729 owns that real bounded-domain identity defect.
The admitted population case was mechanically separated into an active test;
the pending sequence case was not rerun.

At final source head `e14179fc42296e240d77cf0fc020c765e760bf3f`:

| Remaining focused check | Passed | Failed | Exit | Local receipt |
|---|---:|---:|---:|---|
| Independent identity substitutions, exact failed root name | 1 | 0 | 0 | `luna-qsl-2109-final-handoff-identity-fix.log` |
| Ignored-reason repository guard after the new ownership annotation | 1 | 0 | 0 | `luna-qsl-2109-final-ignore-owned.log` |

The identity test retains independent admission, source/domain/result
substitution refusals and noncanonical-byte refusal. Its source substitution
now asserts the strict reader's exact `InvalidPackage` / `InvalidValue`
diagnostic at `/source_map/0`.

Other corrections retain the current name-row check, derive the missing-profile
node index from the admitted fixture, test the promised model-unit target
closure without an unspecified absence assertion, classify unsupported IR kinds
without claiming admission coverage, and retain current Make command order and
resolved feature decisions. Production behavior and recipes were not changed
by these final oracle repairs. Supplemental round-two review is pending.

All receipts are retained beside the owned worktree under `.worktrees/`.
