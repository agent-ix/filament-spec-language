---
id: SR-977
title: "QSL-2573 code-review review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2573; PR 3106 diff (fsl-semantics/src/check/facts.rs, fsl-replay postcondition_range test, tests/it/source_corpus.rs, tests/corpus/collections/fact-guard-*)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-680
    type: references
examined_units: [{"id":"FR-680-AC-9","path":"spec/functional/FR-680-check-indexing-tuple-positions-maps-option-construction-and-quantity-scaling.md","role":"examined","excerpt":"With `function narrow using v(x: Int[0, 9]): Integer pure { x }` and `x`, `y`, `z` of `Int[0, 9]`, `fsl check` of `if x + y <= 9 then narrow(x + y) else 0` and of `if x - y >= 0 then narrow(x - y) else 0` succeeds at S4 with exit 0; the weak, `!=`, three-term and combined-bound forms each refuse `undefined_expression`/`unproved-range` at the argument, exit 20."},{"id":"FR-577-AC-18","path":"fsl-replay/src/spine/clause/tests/postcondition_range.rs","role":"examined","excerpt":"a_transferred_sum_bound_refines_the_same_sum: checks(PLAIN, &with_sum_bound(\"100\"))"}]
---

## Summary

Ticket: QSL-2573. PR: filament-spec-language#3106. Reviewed head: 5138aaa8e882fa4163c7a1fb3f041ba08bedb0ac. Rust lane (rust-review) folded in. Read-only review; no tests run.

## Verdict

MERGEABLE with conditions. The Sum subject, the difference-fact insertion, the != exclusion on both outcomes, and the rekey of Sum keys are sound: every new fact is a literal bound over an exact integer sum or difference of two distinct stable paths, and stripped conversions are value preserving. One medium (an un-ignored test never executed) must be settled by the merge gate; two lows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The PR removes #[ignore] from a_transferred_sum_bound_refines_the_same_sum (FR-577-AC-18), but the test has never run: fsl-replay lib tests do not compile at the branch base (E0599 at refinement.rs:419, fixed on main by #3087). Rebase onto main and run the fsl-replay test before merge. | fsl-replay/src/spine/clause/tests/postcondition_range.rs:655-659 |
| FND-002 | low | The reproducer is promoted as a new file, but the pending copy tests/corpus/collections/pending/fact-guard-sum-2var.{native,expected.json} added to main by #3056 is not deleted, so the case exists twice after merge. | tests/corpus/collections/fact-guard-sum-2var.native |
| FND-003 | low | For a two-path term, IntervalWalk pushes children through through_conversion. Once QSL-2007 adds integer-to-integer narrowing ConvertScalar, a stripped child gets the operand's declared interval instead of the target domain. That loses precision (completeness only, not soundness). | fsl-semantics/src/check/facts.rs:2006-2016 |
