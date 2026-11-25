---
id: SR-847
title: "Gap analysis of FSL QSL-3188 PR #3845"
type: SpecReview
analysis: gap-analysis
scope: "QSL-3188 changed paths and repository-wide computed matrix"
review_set: subset
relationships:
  - target: "ix://agent-ix/filament-spec-language"
    type: references
---

## Summary

The engine-backed matrix was computed with `quire matrix --scope /mnt/build/dev/worktrees/q-3845 --format json` because `quoin matrix --repo ... --json` refused the request size. It reports 1,086 untagged criteria, including live criterion FR-002-AC-9, and 24 criteria tagged only by ignored tests; the two newly added tests' FSpec ids are also not recognized as trace tags. Reverse-gap, stubs, and inflation were inspected for the changed paths and their directly relevant emitter/checker behavior; the whole-tree reverse-surface inventory was not completed, so this is not a full repository reverse-gap clearance.

## Verdict

FAIL. The computed matrix contains untagged live criteria, and the changed tests have untracked FSpec tags. The matrix is repository-wide; the focused reverse-gap inspection found no additional changed-path issue beyond the findings recorded in the code review. Plan completion: not assessed

## Coverage

- Plan mode: planless; Plan completion: not assessed.
- Matrix: engine-backed `quire 0.52.1`; output `/mnt/build/tmp/qsl-3188-review/quire-matrix.json`; 5,483 tagged, 1,086 untagged, 24 tagged-by-ignored-test, 109 method-without-symbol, 15 retired.
- Coverage scan: `/mnt/build/tmp/qsl-3188-review/coverage.json`; 6,511 untracked symbols repo-wide. Reverse-gap inventory and stub scan were limited to changed paths and directly relevant code; the complete source-tree inventory remains incomplete.
- Semantic review: skipped (not opted in).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed matrix leaves live test-method criteria untagged; for example FR-002-AC-9 requires a recovered parse and neighboring declaration preservation but has no bound test. | spec/functional/FR-002-parse-native-units.md:FR-002-AC-9; quire-matrix.json |
| FND-002 | medium | The two new mutual-recursion tests' intended FSpec trace bindings are absent from the computed matrix because the tags omit the required `FSpec-` prefix. | fsl-package/src/emit/value_validity_requirements/tests.rs:643,757; AGENTS.md |
