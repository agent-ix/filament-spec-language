---
id: SR-2026111919340478491420
title: "Gap analysis for QSL-2184 PR #3147"
type: SpecReview
analysis: gap-analysis
scope: "spec/, fsl-replay/src/, fsl-replay/tests/, tests/"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: reviews
---

## Summary

This planless repository gap analysis used quoin 0.36.8 and quire 0.52.0 (engine 0.62.2) against the frozen PR head `2d73ff05bd45f4497c2a7c7a3f657f1ad4e3f117`. The changed behavior is traced: the native refusal writer's `name` field satisfies the strict native reader, while the monitor outcome retains FR-867's `function`, `expected`, and `actual` diagnostic detail. The changed integration test is tagged for FR-867 AC-1/AC-2, and the changed unit tests are tagged for FR-867 AC-4. No changed production behavior is untraced, and no stubs or placeholders were found in the changed production code.

Repository-wide matrix and coverage inventories also report substantial pre-existing trace gaps: 1,217 untagged criteria, 233 criteria tagged only by ignored tests, 104 method-without-symbol criteria, and 5,228 untracked trace tags. The tool output does not attribute these baseline gaps to this PR; none of the new test tags in the changed paths were untracked. No plan-completion analysis was performed. Semantic review was not requested and was skipped.

## Verdict

FAIL for the repository-wide gap-analysis method because matrix trace coverage and reverse inventory have the recorded baseline gaps. The reviewed PR itself introduces no untraced changed behavior and no new untracked test tags.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | high     | Repository-wide matrix reports 1,217 untagged criteria and 233 criteria supported only by ignored tests; changed PR criteria are tagged. These are existing repository baseline gaps, not introduced by this diff. | repository matrix |
| FND-002 | medium   | Coverage inventory reports 5,228 untracked trace tags repository-wide. Inspection of the changed paths found the new tags recognized; the reported inventory is not attributed to this diff. | repository coverage |

## Coverage

- Reconciliation: `quoin matrix --repo /home/peter/dev/worktrees/qsl-2184-result-wire --json` and `quire coverage --scope /home/peter/dev/worktrees/qsl-2184-result-wire --json`; quoin 0.36.8, quire 0.52.0 (engine 0.62.2). Both commands completed successfully. Matrix evidence store reported no run evidence for all 5,999 criteria.
- Matrix static statuses: 5,999 criteria total; 4,432 tagged; 233 tagged-by-ignored-test; 1,217 untagged; 104 method-without-symbol; 13 retired.
- Coverage inventory: backed 4,676 of 5,931; 5,910 criteria; 13 retired; 2,156 property-shaped and 753 specific-shaped; 5,228 untracked tags; 21,086 diagnostics; 105 oracle-resembles-implementation suspicions and 32 vacuous-under-guard suspicions.
- Changed criteria: FR-867 AC-1 and AC-2 are tagged by `tests/it/function_claim_selection.rs::a_non_boolean_function_claim_names_the_function_and_its_type`; AC-3 is an Inspection method without a test symbol; AC-4 is tagged by existing outcome tests and the changed clause round-trip tests. Added test tags `TC-2951` and `FR-867-AC-4` resolve in the matrix.
- Reverse-gap check for the changed production behavior: one changed behavior, zero untraced; no stubs or placeholders found in changed production code.
- Plan completion: not assessed (planless review).
- Semantic review: skipped (not opted in).
- PR base observed during review: `fc34f517c674a53df65d3f19fdd53bdf93303299`; `git diff --check origin/main...HEAD` passed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | The planner accepts the repository-wide matrix baseline and tracks remediation in QSL-2075 through QSL-2098. This is not PR-specific, and the changed FR-867 criteria are tagged; it does not block this diff. |
| FND-002 | accepted-no-change | The repository-wide untracked-tag count is baseline. The review found every new changed-path tag recognized and no PR-specific untracked tag, so no code change is needed for this diff. |
