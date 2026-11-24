---
id: SR-967175026302197768900474181941935084819
title: "Base review of QSL-2968 FR-600 grammar inventory"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-600-keep-the-parser-grammar-in-agreement-with-the-standard-grammar.md; spec/test-cases/TC-1500-parser-and-standard-grammar-production-inventories-agree.md"
review_set: subset
---

## Summary

The updated criterion covers the FR-375 local grammar, and the test case now exercises the local missing/stale correspondence paths. One exact failure detail is still underspecified in the source-absent case, and AC-4 awaits a downstream executable binder.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1500 Step 4 requires failure naming the unpublished FR-375 source path, but FR-600-AC-4 also requires the failure name the affected production. State the production (for example `history-update`) in Step 4's expected result. | spec/test-cases/TC-1500-parser-and-standard-grammar-production-inventories-agree.md |
| FND-002 | medium | Quire's computed matrix reports FR-600-AC-4 untagged. The spec PR adds the criterion, but the downstream implementation must add and trace the executable FR-375 inventory check to TC-1500/AC-4. | FR-600-AC-4; TC-1500 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | deferred | The executable FR-600-AC-4 binder belongs to the downstream code implementation; this PR is the spec-only change. |
