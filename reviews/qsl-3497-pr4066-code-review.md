---
id: SR-967195
title: "code review of PR 4066 spec changes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; spec/functional/FR-201; spec/test-cases/TC-1401..TC-1406, TC-3190"
review_set: subset
---

## Summary

Ticket: QSL-3497. PR 4066 at 8ae7ff9a03772b5bd38d40fa07eebc753885c103. 1 finding.

## Verdict

**FAIL**: one duplication finding. No hash, pin or tracking record is added. No code changes.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | One rule (no admitted instance gives NoInstance) is restated in seven FR-201 profile sections (574, 1168, 1270, 1335, 1447, 1564, 1727), six ACs and seven TCs, although FR-125 owns it and FR-201 'Instancing outcomes' and FR-203-AC-34/35 already state it. This change had to edit every copy, and the copies already differ from the owner (they drop the non-empty initial list condition). Delete the per-profile body sentences and let 'Instancing outcomes' state it once. The copies predate this PR. Severity is the code-review skill's fixed rating for duplication. | spec/functional/FR-201-check-a-probability-certificate.md:574-578 | wrong-requirement |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The seven per-profile copies are deleted; the rule is stated once under 'Instancing outcomes'; ACs and TCs refer to it. |
