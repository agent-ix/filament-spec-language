---
id: SR-202611141554001
title: "Base review of the QSL-1982 FR-495 Kernel rational and reduction criteria"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2708 at 402ca8da65aef4aaafa016888cd1ac451d8e7a77, diff origin/main...HEAD: spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md (Kernel bullet lead sentence, FR-495-AC-14..16), spec/test-cases/TC-1421-kernel-mode-drops-the-rational-range-rational-divisor-and-nonempty-reduction-obligations.md"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-495, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1421, type: references }
---

## Summary

Ticket: QSL-1982. The criteria describe behaviour implemented on PR #2706 (QSL-1931, branch
`qsl-1931-checkmode` at 6297dca4c111619607591b408f56ef5c41a59fb5). I checked them by reading
that branch's code and tests, read-only. No cargo run was needed.

Code read:
- `qsl-semantics/tests/it/check_mode.rs` ~189-222. `an_unproved_rational_range_checks_only_under_kernel`,
  `an_unproved_rational_divisor_checks_only_under_kernel` and
  `an_unproved_nonempty_reduction_checks_only_under_kernel` use the exact `r`, `q` and `m`
  (`[0, 2]`) fixtures of AC-14..16 through `kernel_drops`. That helper asserts exactly one
  Linked refusal with the named `Obligation` (`RationalRange`, `Nonzero`,
  `NonemptyReduction`) and a clean Kernel check.
- `qsl-semantics/src/check/facts.rs`. Under Kernel, `Definedness::discharge` drops any
  `Unproved(_)`. The rational arm (~1291) checks the divisor's nonzero obligation before the
  result domain and returns at the first refusal. The `Fold { identity: None }` arm (~1462-1486)
  refuses `NonemptyReduction` only when the proved size's lower end is below 1. With no size
  fact, that size is `cardinality(collection)` (~403), whose lower end is the collection
  bound's `minimum()`.
- `qsl-semantics/src/check/refusal.rs` ~3518-3523. `Nonzero` is spelled `unproved-nonzero`.
  `RationalRange` and `NonemptyReduction` are both spelled `unproved-range`, with code
  `undefined_expression`.
- `quire-exact` `RationalDomain` (src/rational.rs). `Rational[lo, hi; dmin, dmax]` bounds the
  reduced numerator to `[lo, hi]` and the denominator to `[dmin, dmax]`. `result_of(Add)`
  bounds the numerator by interval arithmetic on cross products and the denominator by the
  product of the two maxima. `contains_domain` compares the numerator and denominator
  intervals.

Claim results:
- AC-14: the outcome and spelling match the code. For `r`, `result_of(Add)` gives numerator
  `[0, 4]` and denominator `[1, 4]`, neither inside `[0, 1]` / `[1, 2]`, so Linked refuses
  `RationalRange` once and Kernel checks. The stated reason is wrong (FND-001).
- AC-15: the outcome and spelling match the code. `b`'s numerator `[0, 1]` contains zero, so
  the nonzero obligation is refused first and is the only refusal. Kernel drops it and never
  reaches the result-domain check.
- AC-16: the `[0, 2]` outcome and spelling match the code. The author's `[1, 2]` control is
  correct by the code, though PR #2706 does not test it yet. The fixtures differ only in the
  collection bound, `cardinality` turns `[1, 2]` into a size whose lower end is 1, so the
  `Fold` arm discharges. The `[0, 2]` form's only Linked refusal is the `reduce`, so no other
  obligation is left to refuse. So the control checks under both modes.
- Each AC can fail on the old behaviour. A Kernel that recorded the obligation fails the
  Kernel half, and a Linked that dropped it fails the Linked half. The AC-16 control also
  fails if the checker ignored the collection's lower bound.
- The lead-sentence list "(range, nonzero, presence, rational-range and nonempty-reduction)"
  matches the `PackageDeclarations::check` doc on #2706, TC-1419's description and the
  boundaries of the merged FR-495. Boundary four still keeps the malformed rational shape
  (also `RationalRange`) refused under Kernel, and AC-13 states that exception.
- TC-1421 is not on origin/main. No other open PR adds or cites it, cites FR-495-AC-14..16,
  or edits FR-495. The branch merges cleanly into origin/main, and `quire validate` passes
  on both changed files.

## Verdict

CHANGES REQUESTED, for FND-001 only. The fixtures, outcomes, refusal spellings and the AC-16
control are right. The AC-14 parenthetical explains the refusal with a value-range comparison
that is not the rule the checker applies.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-14 explains the refusal as "the sum's range `[0, 2]` is not inside the declared rational domain `[0, 1]`". That compares a value range with a numerator bound. `Rational[0, 1; 1, 2]` bounds the reduced numerator to `[0, 1]` and the denominator to `[1, 2]`, and the checker refuses because `result_of(Add)` gives numerator `[0, 4]` and denominator `[1, 4]`, which the declared domain does not contain. Read as written, a result type covering the values `[0, 2]`, such as `Rational[0, 2; 1, 2]`, would check, but it is still refused. | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md:251 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | fa9f478c1: AC-14 now gives the reason as the declared numerator `[0, 1]` and denominator `[1, 2]` against the sum's numerator `[0, 4]` and denominator `[1, 4]`. That matches quire-exact `RationalDomain::result_of(Add)` (cross products `[0, 2]` + `[0, 2]`; denominator 2 * 2) and `contains_domain`. |
