---
id: SR-202611222359110002
title: "Base spec review of QSL-2882 typed clause-run arguments"
type: SpecReview
analysis: base
scope: "FR-109 and FR-312; PR #3526"
review_set: subset
---

## Summary

FAIL: the new FR-312 criterion is testable, but no test case or trace binding owns it. The edited requirements otherwise use the canonical Reference and Population forms consistently with FR-181 and FR-650.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-312-AC-11 has Verification `Test` but names no test case, and no test is tagged to it. Existing TC-1706 scopes FR-650-AC-7 and covers identity/key references plus Text; it does not specify the new public canonical Reference plus Population clause-request cases. Add the new cases and FR-312-AC-11 to a test case (for example, extend TC-1706) so the eventual FSL test can bind the criterion. | FR-312-AC-11, TC-1706 | correct-requirement-no-evidence |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-1706 now scopes and specifies FR-312-AC-11, including the future source trace tags; code tagging remains in the explicitly deferred FSL implementation. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      The values use FR-100's public command encoding, and each request's result equals the report for the same admitted `ClauseRunRequest` built directly. | Test (TC-1706) |
```
