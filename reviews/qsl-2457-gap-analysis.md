---
id: SR-2026111612172400000047
title: "Gap analysis — QSL-2457 red-main fixes"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@213caccd08fda3cd2cd723354c3fbd8413e5f5b5; changed fsl-eval, fsl-replay, and fsl-semantics paths; FR-513 row FR-015-AC-5, FR-100-AC-8, FR-122-AC-1/4, FR-266-AC-2; FR-268-AC-1 context-only"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-015
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-513
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-122
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-266
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-268
    type: references
---

## Summary

Planless, PR-scoped gap analysis of the ten named QSL-2457 regressions and changed tests. The computed matrix tags the selected active criteria, but FR-266-AC-2's active test checks only witness presence instead of the comparison-record details the criterion requires. Plan completion: not assessed.

## Verdict

**CONDITIONAL** — the FR-266-AC-2 evidence needs an exact record-content assertion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-266-AC-2 is tagged, but the active corpus test does not verify the required comparison-record contents | fsl-replay/src/spine/clause/tests/witness.rs:1620-1657; spec/functional/FR-266-report-a-clause-run-s-witness-and-settlement-basis.md:85 |

## Coverage

- FR-015-AC-5: tagged by the state-clause dereference tests; the new branch is exercised by the named absent-field test.
- FR-100-AC-8: tagged by active TC-452 surface tests; the named scanner check passes per the coder's focused evidence.
- FR-122-AC-1 and AC-4: tagged by active TC-517 tests for the replayed postcondition, invariant, input arm, and precondition cases.
- FR-266-AC-2: tagged by active TC-741, but its decisive corpus checks reduce the witness to `is_some()` rather than checking the specified `<` operands/path and root `=` record (FND-001).
- FR-268-AC-1: context-only. The matrix shows only the ignored TC-743 test binds this criterion; its ignore is tracked under QSL-2173/QSL-2391. The changed TC-517 input-arm case uses VersionUnchanged and remains tagged to FR-122-AC-1, so it does not establish FR-268-AC-1's AllBelow/NestedAll/FilteredAll coverage.
- Reverse gap among changed production items: none found; the dereference behavior and public facade boundary map to FR-015-AC-5 and FR-100-AC-8 respectively. Fixture updates support the existing FR-122 and model-query behavior.

Plan completion: not assessed


## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | The fix now checks ParentOrder's comparison record, but changed-version and unchanged-version corpus reports still assert only basis and witness presence. FR-266-AC-2 also requires the VersionUnchanged root-equality operands and decision path, so the active TC-741 coverage remains incomplete. |
| FND-001 | fixed | TC-741 now asserts the VersionUnchanged record occurrence, post-call then pre-call operands, and equality-only decision path for both changed and unchanged corpus runs. |

### FND-001 round 2 after excerpt

```rust
let record = report.witness.expect("the decisive report has a witness record");
assert_eq!(record.deciding_comparison, Some(version_comparison.clone()));
let operands = record.operands.as_ref().expect("comparison operands");
assert!(crate::result::same_element(&operands[0], &integer(post_version)));
assert!(crate::result::same_element(&operands[1], &integer(pre_version)));
assert_eq!(record.decision_path, Some(vec![version_comparison.clone()]));
```

Both corpus outcomes invoke this assertion: the changed run checks post 3 / pre 2 and `decisive-counterexample`; the unchanged run checks post 2 / pre 2 and `decisive-witness`.
