---
id: SR-202611160951003
title: "EARS review of the QSL-2002 exact mod interval contract"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; PR #2745 at f3fcd3fc18f1f9277432fef2f59ce4f929669f09, diff origin/main...HEAD: FR-495, FR-062, FR-093"
review_set: subset
relationships:
  - { target: ix://agent-ix/filament-spec-language/FR-495, type: references }
  - { target: ix://agent-ix/filament-spec-language/FR-062, type: references }
  - { target: ix://agent-ix/filament-spec-language/FR-093, type: references }
---

## Summary

Ticket: QSL-2002. This lens checks the changed requirement statements against the EARS patterns. `quire validate` reports no grammar warning on the changed files.

FR-062's new sentence ("The `ValueType` and the type node SHALL name the same type") is a well-formed ubiquitous statement. The FR-495 "Type of an integer division node" bullet keeps its SHALL, and its added sentences sit under that SHALL. FR-093's additions explain the FR-495 rule and reference it, which is acceptable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new FR-495 bullet "Representation of an integer division result" makes the normative contract for the lowered node, the narrow, the positions with no narrow and the record, but uses no SHALL. Its sub-bullets read as description: "lowers", "`check` wraps", "adds no `Coerce`", "has, as its result bound". Every other behavior bullet of FR-495 uses SHALL. Fix: state each sub-bullet as a SHALL with its actor, for example "`check` SHALL wrap ...", "FR-093 SHALL lower ...", "the record SHALL have ...". | FR-495:216-244 |

## Verdict

REQUEST CHANGES for this lens, a medium finding only.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
