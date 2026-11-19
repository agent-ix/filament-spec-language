---
id: SR-2026111518064800000002
title: "gap-analysis review of PR2720 acquire/require unmangle"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-485, FR-492, TC-1263, TC-1275 and their tagged tests touched by PR2720 c216d31b3f0d4a7c65f6109f6c4e0f44a396c725"
review_set: subset
---

## Summary

Ticket: QSL-2250. Scoped gap check of the renamed symbols against the spec text and the tagged tests. Plan completion: not assessed. Full quoin matrix run skipped: a symbol rename cannot change criterion coverage, and the reviewer brief excludes the aggregate gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope and evidence

- FR-485 Inputs and AC list item 1 name acquire_checked_closure, matching fsl-replay/src/application.rs:787 and the lib.rs re-export.
- FR-492 Inputs and FR-492-AC-1 name acquire_checked_closure; test with_closure in fsl-replay/tests/it/refinement_query_facade.rs uses it; tag #[trace("TC-1275", "FR-492-AC-1")] at line 96 unchanged and correct.
- TC-1275 test layer text names acquire_checked_closure; matches the test file.
- fsl-replay/tests/checked_nominal_declaration.rs #[trace("TC-1263", "FR-485-AC-13")] unchanged; test calls acquire_checked_closure.
- No trace tag, AC id or TC id was renamed; no stub or coverage change.

## Verdict

Clean. Spec, code and tests agree on the restored names.
