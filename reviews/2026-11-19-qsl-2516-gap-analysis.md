---
id: SR-2026111919340478491681418
title: "Gap analysis — QSL-2516 monitor severity exit"
type: SpecReview
analysis: gap-analysis
scope: "Scoped PR #3124 review; FR-285-AC-2, TC-769 step 2; fsl-command/src/command/monitor.rs, fsl-command/src/command/output.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

This scoped gap analysis checks PR #3124's changed behavior against FR-285-AC-2 and TC-769 step 2. The requirement is tagged to the new monitor regression test and the source delegates the ranking to the shared catalog; no gap was found in this change.

## Verdict

**PASS** — within the requested PR scope, the corrected ranking has an owning requirement and trace-linked behavioral evidence for both reversed input orders.

## Coverage

- Reconciliation: `quoin matrix` (quoin 0.36.8, quire 0.52.0); FR-285-AC-2 is `tagged` with binders in `fsl-command/src/command/monitor.rs::tests::monitor_exit_uses_the_catalog_severity_order` and `fsl-foundation/src/diagnostic.rs::foundation_tests::tc_769_multi_item_outcomes_exit_with_the_most_severe_code`.
- Evidence: `no run evidence` in the Quoin evidence store for this criterion; the coder separately reported the focused monitor test passed 1/1.
- Plan completion: not assessed.
- Reverse gap / stubs: no new user-visible behavior or stub was introduced by the changed production code; the changed test setup remains assertion-bearing.
- Semantic review: skipped; it was not requested.
- Scope limit: this is a targeted PR/criterion review, not a repository-wide reverse-gap inventory.

## Findings

| ID      | Severity | Summary                    | Refs |
| ------- | -------- | -------------------------- | ---- |
| FND-001 | low      | No gaps found               | -    |
