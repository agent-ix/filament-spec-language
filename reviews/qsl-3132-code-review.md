---
id: SR-967175026302197768900474181941935084848
title: "Code review of FSL PR #3824: AC-22 fair cyclic interval replay"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@e8f9db656427ce1e8a2dab5eab45ba46869d0c8e; PR #3824 against origin/main; fsl-analyze/src/analyze/exact.rs, fsl-analyze/src/exact_probabilistic/fair.rs, fsl-analyze/tests/it/exact_probabilistic*.rs, fsl-replay/src/probability_certificate/document/**, tools/fsl-probability-check/tests/it/fair_cyclic.rs; FR-1042-AC-22, TC-3212 step 7"
review_set: subset
---
# Code review of FSL PR #3824

## Summary

Ticket: QSL-3132. The review pass at head `5c305b824d` (Linear comment 7e737a89) recorded two findings. This file was rebuilt from that comment for disposition pass 1 because the original scratch copy was not available. The `## Findings` rows are that comment's findings unchanged. Disposition pass 1 is at head `e8f9db656427ce1e8a2dab5eab45ba46869d0c8e`. It covers the fix commit 09f1b3c72a, the compile fix d1a5de0fac (production change), and the merges bfb24e889c and e8f9db6564.

## Verdict

Review pass at `5c305b824d`: one medium and one low finding. Disposition pass 1 at `e8f9db656427ce1e8a2dab5eab45ba46869d0c8e`: FND-001 and FND-002 are fixed. `fair_cyclic.rs` now sends a one-value mutation of each document through a fresh `fsl-probability-check` process and asserts the rejection rule. The TC-3212 step-7 tests now carry `TC-3212`. The compile fix d1a5de0fac does build: the coder's focused runs pass and the AC-22 analyzer test passes here. But its merge resolution regresses main. Main's two QSL-3095 tests for the same `W` scenario at the AC-22 limits fail at this head: `fair_eventual_value_bounds_are_independently_checked` and `fair_eventual_upper_policy_hint_is_optional_but_independently_checked` (focused run here: 1 passed, 2 failed). The cause is FND-003. FND-004 shows that the PR's fair cyclic settlement is never reached from the analyzer. FND-005 and FND-006 are low. Not mergeable at this head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The mutated value is rejected only by the in-process checker; the separate-process test covers only unmodified documents, contrary to TC-3212 step 7. | tools/fsl-probability-check/tests/it/fair_cyclic.rs:13-33 |
| FND-002 | low | New AC-22 tests omit the required `TC-3212` tag, so the case-to-test matrix does not bind their assertions. | tools/fsl-probability-check/tests/it/fair_cyclic.rs:13-15 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | The d1a5de0fac merge resolution routes every fair eventual claim with `max_rational_bits < 8 * max_precision_bits` to `decide_fair_reachability_interval`, which calls `reachability_threshold`. That returns `Err(InvalidState)` when `precision_bits > max_precision_bits`, and `unbounded_failure` maps that to `broken()`, an engine fault. Main solved this case with the exact solver at a raised rational budget and settled it `proved`. Main's TC-3212 tests set `max_rational_bits = 1` and `max_precision_bits = 16` with the default `precision_bits` 64. Both fail at this head: `fair_eventual_value_bounds_are_independently_checked` and `fair_eventual_upper_policy_hint_is_optional_but_independently_checked`. The PR's own AC-22 test passes only because it also sets `precision_bits = 16`, which the AC text does not state. Fix: on the fair-bounds branch, clamp `solve_limits.precision_bits = min(precision_bits, max_precision_bits)` before the call, or fall back to main's exact path. Keep both main tests green. | fsl-analyze/src/analyze/exact.rs:1631-1645, fsl-analyze/src/exact_probabilistic/unbounded.rs:526-531, fsl-analyze/tests/it/exact_probabilistic.rs:2248,2355 |
| FND-004 | medium | `settle_fair_interval` and `fair_cyclic_probability_proof_document` are unreachable from EN-5 after the merges. Main's `if fair_eventual && holds { if let Some(interval) ... return }` block at exact.rs:1990-2043 always returns first. It builds the documents with `probability_proof_document`, which emits the `fair-mdp-eventual/v1` profile. The `!holds` case with an interval returns `rational_bits_limit` at exact.rs:2161-2163. So the analyzer never emits the `cyclic-mdp-fair-eventual/v1` documents that FR-1042-AC-22 names. The AC-22 analyzer and CLI tests do not assert the profile, so they pass on main's path. `fair_cyclic_rejects_positive_values_on_refined_fair_mecs` and `fair_cyclic_rejects_a_flipped_lower_bellman_direction` build that profile by calling the constructor directly. Main's `fair_eventual_upper_policy_hint_is_optional_but_independently_checked` asserts `fair-mdp-eventual/v1` for the same scenario, so the spec and main conflict; the lead must pick one. Fix: route the interval case to `settle_fair_interval`, or delete the dead path and amend AC-22. Then assert `objective.base.profile` in the AC-22 analyzer and CLI tests. | fsl-analyze/src/analyze/exact.rs:1990-2043, 2161-2163, 2280-2301, 2769-2848 |
| FND-005 | low | `fair_interval_none_fallback` is dead. The `fair_eventual && holds` case always returns at exact.rs:2046-2078 before it, so the `not_yet` it returns can never happen. Its unit test `fair_interval_unavailable_returns_not_yet_instead_of_rejection` tests only that dead helper and has no `#[trace]` tag. Fix: delete the helper and its test, or keep main's `broken()` guard as an invariant. | fsl-analyze/src/analyze/exact.rs:367-373, 2158-2160, 4725-4733 |
| FND-006 | low | `decide_fair_reachability_interval` (a public, re-exported function) builds `Graph::new` and the fair components from the raw product. It skips the FR-204 `Evaluation` that `decide_fair_reachability` applies. On a `FreeDelays` product with two or more rows, each row-choice group carries mass P(g) < 1, so `Graph::new` fails with `InvalidProbability`. The threshold solver's policy is also indexed by evaluation node but is passed to `fair_witness_policy` with the raw graph. This is not reachable from EN-5 today: `FreeDelays` arises only under a workload scheduler (digital.rs:95-103, 2832), and `fair_eventual` requires `every scheduler` (exact.rs:1431-1443). So no analyzer verdict changes; the API diverges from its exact counterpart. Fix: wrap it in `evaluation_of` and restrict, as `decide_fair_reachability` does, or reject `FreeDelays` products explicitly. | fsl-analyze/src/exact_probabilistic/fair.rs:380-397 |

## Dispositions

Round 1 at `e8f9db656427ce1e8a2dab5eab45ba46869d0c8e`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 09f1b3c72a: `fair_cyclic_interval_documents_pass_in_a_fresh_process` now mutates the undecided state's value in each document, runs it in a fresh checker process and asserts exit 1 with `inequality-fails` (lower) and `rank-broken` (upper). |
| FND-002 | fixed | 09f1b3c72a: the CLI test, `fair_cyclic_low_rational_budget_proves_with_value_bounds` and `fair_cyclic_rejects_a_flipped_lower_bellman_direction` carry `#[trace("TC-3212", "FR-1042-AC-22")]`. The two tests still tagged only FR-1042-AC-22 run other models outside TC-3212's steps; that is handled under gap-analysis FND-003 and here under FND-004. |
