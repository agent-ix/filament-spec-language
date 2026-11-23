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

## New findings (disposition pass 2)

Head 69344ffc97b7a3ed1fb556e85fa2eab765d48ee7. Focused run on titan: 87 passed, 0 failed, 0 ignored. Mutation in the coder's
titan worktree (reverted, clean after): making prefixes follow array position turns exactly
`incremental_prefixes_follow_causal_order_so_a_verdict_never_returns_to_pending` red, so the AC-26 test discriminates. Probe D
(scratch test, reverted) reproduces FND-014.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-014 | high | Enabledness still returns from `satisfied` to `pending` under causal prefixes: the AC-26 recording plus a third `Placed` of `Buyer` after the second (unordered with `After`), closure `open`, gives `After` `satisfied` at prefix 7 and `pending` at prefix 8. `last_iteration` counts the whole prefix's occurrences, so an iteration begun concurrently with `After` reopens it; the third `Got` can never be ordered before `After` because every causal predecessor of `After` is already in the prefix | fsl-analyze/src/protocol_analysis/recording.rs:3886 |
| FND-015 | medium | The channel-premise obligation can go `satisfied` to `pending` between prefixes (the coder's own note), contradicting FR-895's "a verdict changes only from `pending` to a decision, or from `satisfied` to `violated`", which covers every obligation; the AC-26 test checks enabledness only. An open prefix's `satisfied` channel premise is satisfaction by the absence of a violation, which "Pending, never passed" forbids; it should be `pending` while the prefix is open, and the monotonicity check should cover every obligation | fsl-analyze/tests/it/protocol_replay_mode.rs:1310-1335 |
| FND-016 | medium | The refused-prefix path is untested although it is reachable: `assess_prefixes` takes the `assess` closure, so a `#[cfg(test)]` unit test can inject a refusal at prefix k and assert `PrefixRefusal { prefix: k }`, and the conformance mapping can be extracted into a function and asserted (invalid_runtime_input, subcode, `detail`, `prefix`). The code choice itself is acceptable: `invalid_runtime_input` is FR-737's recording refusal and FR-895 now names it with the `prefix` detail, so no dedicated catalogue code is needed. FR-896-AC-6's sentence "has no test" is not an acceptance criterion | fsl-analyze/src/protocol_analysis/recording.rs:510-573 |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | fixed | per-iteration cases; iterations_taking_different_cases_each_use_their_own (AC-22) and a_case_step_in_the_last_iteration_precedes_the_step_after_the_repeat (AC-23) |
| FND-007 | fixed | exhausted steps collected separately and counted only when they ran; exhausted step has the last iteration as predecessors (AC-24 test) |
| FND-008 | fixed | prefixes in causal order (causal_prefix_order); mutation to array order turns the AC-26 test red. Residual satisfied-to-pending cases are FND-014 and FND-015 |
| FND-009 | fixed | prefixes hold the population of begun instances (FR-896-AC-6 wire test); a refused prefix gives invalid_runtime_input with prefix detail. Missing test is FND-016 |
| FND-010 | fixed | AC-9, AC-10, AC-11 traced; compensation recordings compared batch vs incremental; no-assessment item asserts no prefixes |
| FND-011 | fixed | AC-22 to AC-25 tests: mixed cases, case step in a repeat, exhausted path, any-join |
| FND-012 | still-open | rustfmt --check still reports request.rs:1096 (`item.options.assessment == Some(AssessmentMode::Incremental)` exceeds the width) |
| FND-013 | fixed | AssessmentMode serde enum, kebab-case, default Batch |

## New findings (disposition pass 3)

Head 4dd31a3e50ef92c3d736256d9ac3941a2432f063 (code equal to e3e36334, the coder's titan worktree). Focused runs on titan:
`--test it` filter 88 passed, 0 failed, 0 ignored; `--lib a_refused_prefix` 2 passed. Probe E and a probe D rerun ran as a
scratch test in that worktree, reverted (tree clean after).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-017 | high | A step after a `parallel` under `join any ... outstanding continue` is wrongly `violated`: the closed recording `L`, `Placed`, `R` (Buyer order, `R` still running after the join, which ADR-027 ST-7 allows) gives `Placed` `violated`, `unexpected` in batch, and incrementally `satisfied` at prefix 2 then `violated` at prefix 3 (probe E). The `parallel` arm counts a branch as required when all its steps are observed anywhere, not causally before the step, so a branch that completes after the join becomes a predecessor the step precedes | fsl-analyze/src/protocol_analysis/recording.rs:4008-4040 |
| FND-018 | low | `assert_every_verdict_only_advances` pairs obligations by `find` on the obligation id; enabledness ids are (kind, node, instance), so the second occurrence of a repeated step is compared with the first occurrence's verdict and its own transition is never checked. The helper is otherwise not vacuous (a synthetic satisfied-to-pending pair panics, pending-to-satisfied passes) | fsl-analyze/tests/it/protocol_replay_mode.rs:1333-1355 |

## Dispositions (round 3)

| FND | outcome | reason |
| --- | --- | --- |
| FND-012 | fixed | rustfmt --check clean on request.rs, conformance.rs and recording.rs |
| FND-014 | fixed | iteration and case counts take only observations at or before the step (begun_before); probe D rerun gives After None x6 then satisfied, satisfied; AC-27 test |
| FND-015 | fixed | channel premise pending while the closure is not closed unless violated; assert_every_verdict_only_advances checks every obligation; the two updated expectations follow FR-895 "Pending, never passed" and assert the open/closed premise explicitly |
| FND-016 | fixed | a_refused_prefix_is_reported_with_its_index and a_refused_prefix_refuses_the_item_naming_the_prefix |

## Dispositions (fix round 4)

| Finding | Disposition |
| --- | --- |
| Probe E (`outstanding continue`) | Fixed. A branch counts only when all its steps were observed causally before the step; the join is satisfied when those branches number at least what its policy requires (all, one for `any`/`predicate`, `n` for `quorum(n)`); until then every branch's steps are predecessors. Test `a_branch_finishing_after_an_any_join_does_not_violate_the_step` (FR-895-AC-28): `L, Placed, R` satisfied in batch and from prefix 2, `R, L, Placed` satisfied from prefix 3, the `join all` variant violated. The earlier parallel tests (AC-14, AC-15, AC-25) pass unchanged. |
| Monotonicity helper pairing | Fixed. Obligations are compared per id as a multiset (violated count and decided count never fall); `the_monotonicity_check_covers_every_occurrence_of_a_repeated_step` shows the check fails when a second occurrence of a repeated step goes back to `pending`. |
