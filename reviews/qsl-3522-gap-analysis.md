---
id: SR-9893
title: "Gap analysis of PR #4059: QSL-3522 deeply nested request stack overflow"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@0955f2720ebb46b2513e05beb9d1eda715358b6f; fsl-command/src/unique_json.rs, fsl-command/src/replay_request.rs, fsl-command/tests/it/export_selection.rs, spec/functional/FR-026-run-standalone-native-workflow.md, spec/decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md"
review_set: subset
---
# SR-9893: gap analysis of PR #4059

## Summary

Ticket: QSL-3522. Scoped to the changed code and the criteria it touches.

## Verdict

**CONDITIONAL** - the behaviour is owned (ADR-030 D-1, FR-026-AC-3 for the run request) and both
`deeply_nested` tests carry trace attributes. One trace tag is incomplete (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new test also drives `replay_with_limits` on `native-replay/1`, but is tagged only FR-026-AC-3, which is about a `1-draft` run request. The replay request refusal belongs to FR-862 | fsl-command/tests/it/export_selection.rs:213 |

## Coverage

Plan completion: not assessed. `quoin matrix` failed with `CORE_REFUSED: request exceeds the
accepted size`; `quire matrix` and `quire trace` were used instead. Changed production functions
(`unique_json::read`, `Deep::into_inner`, `parse_unique_json`) carry no `implements` marker; the
PR says none implements a criterion it can name, which is accepted. No dependency version moved.
No stubs in the diff. Semantic review was not run.
