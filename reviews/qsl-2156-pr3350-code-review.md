---
id: SR-2026112117314700003350
title: "QSL-2156 PR 3350 code review"
type: SpecReview
date: 2026-11-21
scope: "agent-ix/filament-spec-language; fsl-analyze/src/exact_probabilistic/finite_horizon.rs, fsl-analyze/src/analyze/exact.rs, fsl-replay/src/probability_certificate.rs, fsl-replay/src/probability_certificate/timed_axes.rs, fsl-replay/src/probability_certificate/document/timed.rs, fsl-replay/src/lib.rs, fsl-analyze/tests/it/timed_product_certificate.rs"
---
# SR-2026112117314700003350: QSL-2156 PR 3350 code review

## Summary

Ticket: QSL-2156. PR 3350 at 860ae812a837726fc50e10f41fb5c73a677d82a5. Method: code-review with rust-review lane, diff only, no builds or test runs. Governing text: FR-921 (checker re-derives with code EN-5 does not share), FR-924 (AC-3, AC-5, AC-6), FR-127 / ADR-018 CX-4.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The producer (EN-5) classifies collapsible components with the checker's own fsl_replay::timed_edge_axes, so one implementation decides both sides; FR-921 requires the checker to compute the components with code EN-5 does not share | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:18, fsl-analyze/src/exact_probabilistic/finite_horizon.rs:263-281, fsl-replay/src/lib.rs:491 |
| FND-002 | medium | Clock(i) components are maximal end components of the axis-restricted graph, not maximal end components judged by the Alur-Dill condition; in a key without a deadline clock (mean-time route) a component can lie inside a progressive end component whose extra action resets the frozen clock, and it is still collapsed | fsl-replay/src/probability_certificate.rs:1837-1880, fsl-analyze/src/exact_probabilistic/finite_horizon.rs:302-340 |
| FND-003 | medium | The checker's workload filter drops unweighted and zero-weight operations from both the actions and the race set, so a state where only such an operation is enabled becomes a checker time-lock (value 0) or a single-choice state; the producer keeps those operations racing or faults on them | fsl-replay/src/probability_certificate/document/timed.rs:1428-1438, fsl-replay/src/probability_certificate/document/timed.rs:2370-2388 |
| FND-004 | medium | The new test sits between the existing TC-632 / FR-197-AC-4 tag and precision_budget_contains_an_exact_threshold, so the tag now binds the policy-iteration test and the PrecisionBudget test lost it | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:1647-1675 |
| FND-005 | low | timed_axes view() fails open: a timed key whose members do not parse is treated as non-timed (Still), an unparseable deadline becomes (MAX, MAX) and equal on both sides, and the Deadline axis has no cap exemption that FR-924 states | fsl-replay/src/probability_certificate/timed_axes.rs:43-70, fsl-replay/src/probability_certificate/timed_axes.rs:80-101 |
| FND-006 | low | The untimed bounded-event path still maps every FhError, including the new PolicyIterationLimit, to broken() | fsl-analyze/src/analyze/exact.rs:1028 |
| FND-007 | low | About 12 files carry rustfmt-only hunks unrelated to QSL-2156 (model_check/check.rs, zone_check.rs, certificate/closure.rs, hyper_wire.rs, hyper_world.rs, main.rs, spine.rs and others) | fsl-analyze/src/model_check/check.rs:891-958, fsl-replay/src/certificate/closure.rs:473-530 |

## Verdict

Fix round needed (FND-001 high). Sound parts: the limit-stop mapping (MaxPolicyIterations becomes PolicyIterationLimit only from policy_iteration; Singular, MissingReward and ZeroWeightComponent stay Cycle; run_timed settles it as the exact_prob.max_policy_iterations limit, never proved). The nested-merge and partial-overlap rule in both producer and checker is fail-closed (Cycle / UnsupportedClaimForm). The Clock(i) edge rule (whole part unchanged, not reset, below cap at both ends) is a correct per-edge reading of FR-924's three progress witnesses, and every collapsed set is truly time-convergent (the frozen clock is never reset and stays inside one unit interval). In deadline products the Deadline axis subsumes every other axis, because the deadline clock is never reset and every cycle keeps its whole part. The workload Unsupported guard on states with more than one choice is fail-closed for ties and free delays. Digital expected elapsed time uses exact rationals (cost 1/scale per unit delay, the producer's elapsed_ticks/scale), bounded by the document state and edge limits, and drops a key-preserving delay exactly as the producer's running_delays does. Raised test-helper limits (4096 states, 65 536 edges) only affect the check() helper of timed_product_certificate.rs; the largest product there is AC-6's 217 states. The writer.rs test the author saw red (bounded_kernel_decimal_output_is_admitted_before_retaining_transport_text) uses document/writer.rs, document.rs's tests fixtures and quire_canonical; the PR touches none of these and no Cargo.lock, so it is not affected by this PR. No Python or logic-bearing shell.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | a_workload_that_gives_an_enabled_operation_no_weight_is_not_proved is tagged FR-923-AC-3, whose text is the Both tie (21/40, refuted / proved), not a zero-weight refusal; and it asserts only "not proved", so it stays green if admit_workload is deleted and the producer refuses or faults on its own | fsl-analyze/tests/it/timed_product_certificate.rs:840-856 |
| FND-009 | low | New hunks are not rustfmt-clean: lone `;` lines after the actions_with_cancel chains and `)) {` on the condition line | fsl-replay/src/probability_certificate/document/timed.rs:880, fsl-replay/src/probability_certificate/document/timed.rs:1431, fsl-replay/src/probability_certificate/document/timed.rs:1800, fsl-replay/src/probability_certificate.rs:2055-2057 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At d3079e1a fsl-analyze has no reference to timed_axes, timed_edge_axes, TimedAxis, component_is_progressive, edge_is_zero_time or nonprogressive_end_components; lib.rs export removed; timed_axes is a private mod with pub(crate) items. The producer collapses zero-elapsed MECs plus, for Deadline-monitor products, every MEC of Graph::with_terminals (decided states have no choices, so a deadline at its cap is in no MEC). Sound. |
| FND-002 | fixed | Checker: MECs over all choices (ComponentKind::Closed), each tested by component_is_progressive for reset / crossing / cap per clock and deadline crossing / past-limit; only non-progressive ones kept; zero-time MECs kept separately. Producer rule is separate code. A disagreement changes which equations the producer's values were built for, so the checker's own conditions reject them; acceptance only when values satisfy the checker's components. Overlap refusal remains but cannot trigger (MECs are disjoint, zero-time MECs nest in one). |
| FND-003 | fixed | admit_workload returns Unsupported for any listed action not positively weighted, at every enumerator and inside digital_enabled_actions; workload_allows is gone. |
| FND-004 | fixed | New test moved above the TC-632 / FR-197-AC-4 block, which again sits on precision_budget_contains_an_exact_threshold. |
| FND-005 | fixed | shape() returns Malformed for unparseable timed members; edge_is_zero_time is false and component_is_progressive true for it, so it never collapses; no (MAX, MAX); deadline capped only when whole > limit. |
| FND-006 | fixed | exact.rs run(): Err(FhError::PolicyIterationLimit) settles ExactProbMaxPolicyIterations before Err(_) => broken(). |
| FND-007 | fixed | Diff is 7 files; the rustfmt-only hunks are gone. |
