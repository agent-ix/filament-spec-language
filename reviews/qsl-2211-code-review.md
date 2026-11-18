---
id: SR-2026111612172400000042
title: "Code review — QSL-2211 observe-only protocol boundaries"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2932 at c6746094bf0273300f29ab67d0bdd8bc8a9d5c80; fsl-forms/src/protocol_control.rs; fsl-forms/src/protocol_clause.rs; FR-218-AC-45; FR-206-AC-13"
review_set: subset
---

## Summary

Reviewed the QSL-2211 diff at the frozen head. The changed semantics and evaluator focused tests pass, but contextual marker detection in the forms layer treats identifier spellings as marker keywords regardless of grammar position, breaking valid names `observe` and `recommend`.

## Verdict

**FIX ROUND REQUIRED** — FND-001 is a reproducible form-building correctness issue. The package-reader failure is a separate known QSL-2449 dependency blocker and remains an independent merge gate; no workaround is appropriate in this PR.

## Acceptance mapping

| Unit | Evidence |
| --- | --- |
| FR-218-AC-45 | The checker test covers marked role/control, capture named `observe`, and basic recommendation. The form builders still mishandle `observe` used as the role's declared name and `recommend` used as an event/recommendation name. |
| FR-206-AC-13 | The evaluator test checks recommendation metadata and the `semantic` state-key member across recommendation/receive transitions. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Contextual marker detection consumes ordinary identifier names spelled `observe` or `recommend`, causing valid role/event declarations to be rejected or misclassified. | fsl-forms/src/protocol_control.rs:792-803; fsl-forms/src/protocol_clause.rs:1162-1173,1240 |

## Rust review

The Rust review found no additional ownership, panic, unsafe-code, concurrency, or resource-management issue in the changed code. The targeted semantic and evaluator tests passed as reported; the strict package-reader test fails at `/semantic_graph/nodes/3` with the known contract-ir v0.1.4 `MalformedWire` associated with QSL-2449. Changed-file rustfmt check passed. No aggregate gate was run.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The fix uses role-prefix position for the role name and recognizes `recommend` only as the first event-form token; regression coverage exercises role `observe`, event `recommend`, and recommendation `recommend`. |

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified closed at 3bd07b0; the pin-only delta leaves the position-aware role and recommendation parsing unchanged. |

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified closed at acfc0bc; the main rebase and evaluator fallback cleanup do not alter contextual role/event parsing. |
