---
id: SR-599006
title: 'Gap analysis of PR #2714 against QSL-1906 acceptance criteria'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/filament-spec-language@508b98459; git diff origin/main...508b98459 (PR #2714): fsl-bench/**, Makefile, spec/assurance/MP-007..MP-011; ticket QSL-1906 acceptance criteria'
review_set: subset
date: '2026-11-15'
---

## Summary

Ticket: QSL-1906. PR: filament-spec-language#2714, head 508b98459. The PR adds no FR or AC to this repository, so `quoin matrix` has no new criterion to trace. This gap analysis is a manual check of the ticket's three acceptance criteria and the eight listed axes against the code and the tests, scoped to the diff. Plan completion: not assessed.

## Scope

| Unit | Evidence | Role |
| --- | --- | --- |
| AC1: every axis runs by name under make bench / make bench-probe and is listed in BENCHMARKING.md with unit and generator | BENCH_AXES gains mc reach key en4; bench-probe lines for all 13 probe axes; BENCHMARKING.md table rows | examined |
| AC2: peak RSS prints a number on macOS and Linux | rss.rs cfg(linux) VmHWM and cfg(macos) vmmap; test peak_rss_is_a_number_on_linux_and_macos | examined |
| AC3: counter totals for the smallest size of each axis are asserted in a test | mc.rs counter_tests::smallest_size_counter_totals_are_fixed (run by ci-all-features) plus the per-axis smallest-size tests | examined |
| Axis mc/state_graph, with and without symmetry | state_graph_smallest_size_explores_the_ring_product (2x2), state_graph_symmetry_quotients_the_ring_product (3x3) | examined |
| Axis mc/safety (invariant, deadlock) | safety_smallest_size_holds_over_the_ring_product | examined |
| Axis mc/liveness | liveness_smallest_size_holds_under_weak_fairness | examined |
| Axis mc/frame_candidates | frame_candidates_smallest_size_generates_every_assignment | examined |
| Axis mc/observation | observation_smallest_size_keeps_the_state_count_fixed | examined |
| Axis reach/workers 1/2/4/all | reach_smallest_size_is_the_same_graph_at_every_worker_count | examined |
| Axis key/encode | encode_smallest_size_keys_one_state | examined |
| Axis en4/sample, 10^3..10^5 samples | sample_smallest_size_draws_the_okamoto_sample_count (1440 at eps 0.04) | examined |
| Reporting: states/s, edges/s, peak RSS, marginal bytes per state, samples/s | probe rates() and mc-marginal | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | For most axes, `canonical_encodes`, `expansions` and `generated_candidates` come from the bench's own breadth-first search over the plain `ModelSystem`, not from the engine run the bench times. For liveness, that search counts 4 states and 8 edges while `check_model` explores 12 product states and 28 edges. These counters fingerprint the generator, which is what the AC asks for. They cannot show an engine change, such as fewer canonical encodes, which is the before/after the ticket's Goal wants. BENCHMARKING.md names the seam but does not say the counters do not move when the engine changes. Say so, or keep the gap in a follow-up ticket. | fsl-bench/src/counters.rs:692-714, fsl-bench/BENCHMARKING.md:113-126 |

## Verdict

All three acceptance criteria are delivered, and all eight axes from the ticket's scope exist, run by name and have a smallest-size test with real values. The Linux branch of AC2 is cfg-gated, so this macOS run did not exercise it; the code is the unchanged VmHWM reader. The symmetric 3x3 count is questioned in SR-599005 FND-001.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | counters.rs module doc and BENCHMARKING.md now say the counters fingerprint the generator and do not move with engine changes, citing liveness 4/8 against 12/28. |
