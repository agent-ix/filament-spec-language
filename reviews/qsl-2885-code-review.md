---
id: SR-3595001
title: "code-review of PR 3595 replay-mode enabledness and stale replay tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/protocol_analysis/recording.rs, fsl-analyze/tests/it/protocol_replay_mode.rs, fsl-analyze/tests/it/protocol_analysis_differential.rs at dac87bebc84fd670e32aba5fcd13e8996a9d11eb"
review_set: subset
---

## Summary

Ticket: QSL-2885. PR 3595 adds `earlier_own_steps_observed` to replay-mode enabledness and repairs five
stale replay/capacity tests; one test stays ignored on a spec gap. Rust review folded in. Focused run on
titan: 64 passed, 0 failed, 1 ignored (protocol_replay_mode, a_symbolic_capacity_settles, protocol_recording,
protocol_conformance, recording_population, analyze_package_replay_conformance). A temporary probe in the
reviewer's own worktree (reverted) showed the new rule accepts a send that precedes its own earlier receive.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `earlier_own_steps_observed` tests that the earlier own step was observed anywhere in the recording, not that it precedes the observation in causal order: a recording with Seller local order [Note, Got] settles `proved` with every obligation satisfied, although Seller sent Note before Got (FR-895 Enabledness "earlier observations", AC-5) | fsl-analyze/src/protocol_analysis/recording.rs:3452-3458 |
| FND-002 | medium | The rule only inspects direct `send`/`receive` siblings owned by the observing role: earlier own `event`/`attempt`/`effect`/`commit` steps and steps nested in a compound sibling (nested sequence, choice, parallel, repeat) are skipped, so their absence never disables the later step; ADR-027 CI-1 / FSpec FR-052 make every sequence predecessor a causal predecessor | fsl-analyze/src/protocol_analysis/recording.rs:3437-3450 |
| FND-003 | high | The AC-6 batch/incremental comparison is tautological: "incremental" re-analyses observation prefixes and keeps the last, which is always the whole recording, i.e. the batch request; with one observation both modes send the identical request. The assertion cannot fail and FSpec FR-059-AC-4 / FR-895-AC-6's agreement clause is untested; FSL exposes no incremental assessment to compare against | fsl-analyze/tests/it/protocol_replay_mode.rs:256-290 |
| FND-004 | medium | `a_pair_on_a_causal_cycle_is_misordered` stays `#[ignore]`, so the ticket's acceptance (listed tests pass un-ignored) is unmet; FR-895-AC-2 determines only that both observations 3 and 0 are named, so an order-free assertion (sorted witness == [0, 3]) is exact against the spec and could run now, pending the spec ruling on order | fsl-analyze/tests/it/protocol_replay_mode.rs:141-176 |
| FND-005 | low | `earlier.into_iter()` is a no-op conversion on an iterator (clippy `useless_conversion`) | fsl-analyze/src/protocol_analysis/recording.rs:3444 |

## Verdict

Not mergeable. The test repairs in (a) are fixes against the spec, not weakenings: closure premises now carry
Flow::Done and the Decide branch premises (FSpec FR-059-AC-5), the non-observable `event` on the empty
`Ship` sequence became the `yes` case observation, the compensation attempt uses the FR-236 label
`successful` and the forward node, the capacity request moved to `temporal.subject.protocol.capacities`
with `temporal.profile/max_depth/max_history` (FR-575, FR-891), and every expected value is unchanged. The
engine rule fixes AC-5's single-observation case and regresses nothing in the focused suites, but it decides
"earlier" by presence rather than causal precedence (FND-001, reproduced), covers only own direct
send/receive siblings (FND-002), and the batch/incremental claim is vacuous (FND-003).
