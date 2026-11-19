---
id: SR-2026111612172400000051
title: "Gap analysis — FSL repository matrix and QSL-2500 change"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; spec/, fsl-command/, fsl-replay/, tests/"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-867", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-286", type: references}
---

## Summary

This planless audit computed the repository-wide static Test Matrix and Quoin evidence matrix, then checked the changed FSL behavior against FR-867 and FR-286. The repository matrix contains broad pre-existing coverage gaps and no bound run evidence. In the changed behavior, FR-286-AC-13 has no test binder for the clause-run document conversion path touched by this PR.

## Verdict

**FAIL** — the computed repository matrix has 1,144 untagged criteria and 308 criteria bound only by ignored tests; the repository-wide coverage report also contains 4,608 untracked tags.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-286-AC-13 is untagged, so the clause-run document conversion path touched by this PR has no test binder. | FR-286-AC-13 |

## Coverage

- Reconciliation: quoin matrix (quoin 0.36.7; quire 0.52.0, engine 0.62.2); quire coverage used for untracked symbols and diagnostics.
- Plan completion: not assessed
- Criteria: tagged 4,248 · untagged 1,144 · tagged-by-ignored-test 308 · method-without-symbol 103 · retired 13 (5,816 total)
- Evidence: bound 0 · stale 0 · suspect 0 · undischarged 0 · no run evidence 5,816
- Coverage report: 4,608 untracked symbols and 20,172 diagnostics; no untracked tag was introduced by the changed files' new criterion tags.
- Changed behavior ownership: FR-867 and FR-286 cover the added serialization behavior. The PR changes no public command, configuration surface, or persistence behavior outside those requirements.
- Reverse-gap scan: changed implementation and test surface only; a manual inventory of every repository behavior was not completed.
- Semantic review: skipped.

## Dispositions

Disposition pass 1 rechecked PR #2982 at `36a0eeb9447c92d9e91ef6428d939672a7372134`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The changed claim document tests now carry an FR-286-AC-13 trace tag, and the current computed matrix lists that criterion as tagged with four binders. |

- FND-001 after_excerpt: `#[trace("TC-2951", "FR-867-AC-3", "FR-286-AC-13")] #[test] fn undefined_claim_item_serializes_location_and_cause()`
