---
id: SR-121
title: "Base review of FR-121 package identity"
type: SpecReview
analysis: base
scope: "spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md"
review_set: subset
---

## Summary

FR-121's IDs and references are structurally valid, and the package identity correction now follows the checked-package contract without a content-derived identity. The base coverage check found two existing criteria without tagged executable evidence.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The computed matrix reports no tagged binder for the nested record/tuple/union field-key criterion or recursive-key criterion. Add owning executable evidence or disclose these as coverage gaps. | FR-121-AC-28, FR-121-AC-29 | correct-requirement-no-evidence |
