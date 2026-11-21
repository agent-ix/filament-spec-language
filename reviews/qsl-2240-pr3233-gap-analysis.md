---
id: SR-2026112022385700000000003
title: "Gap analysis — QSL-2240 PR #3233 FR-418 checker DBM operations (slice S2)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3233 diff at 3aaef3851: fsl-replay/src/certificate/zone.rs, fsl-replay/src/certificate/zone/region_tests.rs"
review_set: subset
evaluated_revision: "3aaef38517238cbbbda911d6f44222ec93f4ecc2"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-245
    type: references
---

## Summary

Ticket: QSL-2240. Plan completion: not assessed. Scoped AC-to-test check of FSpec FR-418 against the tests this PR adds. FR-418 is an FSpec requirement; at the FSL pin `5c4384df` it has AC-1 to AC-17, and the unmerged FSpec branch adds AC-18 to AC-21.

| Criterion | Test | Result |
| --- | --- | --- |
| FR-418-AC-4 (arbitrary-precision bounds) | `subtract_bounded_is_exact_beyond_machine_integers` | covered (narrow: one operation) |
| FR-418-AC-13 (templates, renaming, replay) | 12 region_tests tests | tagged but not exercised (FND-001) |
| FR-418-AC-18 (inclusion after region closure, end to end) | none at the pin; `saturation_decides_coverage_above_the_ceiling`, `diagonal_truth_decides_coverage` and the property test exercise its unit core | AC not yet at the pin |
| FR-418 behaviour: refined-region relation, bounded subtraction guard | all region_tests | no AC at the pin (FND-001) |

Production code with no owning AC: `subtract_bounded`, `constrain_diagonal`, `region_normalize`, `region_covers` are owned by FR-418 behaviour text, not by any AC at the pin.

## Verdict

**FAIL** on traceability only: FND-001 of SR-2026112022385700000000002 inflates AC-13. No further gap of its own.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-418-AC-13 would read covered in the matrix from tests that exercise only DBM operations; the refined-region relation and the subtraction guard have no AC at the pin, and FR-418-AC-18 is not yet pinned | fsl-replay/src/certificate/zone/region_tests.rs:87-431 |
