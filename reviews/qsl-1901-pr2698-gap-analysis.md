---
id: SR-2026111612172400000055
title: "Gap analysis of QSL-1901 Compile refusal region and ticket acceptance (filament-spec-language PR 2698, post-merge)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@d1c76b6be3e37fb556afbabfd8ed662cf51eeaf6; FR-121-AC-37, FR-577-AC-1..4, TC-1416 and fsl-replay/src/spine/clause/tests/postcondition_range.rs (PR 2698)"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-1901. Plan completion: not assessed. Planless gap analysis of the
post-merge diff, plus a check that the ticket's acceptance holds on the PR head.
On origin/main, FR-121-AC-37 was specified but unmet. `CallSiteRefusal::Compile`
had no `region`, and no test carried the `FR-121-AC-37` tag. This PR closes that
gap. The new test `the_refusal_region_spans_exactly_the_bytes_of_the_unproved_node`
is the only binder of FR-121-AC-37. It asserts `Some`, the source (authority
`agent-ix`, identity `clause-run-fixture`) and the exact byte span of
`pre(self.balance) - amount`. One conjunct of the AC, the `code`, is not asserted
in that test.

Ticket acceptance on the PR head:
(1) The equality postcondition is accepted when the precondition proves the range.
`the_precondition_discharges_the_postcondition_range` (FR-577-AC-1) checks the
bank unit through `call_site` and asserts the difference is typed `Int[0, 100]`.
It passes.
(2) Without the precondition, the refusal names the span and the operand.
`without_the_precondition_the_difference_is_refused_with_operand_and_intervals`
(AC-3, AC-4) asserts the operand and both intervals in order, and the new test
asserts the region span. Both pass.
(3) The tests run over `test/bank`. All 39 `postcondition_range` tests pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-121-AC-37 also requires that the `Compile` refusal's `code` is `undefined_expression`. The test tagged to it binds `region` with `..` and never checks `code`. The conjunct does hold for the same unit, through `refusal()` in the AC-3/AC-4 test, but that test is not tagged FR-121-AC-37. Bind `code` in the new test and assert `Code::UndefinedExpression` | fsl-replay/src/spine/clause/tests/postcondition_range.rs:422-439 |

## Scope and evidence

- Run on the PR head (CARGO_TARGET_DIR per worktree): `cargo-slot cargo test -p fsl-replay --lib -- postcondition_range proof_result::tests execute::tests --test-threads=1 --skip ...`. Result: 270 passed, 0 failed, 5 ignored; all 39 `postcondition_range` tests pass, including the new one. Clippy for fsl-replay is reported in the reviewer's report.
- Trace check: `quoin matrix --json` did not finish within 300 s on the full repository, so the binding was checked by grep instead. `"FR-121-AC-37"` appears only at postcondition_range.rs:422 on the head, and nowhere on origin/main.
- TC-1416 step 2 on main ("CallSiteRefusal::Compile has code undefined_expression, its message contains, in order, ... and its region spans exactly the bytes of pre(self.balance) - amount") matches the implementation. Step 2's trace instruction (`#[trace("TC-1416", "FR-121-AC-37")]`) is followed.
- Inherited on main, not PR findings: the earlier weak-test findings no longer apply on main. AC-9 and AC-10 have reversed-order tests (`the_smaller_bound_is_kept_when_the_stricter_comes_first`, `..._across_clauses_when_the_stricter_clause_comes_first`). AC-32 asserts `CallRefusal::Record` with `integer_out_of_domain`, located exactly at the difference, before the replay checks. AC-2 compiles through `bank_site` (`call_site`) and compares the clause sites and the requirement record keys. AC-1 asserts the `Int[0, 100]` type of the coerced difference.

- FR-121-AC-37 (spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md): call_site refuses CallSiteRefusal::Compile whose region is Some, names the unit's source and spans exactly the bytes of `pre(self.balance) - amount`, and whose code is undefined_expression. (examined)
- FR-577-AC-1 (spec/functional/FR-577-discharge-a-postcondition-s-range-obligations-from-its-operation-s-preconditions.md): the bank unit checks; the difference is typed Int[0, 100]. (examined)
- FR-577-AC-2: moving CanAfford after Debited gives equal clause identities and requirement keys. (examined)
- FR-577-AC-3 / FR-577-AC-4: without CanAfford, refused undefined_expression/unproved-range at the difference; the message names the operand, [0, 100] and [-100, 100]. (examined)
- FR-577-AC-9 / FR-577-AC-10 / FR-577-AC-32: inherited-test checks on main. (examined)
- TC-1416 (spec/test-cases/TC-1416-a-postcondition-s-range-is-proved-from-its-operation-s-precondition.md): steps 1, 2 and 13. (examined)

## Verdict

The ticket acceptance holds on the PR head, and FR-121-AC-37 moves from unmet on
main to tested. FND-001 is a small strengthening of a test and does not block the
merge.
