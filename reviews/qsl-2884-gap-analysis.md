---
id: SR-967177
title: "QSL-2884 PR 3590 gap analysis: FR-204 several racing given delays"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@99104cefc512ce65d96a0f37916a967f5b5748f4; fsl-replay/src/probability_certificate/document/timed.rs, fsl-analyze/tests/it/probability_from_source.rs, spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-204", type: references }
---

## Summary

Ticket: QSL-2884. A planless gap analysis scoped to the PR diff. The changed
code implements FR-204's workload race clause ("each racing identity SHALL
draw its `discrete` delay conditioned on its window, the smallest draw winning
with ties ordered by the workload's weights") for several given racers.
`quire matrix --scope .` was read for FR-204-AC-1..17 and FR-741-AC-3.

## Verdict

CONDITIONAL. FR-741-AC-3 is tagged by `pta_from_source`, which is now
un-ignored with its assertions unchanged. The five on-each tests stay
ignored, citing the open QSL-2979, which matches the measured state: nothing
in fsl-analyze or fsl-replay reads a probabilistic `on each` activation. No
stubs were found in the diff, and no reverse gap: the new code is owned by
FR-204.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No FR-204 criterion exercises two or more given-delay identities racing under a workload. AC-3 and AC-6..12 and AC-17 each have a single given racer. The new branch is reached only through FR-741-AC-3, a source-routing criterion with equal weights and identical supports, so the weight-ordered tie and the support product of the FR-204 clause are untested at criterion level. | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:136-140, fsl-replay/src/probability_certificate/document/timed.rs:3463-3506 |

## Coverage

- Criteria examined: FR-204-AC-3, FR-204-AC-6..12, FR-204-AC-17 and FR-741-AC-3, all tagged by tests in the computed matrix.
- Untagged criteria in scope: none. The clause gap is recorded as FND-001.
- Ignored tests in the touched file: 5, all citing QSL-2979 (In Progress). None cites a Done ticket.
- Semantic review: skipped (not opted in). Intent was checked manually for the changed function only.
- Tests were not executed. Host load limits allowed at most one focused cargo test, and it was not essential: it is deferred to the aggregate gate.
- Plan completion: not assessed

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | AC-19's `Pair` is realised with one `Call` object and `fire(which: Boolean)` (F1 = which true, F2 = which false), not the spec's two-object `Node` population with `n1.hit`. The divide-by-identity-count rule is keyed on the operation name, so the rule under test is preserved. But AC-19's literal model is not exercised, and the coder reports a two-object population fails the claim's `over` binding. If that is confirmed, it is a tool or spec gap and needs a ticket (or a citation of QSL-2979 if that covers it). | fsl-analyze/tests/it/digital_clocks.rs:1963-2045, spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:254 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Spec PR #3594 merged FR-204-AC-18/19/20 (TC-3204/3205/3206). Tagged tests `given_delays_race_under_unequal_weights`, `operation_weight_is_shared_among_tied_identities` and `tied_operations_are_ordered_without_replacement` assert the merged numbers and rejections. |
