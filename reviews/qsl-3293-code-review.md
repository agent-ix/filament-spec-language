---
id: SR-967178
title: PR 3897 red-main probabilistic settlements code-review
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-3293; fsl-analyze/src/analyze/exact.rs; fsl-replay/src/probability_certificate/document/timed.rs; fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs; fsl-analyze/tests/it/long_run_certificate.rs"
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-203
  type: references
- target: ix://agent-ix/filament-spec-language/FR-201
  type: references
- target: ix://agent-ix/filament-spec-language/FR-200
  type: references
- target: ix://agent-ix/filament-spec-language/FR-921
  type: references
---

## Summary

Ticket: QSL-3293. PR: filament-spec-language#3897. Head 8b24beca8885dfc2354fe0041ef3efe2539eb0f6, scope `git diff origin/main...HEAD` (merge base ab4753e6b438797ac9e807e6ccbbebf927eebdf8). Partial delivery: 9 of 29 red tests; 17 timed_product_certificate and 3 free_delays wait on QSL-3281 (FSL #3902). Rust lane (rust-review) folded in. No tests were run: MemAvailable was 28.9 GB inside the lead-prob-cargo-C flock, below the 30 GB gate, so the review is from source only.

Spec checks for each test the coder called stale:
- subsystem_refutation long-run bound: FSpec FR-413 "Claim bound" requires `{"type":"long-run-fraction","comparison":"greater-or-equal","threshold":θ}` (FR-413-AC-17). The new assertion is the spec shape; the old `{comparison: ">="}` was stale.
- Weak (toggling, fair weak repair): the deterministic optimum (fail forever) is unfair, so FR-200 Behavior "LongRunFraction" and ADR-028 FS-9 require the 2^-k randomized witness. k=1 gives exactly 1/2 (not past `>= 1/2`); k=2 gives stationary up-fraction 1/4, recomputed by hand. The old "no witness, not refuted" assertion contradicted the spec.
- FairFails (architecture, fair weak Service::call, whole): here the deterministic optimum call(false) is already fair, so a deterministic witness of value 0 would be valid evidence. FR-200 nonetheless says "For a refutation it SHALL build a randomized witness that takes the actions the optimum leaves out with probability 2^-k, starting at k = 1", with no exception for an attained fair optimum, and FR-413 and ADR-028 FS-9 say the same. call(true) is left out by the optimum at both states, and k=2 / 1/4 checks out. The new assertion follows the spec text; production is not wrong. The "need not be attained" wording in FS-8/FS-9 gives the reason for the rule; it does not limit it.
- long_run `using ev`: FR-201-AC-30 says a workload `using ev` claim "no longer settle[s] unsupported" and settles through the DTMC profile; FR-201-AC-74 says it gives the same values as `using inf`. Service cycle = 1 + 1 + 2/100 + 1 = 151/50 positions, phase 0 once, so 50/151 < 1/2 refutes. The new assertion is the spec. No production commit is pinned, but production already behaves to the spec.

Production fixes checked:
- timed.rs: removing the row-count/foreign-row ObjectiveMismatch check lets `certificate_values` (allow_partial_lower=false) reject a missing row, an extra or foreign key, or a wrong-cap key as `Malformed` at `values`, which is what FR-921-AC-2 and AC-3 require.
- max_states_limit (both sites): `ProbBuild::Partial` is produced only when `max_states` stopped admission. FR-203 Budgets requires `Incomplete(LimitReached{max_states, value, exact_prob.max_states})`; the old not_yet/unsupported was wrong. The value conversion saturates.
- bounded_formula filter revert: this restores the pre-897e1eac03 routing. A formula the qualified monitor does not admit now reaches the existing guard at exact.rs:1452 and settles not_yet before any product work (FR-201-AC-62 nested/bounded forms "issue no proof"). It no longer falls into the unbounded route, where it produced runtime_invariant. `supports_observations` admits only the event-position profile, so non-`ev` formula events (other than unbounded eventually, which is routed separately) now settle unsupported, as they did before 897e. That matches AC-62.
- Cyclic fallback (FR-201-AC-65): on MaxRationalBits for an until or always subject, it re-admits ModelSystem and runs the existing acyclicity admission. A cyclic subject settles LimitReached{MaxRationalBits, caller value} with no FR-1042 document; an acyclic subject keeps the interval route. Cancellation is polled after admission, and enumeration polls inside. Errors map through document_failure (Cancelled, Limit and Accounting keep their causes). Enumeration is bounded by max_states, max_transitions and storage_bytes. The scalar cap is lifted to u64::MAX, which is safe because edge probabilities come from finite source weights. Nothing is charged twice: the enumeration runs on a fresh meter (see FND-001).

No assertion was weakened. Each changed test asserts the spec settlement, and the fair tests went from `assert_ne!(Refuted)` to a full witness, document and replay check.

## Verdict

PASS with low findings. Mergeable at 8b24beca8885dfc2354fe0041ef3efe2539eb0f6 as a partial delivery. QSL-3293 stays open until FSL #3902 lands and the 20 remaining tests are re-run. Tests were not re-run by the reviewer (memory gate); the PR body reports 361 passed / 27 failed, with every failure attributed to QSL-3281, QSL-3292 or failures already red on main.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The cyclic-subject fallback re-admits ModelSystem and enumerates the whole model graph under a fresh Meter built from system.meter_limits(), so this work is not charged to the item's caller meter (FR-203 cumulative allowance). An item can spend up to one extra meter allowance past the caller's budget. This repeats the existing call at exact.rs:2222. | fsl-analyze/src/analyze/exact.rs:1737-1760; fsl-replay/src/probability_certificate/cyclic.rs:39 |
| FND-002 | low | No test reaches the new fallback branches other than Ok(false): cancel after re-admission, admission refusal, document_failure from the shape enumeration, and the Ok(true) acyclic continuation under a small max_rational_bits. | fsl-analyze/src/analyze/exact.rs:1736-1759 |
| FND-003 | low | The test finite_profile_claims_issue_no_long_run_document now asserts that both profiles refute with a long-run subsystem certificate. Its name and the claim it states no longer describe what it checks (it issues no proof document, but it does issue a long-run document). | fsl-analyze/tests/it/long_run_certificate.rs:640 |
