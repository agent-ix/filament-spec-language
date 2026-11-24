---
id: SR-9999999999999999999999999999999999991
title: "Code review — QSL-3108 bounded Set/Bag min/max folds"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-cst/src/grammar.rs; fsl-forms/src/value.rs; fsl-semantics/src/check/check.rs; fsl-semantics/src/check/check/typing.rs; fsl-semantics/tests/it/checker_rulings.rs; fsl-eval/tests/it/collection_queries.rs; fsl-eval/tests/it/value_properties.rs; QSL-3108"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-701", type: reviews}
---

## Summary

Reviewed QSL-3108 PR #3701 at frozen head `c04ac782a1e8a97ab3d9e7ec34cf19230e51c65d` against FR-701-AC-1/2/3, TC-1907, and QSL-2055 P5. The grammar, typing context, catalog lowering, bounded order cases, empty fold identity, nonempty Set/Bag permutation behavior, and FR-701-AC-1/2 controls are consistent; one medium verification gap remains for Rational Bag reductions.

## Verdict

**CONDITIONAL** — the implementation is scoped to Set/Bag fold/reduce step checking, but AC-3's bounded Rational Bag `reduce` forms have no focused checker or evaluator coverage.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-3's bounded Rational Bag min/max reductions are not checked or evaluated by the added cases; current Rational coverage checks folds only, while reductions cover bounded Int Set/Bag. | fsl-semantics/tests/it/checker_rulings.rs:726; fsl-eval/tests/it/value_properties.rs:1730 |

## Review notes

`min`/`max` call typing is gated by the unordered fold/reduce step context, and the catalog accepts only the lowered ordered-comparison/operand-selection form; Sequence min/max remains on its prior path. The `max` parser case only adds reserved-word call syntax; type checking still controls admissibility. The P5 property generates nonempty Set/Bag cases, compares three permutations, checks empty fold identities, and separately exercises nonempty reductions. FR-701-AC-1/2 checker cases are unchanged, including bounded Rational arithmetic refusal. The changed tests carry TC-1907 and FR-701-AC-3 trace tags. No applicable AssuranceProfile was found.

## Test evidence

The PR's listed focused filters are the applicable checks. My rerun waited on the worktree target lock held by a concurrent Cargo build and produced no test result; the code lead reports the same two filters green on this exact head.
