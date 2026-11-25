---
id: SR-122
title: "Integrity analysis of FR-121 package identity"
type: SpecReview
analysis: integrity
scope: "spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md"
review_set: subset
---

## Summary

The edited package identity criterion is atomic and consistent with FSpec FR-322: the package's exact name is the identity, carried as `CallSite.package_id`, with no content-derived preimage. The existing acceptance-criteria matrix still has two unverified field-domain criteria.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The matrix has no tagged executable binder for the nested field-type traversal criterion and recursive-type key criterion; their verification cells name a test but do not currently bind to tagged tests. | FR-121-AC-28, FR-121-AC-29 | correct-requirement-no-evidence |
