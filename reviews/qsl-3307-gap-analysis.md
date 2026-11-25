---
id: SR-202611251158302
title: "gap-analysis of PR 3915 (QSL-3307) unbounded threshold reporting"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3915 head 9ef3ee9a1540e203180e670bcef460a3d6687f00; FR-198, FR-1042, FR-201-AC-65, FR-202-AC-20, ADR-028 UR-4/LM-1/LM-2/XV-1"
review_set: subset
---

## Summary

Ticket: QSL-3307. Planless gap analysis scoped to the criteria this PR touches, using the computed matrix (`quire matrix --scope . --format json` at the PR head). Semantic review was not run as a separate fan-out; the code-review artifact SR-202611251158301 covers intent against code for the changed paths.

## Verdict

CONDITIONAL. FR-198-AC-3, FR-198-AC-4, FR-201-AC-65, FR-202-AC-20 and FR-1042-AC-21 are all tagged in the computed matrix. No stub or hollow evidence was found in the diff. The coder's reported spec gap is confirmed and recorded below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Spec gap: FR-198's Result rules, FR-1042's Inputs and ADR-028 XV-1 do not say how a UR-4 policy-iteration value longer than max_rational_bits is reported. They do not say which length measure applies (per part, as in FR-1042 Behavior, FR-1042-AC-15 and FR-197, or the one-scalar numerator+denominator+1 that exact.rs uses). They do not name a Bounds method for an outward-rounded exact value; FR-1042 Inputs name only backward induction and interval iteration, so the code labels it IntervalIteration with the earlier phase's sweeps. They do not say what settles when the rounded value cannot separate the threshold (LM-2 PrecisionBudget, or ExactValue). | spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md; spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:52-57 |

## Coverage

- Matrix: FR-198-AC-3 tagged, FR-198-AC-4 tagged, FR-201-AC-65 tagged, FR-202-AC-20 tagged (includes actual_s3_over_parameter_and_single_admitted_instance_drive_replay), FR-1042-AC-21 tagged.
- Reverse gap: none new. The new helper exceeds_rational_budget is extracted from existing exact.rs code.
- Stubs: none.
- Semantic review: skipped as a separate pass; covered for the changed paths by SR-202611251158301.
- Plan completion: not assessed
