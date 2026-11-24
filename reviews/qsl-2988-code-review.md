---
id: SR-967175026302197768900474181941935084813
title: "Code review — QSL-2988 FR-1042 claim-side mapping"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md; spec/test-cases/TC-3212-build-value-bounds-capabilities-in-conformance-order.md"
review_set: subset
---

## Summary

Reviewed the changed claim-side criterion and its test-case procedure against the computed matrix and implementation references. The changed criterion is intentionally untested by a live tagged test function at this spec-only head.

## Verdict

**CONDITIONAL** — FR-1042-AC-17 is not yet bound to an executable test in the computed matrix.

## Scope

- Examined: FR-1042-AC-17 and the TC-3212 procedure and expected-results additions.
- Context only: FR-1042's base/profile and claim-side rules; FR-201's source objective mapping and closed cyclic-MDP profile; `fsl-analyze/tests/it/cyclic_state_certificate.rs`'s objective-mismatch cases.
- The change adds no implementation code. No builds or test suites were run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The matrix reports FR-1042-AC-17 as untagged. TC-3212 now describes the required Maximum claim-side acceptance and Minimum objective-mismatch case, but no executable test function currently carries the criterion tag. Add the tagged assertion when the implementation test lands so this new acceptance/refusal contract is covered. | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:258 |


## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | The tagged test exists in downstream PR #3672, `tools/fsl-probability-check/tests/it/value_bounds.rs`, but its old expected assertion must be updated to the corrected FR-1042-AC-17 contract after this specification merges. The code change is owned by QSL-2988/PR #3672 and is outside PR #3689. |
