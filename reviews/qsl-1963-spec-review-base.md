---
id: SR-202611141459001
title: "Base review of the QSL-1963 FR-495 Kernel-mode criteria"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2707 at 3b0bc910d964198a45d79d8f7c6f4489b93fc179, diff origin/main...HEAD: spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md (Kernel boundaries bullet, FR-495-AC-10..13), spec/test-cases/TC-1419-kernel-mode-drops-a-function-body-s-presence-obligation-and-nothing-else.md, spec/test-cases/TC-1420-kernel-mode-still-refuses-a-malformed-rational-shape-and-dispatch-declaration.md"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-495, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-104, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1419, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1420, type: references }
---

## Summary

Ticket: QSL-1963. The criteria describe the `CheckMode` behaviour PR #2706 (QSL-1931, branch
`qsl-1931-checkmode` at 8478f1759e98aaa0e60d7779f59d0dd43099377f) implements. Each claim was
checked by reading that branch's code and tests, read-only; no cargo run was needed.

Code read:
- `qsl-semantics/src/check/facts.rs`: `Definedness::discharge` (~1197) returns the malformed
  Rational shape refusal under Kernel before the per-node discharge, then drops any
  `CheckCause::Unproved(_)` when `kernel` is set. The walk is iterative (`quire_walk`), so a
  dropped obligation does not stop the walk, and call sites are still recorded.
- `qsl-semantics/src/check/family.rs` (~1607-1626): the body walk gets `with_check_mode(mode)`.
  The `decreases` measure is walked by a fresh `Definedness` with no mode, so it is checked as
  under Linked. Termination (`UnprovedDecrease`) is a separate phase that takes no mode.
- `qsl-semantics/src/check/mod.rs`: the dispatch refusals `TableOutOfRange` (~2672) and
  `FunctionOutOfRange` (`validate_dispatch_function`, ~1686) are made up front, before any
  body walk, whatever the mode. `CheckedGraph::check_clause_expression_inner` (~8609) now
  passes `mode` to `Definedness::with_check_mode`.
- `qsl-semantics/src/check/refusal.rs` (~3435, ~3518-3529): `Unproved(Range | RationalRange
  | NonemptyReduction)` renders `undefined_expression`/`unproved-range`, `Unproved(Presence)`
  `unproved-presence`, `UnprovedDecrease` `unproved-decrease`, and
  `InvalidDispatchDeclaration` `invalid_package`/`invalid-value`.
- `qsl-semantics/src/check/check.rs`: the Typer builds `NodeKind::Rational` (~4026-4038) only
  for two `Rational` operands and `NodeKind::RationalNegate` (~4166-4177) only for a
  `Rational` operand. No other code in `qsl-semantics` builds either node.
- `qsl-semantics/tests/it/check_mode.rs`: tests cover `p` (Linked refusal, Kernel check),
  `NoGuard` (one presence refusal in each mode), `w`, `f(x)` (`missing-measure`, path `[]`),
  and `f(x) + 1` (one `Range` refusal under Linked, `missing-measure` at path `[0]` under
  Kernel).

Claim results:
- AC-10 `p`: matches the code and `an_unproved_presence_checks_only_under_kernel`. `g` has no
  test on #2706 yet, but the presence guard over a parameter path is the same mechanism
  `qsl-eval/tests/it/child_composite.rs:476` relies on.
- AC-11: matches `a_state_clause_still_refuses_its_obligation_under_kernel`, one refusal in
  each mode. FR-104 "Definedness facts" does require the Linked check for state clauses.
- AC-12: the `w`, `f(x)` and `f(x) + 1` outcomes and locations match the code and tests. The
  `f(x) + 1` difference between Kernel and Linked is accurate: Linked stops at the range
  refusal before termination runs, and Kernel drops the range obligation, records the call
  and reaches `missing-measure` at the first operand.
- AC-13 spellings `undefined_expression`/`unproved-range` and `invalid_package`/`invalid-value`
  are accurate. For the dispatch halves, "same under both modes" is accurate. The rational
  half cannot be built as worded (FND-001).
- FR-495-AC-3 is unchanged by the diff. It still requires the Linked integer
  `div`/`rem`/`mod` `unproved-nonzero` refusal that QSL-1962 tracks as unimplemented, and the
  new bullet agrees with it: Linked refuses and Kernel drops the obligation.
- TC-1419 and TC-1420 are not on origin/main, no other open PR adds or cites them, and no other
  open PR edits FR-495. The diff merges cleanly into origin/main.
- `quire validate` passes on all three changed files.

## Verdict

CHANGES REQUESTED. The presence, state-clause, measure and dispatch claims are accurate,
and AC-10, AC-11 and AC-12 can fail and match the implemented behaviour. FND-001 is blocking:
the rational half of AC-13 names a fixture that the stated entry point cannot build.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-495-AC-13's rational half cannot be built through `PackageDeclarations::check`. A package declaration holds forms, not checked nodes, and the Typer builds `Rational` and `RationalNegate` only from `Rational` operands. So no supplied declaration yields a checked rational operation with an `Integer` operand, and the Kernel shape guard is reachable only by walking a hand-built checked body directly. | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md:245 |
| FND-002 | medium | The state-clause boundary says Kernel "does not reach a state clause" and "never admits a clause whose obligation Linked refuses" without naming the entry point. `CheckedGraph::check_clause_expression` with `ClauseKind::Invariant`, `Precondition` or `Postcondition` takes a `CheckMode`, and on #2706 it passes that mode to `Definedness`, so a clause expression checked there under Kernel does drop its obligations. The claim holds only for state clauses a package declares, checked through `PackageDeclarations::check`. | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md:165 |
| FND-003 | low | AC-12's Linked outcome for `f(x) + 1` ("refuses `unproved-range` at the `+` ... which refuses that obligation first") reads as if more refusals follow. The code returns exactly one refusal, because definedness refuses before termination runs, and the #2706 test asserts one. Saying "one refusal", as AC-11 does, would let a test fail on an extra `missing-measure` refusal. | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md:244 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
