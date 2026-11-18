---
id: SR-2026111612172400000043
title: "Gap analysis of QSL-2000 slice B delay_distribution emission (filament-spec-language PR 2934)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@a5386b9f5ebbef2e3ccaea2144efd648743fd4db; git diff origin/main...HEAD (PR 2934); FSL FR-740 AC-3 and Behavior (TC-2062), FR-253 Behavior; FSpec FR-640 AC-2..4 (TC-1563) and FR-322 'Probabilistic-model nodes' at agent-ix/filament-specification origin/main 06d4f215"
review_set: subset
date: 2026-11-18
---

## Summary

Ticket: QSL-2439 (QSL-2000 slice B). AC-to-test check over the diff's tagged tests and
code (`quoin matrix --repo . --json` produced no output in this worktree, so the computed matrix was unavailable and this is a manual check). Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-740 "A failure to emit one of these nodes SHALL refuse the package with the declaration's source locus" has no delay test: no test drives `delay_node`'s refusal arms (no model node for the declaring key; no dense timed root on the declaring node), so the non-null-locus requirement QSL-2439 states for delays is unasserted (see SR-...42 FND-001 for the reachable case) | fsl-package/src/emit/probabilistic.rs:430-449 |
| FND-002 | low | FR-740 "reordering ... the unit's declarations changes no node's bytes" is untested for delays: every delay test unit holds one delay, so the node-id sort of two or more `delay_distribution` nodes (and their order among random parameters and workloads) is never exercised | fsl-package/src/emit/probabilistic.rs:59-62; fsl-package/src/emit/tests/probabilistic.rs:966-1078 |

## Scope and evidence

Examined (clean unless listed above):
- FR-740-AC-3 delay half: covered. `each_delay_family_is_emitted_as_a_delay_distribution_node`
  (fsl-analyze, `time dense` unit, `discrete { 1 ms: 90, 3 ms: 10 }` and the other three
  families, through `emit_checked`) and `delay_round_trips` (strict reader admits, one
  `declaration` occurrence at the label span). The original
  `discrete_delays_are_emitted_as_delay_distribution_nodes` stays ignored, now citing IR-1569;
  run with `--include-ignored` it fails as the reason states (Boolean random parameter
  `send(lost)`), body unchanged.
- FR-740 "converted exactly to the timed model's time unit": S3 converts (FR-253
  `distribution`), S4 types at the dense source's unit node; IR admission checks the type id.
- FR-740 "rebuild ... the delay records equal to S3's": `a_delay_reads_back_as_the_s3_record`
  compares count, operation key and distribution per family; `declared` differs by design
  (slice A convention).
- FSpec FR-640-AC-2: emission and admission per family covered; the AC's literal values are
  not asserted (SR-...42 FND-003).
- FSpec FR-640-AC-3: `changing_a_delay_parameter_changes_the_node_identity` asserts a changed
  discrete weight, a changed discrete delay and a changed rate change the node id.
- FSpec FR-640-AC-4 delay mutations (untimed model, discrete weight 0, second
  `delay_distribution`): IR reader checks, not in this repository's diff.
- FR-253 refusals (rate <= 0, `uniform[a, b]` outside `0 <= a <= b`, negative/repeated
  delays, weight <= 0, untimed model with locus): S3 code unchanged by this PR.
- Admission corpus: `DelayDistribution` moved from NOT_EMITTED_BY_QSL to a backed row
  (`Fixture::Delay`); `every_emitted_node_family_is_admitted` passes.
- No test deleted or weakened; the fsl-semantics `assert_rebuilds_s3` literal gains
  `delays: Vec::new()` only.

## Verdict

FR-740-AC-3's delay half and FSpec FR-640-AC-2/AC-3 delay cases are met and tested. FND-001
(refusal locus coverage) should be closed in this PR together with SR-...42 FND-001; FND-002
is a cheap determinism test.

## Dispositions

Round 1, head c4aea60878421a962cdf80497d98628a059ecd80 (fix commit 5d65b7f59).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The refusal arm is reachable (a timed source model's delay refuses at S4, see SR-2026111612172400000042 FND-001 round 1), and no test yet asserts that refusal or its locus; `the_delay_unit_is_that_of_the_dense_source_of_the_declaring_nodes_root` checks only `dense_unit` returning None for a non-root node, and `a_delay_beside_a_second_object_type_is_refused_at_the_time_source` stops at S3 |
| FND-002 | fixed | `reordering_two_delays_changes_no_delay_node` emits two delays in both declaration orders and asserts byte-equal delay nodes in the same order |
