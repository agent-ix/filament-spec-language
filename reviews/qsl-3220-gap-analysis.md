---
id: SR-967175026302197768900474181941935084847
title: "Gap analysis — QSL-3220 uppercase hex diagnostic"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository matrix, fsl-cst/src/parser.rs, fsl-cst/tests/it/syntax_messages.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Audited the changed parser behavior against the owning ticket criterion, the relevant FR acceptance criteria, and the computed repository trace matrix. The changed behavior is backed by a passing regression test, but the full static matrix reports unrelated untagged criteria, so the repository-wide gap verdict is FAIL.

## Verdict

**FAIL** — the static repository matrix contains 1,077 untagged criteria. The QSL-3220 path itself is covered and does not introduce an unowned behavior or stub.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The repository-wide computed matrix reports 1,077 untagged live criteria; this is a pre-existing broad traceability gap outside the changed parser/test behavior. | spec/ |

## Coverage

- Reconciliation: `quire matrix --scope . --format json` (quire 0.52.1), static matrix only; `quoin matrix --repo . --json` (quoin 0.36.10) refused with `CORE_REFUSED: request exceeds the accepted size`, so run evidence was unavailable.
- Matrix counts: tagged 5,498; untagged 1,077; tagged-by-ignored-test 24; retired 15; method-without-symbol 109. `quire coverage --scope . --json` reported no untracked symbol for the changed test's trace tags.
- Relevant criterion: FR-397-AC-11 is tagged by `fsl-eval/tests/it/float_callables.rs::malformed_and_mistyped_bit_constructors_refuse_at_the_constructor`; the existing parser-location test also passes after the change. FR-762-AC-4 is tagged by two relevant missing-semicolon tests, plus a false extra binding from the uppercase-hex test.
- Changed behavior inventory: one parser predicate adjustment, owned by the bit-constructor syntax behavior and QSL-2198 AC-4; no new public surface, untraced behavior, or source/test stub introduced by this diff.
- Plan completion: not assessed
- Semantic review: skipped (not requested).
