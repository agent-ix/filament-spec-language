---
id: SR-2026112116405040524102
title: "Spec integrity (narrow addendum) — PR #3307 FR-390-AC-75 and TC-1070 Rational reader vectors"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@d4f92088ca14fd09ff626c7b8a761b62dd01ae95 (delta from e1900638); spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md (AC-75 row), spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md (scope line, section 'Rational literal and division reader vectors')"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-390
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-1070
    type: reviews
---
# SR-2026112116405040524102: Spec integrity — PR #3307 Markdown delta

## Summary

Ticket: QSL-2271 (PR filament-spec-language#3307). This is a narrow spec-integrity addendum covering
only the Markdown delta of fix round 1: the new FR-390-AC-75 row, the TC-1070 scope line, and the
new TC-1070 section "Rational literal and division reader vectors" (rows R1 to R4). It is not a
review of the whole FR-390 catalog. Result: clean.

## Verdict

**PASS**: nothing qualifies as a finding. I checked:

- **Completeness and trace.** AC-75 names a verification method, `Test (TC-1070)`. TC-1070 already
  `verifies` FR-390, and its scope line now adds AC-75. `quire trace --id FR-390-AC-75` resolves 4
  claims, which are exactly the four tests. FR-390-AC-28 is back to 0 claims, with no inflation.
- **Consistency.** AC-75 bounds itself to the one slot (`Refunds.activation.guard`, fixture E) and
  the four expressions, and it explicitly does not extend AC-28. That removes any overlap with
  AC-28's listed slots. Neither AC-1, which expects TC-1070's unmodified values, nor AC-43, which
  covers E-C integer arithmetic, is contradicted. The TC-1070 rows R1 to R4 match the declarations
  and guards of the four tests verbatim, along with the supply `Tick(ready true, count 2)`, the added
  profile `v`, and R4's profile `d`. The statement "R2 and R3 share `filament.op.rational.div` and are
  told apart by operand type" matches the source: lowering.rs:4812-4825 emits both as the same
  identity, and the reader arm at reader_protocol_projection.rs:2541-2577 dispatches by operand type.
- **Atomicity and testability.** AC-75 states one obligation, source/reader parity of value,
  outcome, losses and charge log, over a closed set of four vectors. That is the same shape as the
  sibling criteria. Each vector carries an oracle that can fail: R1 fails if 1/2 is truncated or
  rounded, R2 and R3 fail if the result is not 1/2, and R4 fails on `-7 div 2` if the law is not
  truncating. The TC requires the source route to complete `true` before the reader view is built,
  so the test cannot pass with a vacuous `false` on both routes.
- **Scope.** No new behavior and no new FR or contract policy: AC-75 verifies FR-390's existing
  statement that the reader view gives the same `ProtocolSlotEvaluation` and charge log.

`quire validate` on both spec files exits 0. Failure-domain cross-check: no identity-key, purity or
extension issue is introduced, because the vectors are pure functions over a fixed supply.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low      | No findings (placeholder) | - |
