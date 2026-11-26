---
id: SR-34921
title: "QSL-3492 gap analysis of filament-spec-language#4060"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@a5695763d68a7aba58b7f9edc110d7e9bde606bb; FR-125-AC-7, FR-125-AC-13, FR-232-AC-3, FR-239-AC-5, FR-240-AC-4, FR-632-AC-2; added and changed tests and implements marks of the four authored commits"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-632
    type: references
---

## Summary

Ticket: QSL-3492. Scoped to the PR's criteria with quire trace; no tests were run. Semantic review was done by hand for the added tests.

## Verdict

CONDITIONAL: trace tags that claim more than their tests assert.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Tests tagged FR-632-AC-2 and TC-1592 assert neither: one never runs EN-6, and neither uses the claims or procedure those ids state | fsl-analyze/tests/it/analyze_timed_cli.rs:1276-1292 |
| FND-002 | low | The changed empty-universe test keeps the TC-1170 tag, whose procedure and scope (FR-430) have no empty-universe step | fsl-analyze/tests/it/zone_dense.rs:585-590 |

## Coverage

Plan completion: not assessed. implements marks: claim_start (FR-125-AC-7, AC-13), admitted_start (FR-125-AC-13, FR-232-AC-3, FR-240-AC-4), admitted_refutation (FR-239-AC-5), Digital::decide, check_timed_model and check_zone_certificate_with_time_source (FR-125-AC-7) match their bodies. Every criterion in scope has a tagged test. Expected values of the added tests were re-derived by hand from their models and agree. Missing cases are in the code review (SR-34920): a time-lock reached after a delay, and fairness with no fair behaviour.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The new liveness-product route for fair TT-1 and TT-2 claims has no test of a refutation, of a TT-2 window past vacuity, or of the symmetry refusal | fsl-analyze/tests/it/analyze_timed_cli.rs:1456-1520 |

The new tests settle fair claims by vacuity (decided before any claim product) and one TT-1 proof. FR-239 now requires every fair claim to be decided on the liveness product, whatever its form; no timed test elsewhere declares a fairness on an always or windowed claim.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The FR-632-AC-2 and TC-1592 tags are removed |
| FND-002 | fixed | The TC-1170 tag is removed |
| FND-003 | still-open: no fix landed | Found this pass |
