---
id: SR-2026111612172400000025
title: "gap-analysis of PR2834 QSL-2361 FR-495-AC-24 and AC-25 coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-495-AC-24, FR-495-AC-25, TC-2873, TC-2874, fsl-replay/src/application/division_tests.rs; PR2834 0dc06c81bbf1b415668602fe8cf1c484eaea1e03"
review_set: subset
---

## Summary

Ticket: QSL-2361. This is a scoped check of whether the tests assert each clause of the new criteria FR-495-AC-24 and FR-495-AC-25, and of the baseline-failure claim. Plan completion was not assessed. `quire trace --id FR-495-AC-24` did not finish within 300 s, so the AC-to-test binding was checked by hand from the `#[trace]` attributes and the TC files. The test for AC-25 asserts every clause, and a positive control confirmed its count assertion is not vacuous. The test for AC-24 leaves one clause unasserted for three of its eight sources.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-495-AC-24 test asserts the operand source-type clause for only 5 of the 8 sources. `(n mod a) + 1 = 1` skips the check, and the Euclidean `n div a = 0` and `n rem a = 0` cases assert only emit and admit. The check that runs (`sources.contains(&mod_type)`) does not pin the builtin `Integer` node the AC names, and does not identify which operand is the division | fsl-replay/src/application/division_tests.rs:456-491 |

## Scope and evidence

- **FR-495-AC-24 → TC-2873 → `an_equality_over_a_mod_application_emits_and_reads_back`.** The binding is correct. Emit and admit are asserted for all 8 sources. The equality decision count of 1 is asserted for the 6 mod sources. The source type is asserted for the 5 sources that do not end in `+ 1`. A probe printed the decision's operands. The mod side is `scalar_type/integer` and the literal side is `bounded_domain [0,1]`, so `contains` currently discriminates. It is still not positional, and it does not assert the Integer node by name.
- **FR-495-AC-25 → TC-2874 → `an_if_over_integer_arithmetic_emits_and_reads_back_without_an_orphan_interval`.** The binding is correct. All four sources are covered, and emit, admit and zero hull nodes are each asserted. Positive control: in the same wires the helper counts the parameter's own interval node (max 999 for `amount: Int[0,999]`, max 1000 for `Int[0,1000]` and for `versionNumber`) as 1. So the zero count for 1999/1001/2000 measures something real. Without hunk 1 the test fails earlier, at emission (`UnlocatedOccurrence`).
- **Fails without the fix.** Each lowering hunk was reverted on its own, and each time one of the two tests failed (see SR-2026111612172400000024).
- **Ticket AC 2.** The ticket's second criterion is that CG `tc_025_*` passes with no CG test edits. It cannot be verified in this repo and needs the CG pin bump after merge. The AC-25 sources reproduce those `if` clauses.
- **Baseline-failure claim.** All runs were with `FSPEC_DIR=/home/peter/dev/filament-specification`. PR head failures: fsl-semantics `--test it` 3, `--lib` 12; fsl-package `--lib` 88; fsl-replay `--lib` 31, `--test it` 14. That totals 148, the same counts the coder reported, so FSPEC_DIR was not the cause. At the merge-base faac49e15, each lane's set of failing test names is identical to the PR head's. At origin/main f927d9a14 the sets differ, because main has moved on: sem-it 4 (+1), sem-lib 13 (+1), pkg-lib 105 (+22 checked_v2/timed conformance, +3 timed_order, -8 source_objects), rep-lib 53 (+22), rep-it 16 (+2). Every PR-head failure in sem-it, sem-lib, rep-lib and rep-it also fails on main. The merge-base causes are unrelated to this diff: `ProtocolAnchor(WrongObservation … Unlisted)` in the protocol fixtures (pkg-lib protocol_view/requirements/locations/occurrence/controls, and sem-lib protocol_*); `Unproved(Nonzero)` in the possibly-zero-divisor fixtures after #2738 (rep-lib application_claim, temporal on-each guard); `UnrepresentedConstruct HyperExpression` (hyper tests); the `null` not-yet-implemented diagnostic (parse_outcome_facade); `SourceObjectEmission` (pkg-lib source_objects, which pass on main); and refusal-cause drift in field reads (sem-it child_composite/state_clauses).

## Verdict

AC-25 is fully asserted, and the test fails without the fix. AC-24 is asserted for the reported regression (the mod-equality invariant) and fails without hunks 2 and 3. The source-type clause is still unasserted for the div/rem and `+ 1` sources, and the asserted cases do not check it positionally. FND-001 is a gap in test strength. It does not break correctness, and it can be fixed in the same PR or accepted.

## Dispositions

Round 1 was checked against head 2c8a731816f5f3ff59c934ce6afd72510315e209: fix commit 2c8a73181, after merging origin/main 15faf6452. The fsl-replay lib tests do not compile at this head, and they fail the same way on origin/main 15faf6452: E0063, `ProtocolSubject` is missing `capacities` at fsl-replay/src/spine/clause/tests/postcondition_range.rs:1041. That breakage is a semantic conflict on main between #2817, which added the test, and #2828, which added the field; it is not part of this PR. In a scratch worktree, with only that one field added, `division_tests` passes 9 of 9 at the new head. Reverting the equality-operand lowering hunk still fails `an_equality_over_a_mod_application_emits_and_reads_back` with MissingEqualityIdentityContext.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The new helper `assert_division_operand_compared_at_integer` runs for all 8 sources, div and rem included. For each, it finds the equality's division operand by position. It asserts that the division's `semantic_type` is the builtin `scalar_type/integer` node and that `decisions[0][side].operand == position`, and that side's `source_type` is that Integer node. The `+ 1` source was replaced with `n mod a = 1`. |
