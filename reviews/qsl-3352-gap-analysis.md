---
id: SR-2026112520490002
title: "Gap analysis — filament-spec-language partial measure lower proof"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3970 changed criteria FR-1042-AC-10, FR-1042-AC-27, FSpec-FR-411-AC-8, FSpec-FR-412-AC-5, FSpec-FR-412-AC-16, FR-1042-AC-11"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-1042", type: references }
---

## Summary

Ticket: QSL-3352. PR-scoped gap audit at head `0b64d104a3b74c1377c7bb1dd777cb5b2397c3c9` using `quire trace --id` per changed criterion (static, no run evidence read). The repository-wide matrix and reverse-gap inventory were not run; this is not a full repository gap audit.

## Verdict

**PASS** — every criterion the PR claims has a tagged test; FR-1042-AC-11 is deliberately not claimed and stays open on QSL-3352.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- FR-1042-AC-27: verifies `value_bounds.rs::a_claim_side_measure_bound_of_the_wrong_kind_is_rejected`; implements `measure.rs::check`.
- FSpec-FR-412-AC-5: verifies four tests incl. the two new partial measure tests; implements `whole_or_lower_part`.
- FSpec-FR-411-AC-8: verifies `a_partial_shifted_proof_writes_a_schema_valid_basis`, `a_measure_prefix_that_settles_nothing_stops_at_max_states`, `a_measure_probability_is_proved_on_a_partial_product`, `a_partial_shifted_lower_over_the_named_states_refuses_a_false_document`.
- FR-1042-AC-10 / FSpec-FR-412-AC-16: `a_partial_shifted_lower_naming_the_initial_key_counts_the_rest_at_minus_the_level` on the `fast`/`slow` model the ACs name.
- FR-1042-AC-11: not tagged (cyclic reward chain and timed `MeanTime` clauses remain; QSL-3352).
- Changed-source reverse check: no stubs, no untraced new production behaviour.
- Plan completion: not assessed
- Semantic review: skipped (not opted into)
