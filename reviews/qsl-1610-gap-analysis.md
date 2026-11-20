---
id: SR-2026112001423700000002
title: "Gap analysis — QSL-1610 temporal wire reader"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; spec/, fsl-replay/src/spine/clause/tests/temporal/fspec_kinds.rs, fsl-replay/src/spine/clause/tests/temporal/wire.rs, fsl-replay/src/spine/native_result/temporal.rs; QSL-1610"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-267", type: references }
---

## Summary

The repository-driven matrix for filament-spec-language reports 1,238 untagged criteria (4,386 tagged, 251 tagged only by ignored tests, 104 method-without-symbol, 13 retired), so repository assurance does not pass. The changed FR-267-AC-20 has three tagged binders, and review of the changed reader/test delta found no unowned behavior or source/test stubs; matrix reconciliation could not join the referenced FSpec repository because the installed matrix CLI rejects multiple `--repo` arguments.

## Verdict

**FAIL** — the computed repository matrix contains untagged criteria; the cross-repository FSpec trace tags could not be reconciled in a two-repository matrix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The engine-backed repository matrix reports 1,238 untagged criteria. The target FR-267-AC-20 is tagged, but the broad repository audit cannot establish full criterion coverage. | FR-001-AC-7; FR-267-AC-20 |

## Coverage

- Reconciliation: `quire matrix --scope . --format json` (`quire` 0.36.7; matrix engine 0.62.2); engine-backed result, no run evidence read.
- Plan completion: not assessed
- Criteria: tagged 4,386 · untagged 1,238 · tagged-by-ignored-test 251 · method-without-symbol 104 · retired 13.
- Evidence: no run evidence.
- Untraced behavior / stubs in changed delta: 3 files inventoried; 0 unowned behaviors found; 0 source stubs; 0 test stubs.
- Semantic review: skipped (not requested).
- Matrix limitation: `quoin matrix --repo agent-ix/filament-spec-language=<path> --repo agent-ix/filament-specification=<path> --json` exits 2 because this installed CLI rejects repeated `--repo`; a single-repository matrix cannot resolve the FSpec trace tags.
