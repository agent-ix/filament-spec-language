---
id: SR-2026112022385700000000003
title: "Gap analysis of filament-spec-language PR #3234: FR-230-AC-68 and FR-230-AC-4"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@b550cd35526f11fe3c84a5ff8c23e110ed394d9a; PR #3234; FR-230-AC-68, FR-230-AC-4 and the FR-230 Recursive clock readings statement; fsl-semantics/src/check/timed_model/reading.rs, fsl-semantics/tests/it/timed_model.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: references
---
# Gap analysis of PR #3234

## Summary

Ticket: QSL-2144. Planless, scoped to the two criteria the PR claims. `quire matrix --scope <repo> --format json`
at the head binds FR-230-AC-68 to `a_measured_recursive_clock_reading_unfolds`
(no longer `#[ignore]`d) and FR-230-AC-4 to the new
`a_disjunctive_guard_with_an_inadmissible_operand_refuses` beside its earlier
tests. No criterion in scope is untagged. Semantic intent was checked by hand
in the code-review artifact (SR-2026112022385700000000002); the opt-in
LLM semantic pass was not run.

## Verdict

FAIL: one AC-68 clause has no implementation and no assertion (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-230-AC-68's "refuses ... at the call naming `loop`, `loop`" is tagged as covered but neither implemented nor asserted: the refusal names no declaration and the test checks only code, tag and region | fsl-semantics/tests/it/timed_model.rs:1751 |
| FND-002 | medium | FR-230 Recursive clock readings statements with no implementing code: Clock-result recursion unfolding, `match` condition evaluation and named-constant arguments (detail in SR-2026112022385700000000002 FND-003, FND-005) | fsl-semantics/src/check/timed_model/reading.rs:1420; fsl-semantics/src/check/timed_model/reading.rs:1552 |

## Coverage

- FR-230-AC-68: tagged, one clause (cycle naming) unasserted.
- FR-230-AC-4: tagged; the new negative vectors can fail.
- Reverse gaps: none in the diff; `settle_unfoldings` and `PartBudget::unfold` serve FR-230 Recursive clock readings.
- Stubs: none.
- Semantic review: skipped (opt-in pass; manual intent check in the code review).

Plan completion: not assessed
