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

## New findings (disposition pass 1)

Head c548c7922f72dbe30ec10f6b769bf588a7cc5860. Focused run on titan: 81 passed, 0 failed, 0 ignored. Three probes,
run in a scratch test appended to the coder's titan worktree and reverted (tree clean afterwards), reproduce FND-006 to FND-008.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | A step after a `repeat` whose iterations took different `choice` cases is wrongly `violated`: every case observed anywhere becomes a body step and "most occurrences" requires the 2nd occurrence of each, so `Placed, Got, yes, Placed, Got, no, After` (closed) gives `After` `violated`, `unexpected` (probe A). Inside a repeat, a step after a choice also matches the n-th case occurrence across iterations | fsl-analyze/src/protocol_analysis/recording.rs:3736-3790 |
| FND-007 | high | `exhausted` path steps are collected as iteration body steps: with `exhausted sequence Out { event X }`, the closed recording `Placed, Got, Placed, Got, X, After` gives `After` `violated`, `unexpected` (probe B). FO-1 and the compiled graph make `exhausted` an alternative exit taken once after the last iteration, not part of an iteration; an exhausted step itself gets no predecessor from the last iteration | fsl-analyze/src/protocol_analysis/recording.rs:3756-3790 |
| FND-008 | high | Incremental monotonicity is broken: with array order `Placed, Got, After, Placed, Got` (legitimate, causal order via local orders), `After` is `satisfied` at prefix 3, `pending` at prefix 4, `satisfied` at prefix 5 (probe C). FR-895 "Incremental assessment" says a verdict changes only from `pending` to a decision or from `satisfied` to `violated`; `assert_violations_persist` checks only violated to violated, so nothing tests the claimed monotonicity | fsl-analyze/tests/it/protocol_replay_mode.rs:403-421 |
| FND-009 | medium | A refused prefix turns the whole incremental item into `runtime_invariant`/`established-invariant-broken`. A prefix of an admitted recording can be refused: an `on each` population naming an instance whose observations come later in the array fails `validate_population` ("names unrecorded instance") on the earlier prefixes, so a legitimate request fails with an internal-invariant code; FR-896 does not say what a refused prefix gives | fsl-analyze/src/protocol_analysis/conformance.rs:128-144 |
| FND-010 | medium | Traceability and stale text: FR-895-AC-9, AC-10 and AC-11 have no traced test (the AC-9/AC-10 tests trace AC-5, the incremental tests trace AC-6); AC-6's own compensation recordings are never assessed incrementally, and the AC-6 test still says the incremental surface "awaits the spec ruling"; FR-896-AC-5's "or none carries no prefixes" is not asserted | fsl-analyze/tests/it/protocol_replay_mode.rs:230-283 |
| FND-011 | medium | No test for a `parallel` under a non-`all` join (the "branches whose steps were all observed" rule), a step in a choice case inside a repeat, mixed cases across iterations, or a non-empty `exhausted` path | fsl-analyze/tests/it/protocol_replay_mode.rs:719-960 |
| FND-012 | low | PR-added lines are not rustfmt-clean: the `assessment` struct fields in document.rs are over-indented and the fsl-replay re-export line exceeds the width | fsl-analyze/src/analyze/document.rs:1801 |
| FND-013 | low | `assessment` is an `Option<String>` checked only in fsl-command; library callers pass any word and get batch silently (`== Some("incremental")`). A serde enum `Batch`/`Incremental` would make the bad state unrepresentable | fsl-analyze/src/analyze/request.rs:125 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | earlier_steps_precede requires a causally earlier observation; tests a_send_before_its_earlier_own_receive_is_unexpected and an_earlier_own_receive_listed_first_but_ordered_later_is_unexpected |
| FND-002 | fixed | collect_predecessors covers every step kind of any role and nested sequence, choice, parallel and repeat siblings per the arch ruling; repeat defects are new findings FND-006/FND-007 |
| FND-003 | fixed | tautological prefix loop removed; real assess_recording_incrementally with distinct prefix labels and last prefix equal to batch |
| FND-004 | fixed | test un-ignored, asserts witness [0, 3] and trace_position 0 |
| FND-005 | fixed | earlier_own_steps_observed and its into_iter removed |

## Dispositions (fix round 1)

| Finding | Disposition |
| --- | --- |
| FND-006 (different cases across iterations) | Fixed. Each iteration's predecessors use its own taken case (`collect_predecessors` with an iteration occurrence). Tests: `iterations_taking_different_cases_each_use_their_own` (FR-895-AC-22), `a_case_step_in_the_last_iteration_precedes_the_step_after_the_repeat` (AC-23). |
| FND-007 (`exhausted` counted as iteration) | Fixed. The `exhausted` path is an alternative exit: its steps precede a later step only when it ran, and its own predecessors are the last iteration's steps. Test: `the_exhausted_path_follows_the_last_iteration_and_precedes_the_step_after` (AC-24); FR-895 rules 6 and 7. |
| FND-010, FND-011 (test gaps) | Fixed. Non-all join (AC-25), case step in a repeat (AC-23), mixed cases (AC-22), non-empty exhausted path (AC-24); tests now trace AC-9, AC-10, AC-11; AC-6's compensation recordings are also assessed incrementally; FR-896-AC-5 also asserts no `prefixes` without `assessment`; the stale comment is corrected. |
| Low: rustfmt and `assessment` typing | Fixed. document.rs hunks and the fsl-replay re-export are formatted; `assessment` is the serde enum `AssessmentMode` (`batch`/`incremental`), so library callers cannot pass any word. |
| Monotonicity (arch ruling, option a) | Implemented. Prefixes follow the causal order, ties by array position. Test: `incremental_prefixes_follow_causal_order_so_a_verdict_never_returns_to_pending` (FR-895-AC-26). The check covers enabledness obligations; channel-premise obligations may return from `satisfied` to `pending` when a later send is not yet received, which is not a wrong verdict. |
| Refused prefix (arch ruling) | Prefixes close over every instance already begun (FR-896-AC-6, test `an_incremental_prefix_assesses_only_the_instances_it_has_begun`). A prefix refusal now refuses the whole item `invalid_runtime_input` with the cause and `prefix` index in its detail. No fixture of an admitted recording reaches a refused prefix, so that refusal path has no test. |
