---
id: SR-2026112115155546054652102
title: "spec-review/integrity of the QSL-2247 TP-3 fix-round addendum (FR-338-AC-29, FR-1030 clarification)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-338-check-an-en-1-closure-certificate.md (new AC-29 row only); spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md (statement paragraph L133-143 only); fix commit 7eaa8d52dd..943172bbeda2f23bd42754443278491dae142602; PR agent-ix/filament-spec-language#3330; QSL-2247"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: reviews
---

## Summary

Ticket: QSL-2247. This addendum review covers only the Markdown delta of fix round 1 on PR #3330, at head 943172bb: one new acceptance row, FR-338-AC-29, and the rewritten FR-1030 statement paragraph. It is not a whole-FR review.

Methods:
- **Consistency:** the delta is checked against the surrounding FR-338 statement and the existing FR-1030 ACs.
- **Atomicity and testability:** each claim in AC-29 is matched against its tagged tests.
- **Completeness:** both changes carry a verification method (AC-29 `Test (TC-525)`), and FR-1030's ACs are unchanged.

The user and architecture approved the existing TP-3 contract. The delta adds no new normative policy, which was confirmed: AC-29 restates behaviour the code already has, and the FR-1030 text narrows the remaining-scope sentence without weakening any AC.

Clean:
- FR-1030's new paragraph is consistent with FR-1030-AC-1 (PrevSafe and SoonTwo `proved`) and AC-5 (PrevStutter `refuted`). It names the uncertified classes (`on each`, protocol), keeps `ClosureStutterLoop`, and leaves QSL-2247 open for them and for closure-only loop replay.
- AC-29's positive bodies (`history` offsets `["1"]`/`[]` and deadline `"2"`/`"1"`) follow FR-338's "other finite past interval" row and the window form. They are pinned by literal-byte tests, not by kernel round-trips.

Atomicity: AC-29 bundles a positive vector, key bodies and several negatives in one row. That matches the existing FR-338 AC style (AC-10, AC-28), so it is not raised.

The protocol-subject testability gap in AC-29's last sentence is recorded once, as gap-analysis SR-2026112115155546054652101 FND-003, and not repeated here.

## Verdict

**CONDITIONAL.** One medium consistency finding. AC-29 extends the rejecting-key locus to an undefined origin letter without the empty-certificate qualifier that the FR-338 statement sets. Qualify the sentence to match what the test asserts; no code change is needed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-29 says "a certificate offered for a clause violated at the origin, an undefined letter included, is rejected `BadState` at the origin under `["safety",[]]`". FR-338's statement fixes the undefined-letter locus differently. For an initial model state it is "the least certificate state carrying that model state, or, when none carries it, the least certificate state". Only "for an empty certificate" is it the item context's rejecting key, `K_C(["safety",[]])` for TP-1 and TP-3. As written, AC-29 contradicts the statement for any non-empty certificate whose origin letter is undefined. The tagged test (`a_certificate_for_a_violated_tp3_clause_is_rejected_bad_state`) uses only the empty certificate. Fix: qualify the clause as "an empty certificate offered for a clause whose origin letter is undefined". The defined-letter origin and later-violation clauses are unaffected, because they reach a real rejecting monitor state. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:1127; spec/functional/FR-338-check-an-en-1-closure-certificate.md:644-657; fsl-analyze/tests/it/closure_certificate.rs:900 |

## Scope

- FR-338-AC-29 (examined; FND-001)
- FR-338 statement, undefined-letter locus paragraph L644-657 (context_only)
- FR-338 state-algebra table, history and window rows (context_only)
- FR-1030 statement paragraph L133-143 (examined, clean)
- FR-1030-AC-1, AC-5 (context_only)

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 2 (head 8ccf5300). AC-29 now says "the empty certificate offered for a clause violated at the origin, or whose atom is undefined there, is rejected `BadState` at the origin under `["safety",[]]`". This matches the statement's empty-certificate rule and the tests. Its later clause, "violated later, or whose atom is undefined at a later model state … at that model state under `["safety",[]]`", matches the round-2 code and the fourth vector. It does not conflict with the statement, because the statement's predecessor-locus sentence governs a `when` condition, which an `on origin` TP-3 clause does not have. No new policy. |
