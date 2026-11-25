---
id: SR-967175026302197768900474181941935084828
title: "Gap analysis — filament-spec-language QSL-3204 bounded integer literal narrowing"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language PR #3821 @ fb9baf4bf41f3ce8c209cb069821f2ce4684fddb; diff from 6822cf4793; FR-093-AC-4 and related FSpec FR-812/FR-813 behavior; fsl-semantics/src/check/check/typing.rs; QSL-3204"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: reviews
---

## Summary

Planless review of the changed checker behavior against FR-093-AC-4, its test, and its temporal-export rationale. The scoped test is tagged and exercises the required narrow node. A full engine-backed repository matrix could not be obtained: `quoin matrix --repo . --json` refused with `CORE_REFUSED: request exceeds the accepted size`; the `quire matrix --scope . --format json` fallback produced a large symbol-collision diagnostic stream without a usable bounded result. Reverse-gap and repository-wide stub audit are therefore incomplete. Plan completion: not assessed.

## Verdict

**CONDITIONAL** — the changed criterion is covered and its source behavior matches the requirement, but repository-wide matrix/reverse-gap assurance remains incomplete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The repository-wide computed Test Matrix and reverse-gap audit could not be completed because the Quoin matrix request was refused for exceeding accepted size; the Quire fallback emitted an unbounded diagnostic stream and no usable result. This limits the scope of the assurance claim, not the observed FR-093-AC-4 behavior. | quoin matrix; quire matrix |

## Coverage

- Examined FR-093-AC-4 and `conversions_are_classified_and_flat_map_builds_one_node`; confirmed the test tag and assertions exercise the literal-to-`numeric.narrow` path and target type argument.
- Reviewed the modified inference branch; it now emits `ValueType::Integer`, allowing `check_as` coercion to build the narrow node and definedness fact consumed downstream.
- FSpec FR-812/FR-813 context was considered through the stated partial-operation definedness contract and prior repository export work; no changed FSpec artifacts are in this PR.
- Engine-backed whole-repository matrix, complete reverse-gap inventory, and complete stub inventory: incomplete for the command failure above.
- Semantic intent review: not separately opted into; only the requested changed behavior was manually checked.
- Plan completion: not assessed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | rejected | The whole-repository matrix/reverse-gap inventory is outside this PR's bounded review scope, so its unavailability is not a code defect. The single-requirement trace query for FR-093-AC-4 resolved the expected tagged tests, including `fsl-semantics/src/check/lowering/tests.rs::conversions_are_classified_and_flat_map_builds_one_node` and `tests/it/lowering_intake_boundaries/lowering.rs::a_bounded_literal_lowers_to_numeric_narrow_over_its_literal`; the former passed its filtered run (1 passed). The attempted global audit remains unavailable as recorded above. |
