---
id: SR-20261121032511
title: "code review of PR3249 lowered equality operand facts and decimal narrowing by declared ranges"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/lowering.rs, fsl-semantics/src/check/facts/conversions.rs, fsl-semantics/tests/it/narrowing_conversions.rs, fsl-semantics/tests/it/native_literals.rs, fsl-package/src/emit/tests/decimal_constructors.rs, spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, tests/it/bare_recovery_binder/checked_state_core.rs; PR3249 head bd2c2fc3046f5d4ae5382469a8ed5eb67ba40980"
review_set: subset
---

## Summary

Ticket: QSL-2723. Code-review with the rust-review lane folded in. Diff only, read at the PR head with `git show`, no build. The equality-fact retyping is exact and safe. The decimal interval proof is exact for `exact`-mode arithmetic but unsound when the arithmetic node's type rounds: a `*` node whose result type is `nearest-even` (or any non-`exact` mode) retains its result rounded at its own `smax`, so the runtime value can lie outside the exact interval the new code proves, and a range narrowing is admitted that fails at run time.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `DecimalBounds::of` treats a `Decimal` arithmetic node's value as the exact interval of its operands, but a node typed at a non-`exact` rounding mode rounds its result at its `smax` (filament-exact `evaluate`), so e.g. `s: Decimal[0,99;1,1;nearest-even]`, `convert<Decimal[0,495;2,2;nearest-even]>(s * decimal(5,1))` is admitted (exact upper 4.95) while `s = 9.9` yields 5.0 and the narrowing fails at run time | fsl-semantics/src/check/facts/conversions.rs:232-270 |
| FND-002 | low | No test pins the retyped Source facts for the nested shapes the predicate also covers: an `if` over a `mod` operand, both operands `mod`, `(a mod 3) mod 2` | fsl-semantics/src/check/lowering.rs:1267-1287 |

## Verdict

Not mergeable until FND-001 is fixed.

- FND-001: in the `NodeKind::Decimal` arm return `None` unless the node's own `ValueType::Decimal` has `rounding() == RoundingMode::Exact` (or round the interval outward to one unit at the node's `max_scale`), so a rounding node falls through to the previous `unproved-range` refusal.
- FND-002: add one emitter test with `if c then n mod 3 else n mod 2 = 0` and `n mod 3 = n mod 2` asserting both Source witnesses carry `Integer`.

Checked clean:

- **Retyping is exact (item 1a).** `data_at_lowered_types` uses the same predicate `equality_identity` already used for the identity node (lowering.rs:6245): Source role, child at `operand_position`, child's checked type equal to the fact's type, then `lowered_value_type()`, which differs from the checked type only where `lowered_integer` is set (ir.rs:720). So the emitted fact value type now matches the identity type node exactly where it previously disagreed, and is unchanged everywhere else. Enum, Text/Plan and quantity facts are untouched (the flag is integer-only). An operand with a conversion target does not match (its child is typed at the target), and the ConversionTarget role is skipped, as is the identity side.
- **Proof unchanged (1b).** The checker's equality proof (`check_equality_in` in check.rs) runs at check time on the checked operands; the only reader of `operand_type_facts` after lowering is the emitter's witness rows (emit.rs:3431-3540). Only the emitted fact changes.
- **Conversion target (1c).** The literal `0` keeps its ConversionTarget fact `Int[0,2]` and is an inline literal term of the integer `eq`, as before; nothing about it changed.
- **Nested (1d).** `(a mod 3) mod 2`, an `if` over a division and both-mod operands all reach the same flag, so they are retyped by construction (FND-002 is only the missing test).
- **Interval arithmetic.** `+`, `-` (`[l.lo - r.hi, l.hi - r.lo]`) and `*` (min/max of all four corner products) are correct for negative ranges; all arithmetic is exact `Integer` numerators over a power-of-ten scale, no floats; `/` yields `None`. The declared hull takes lo/hi at both `smin` and `smax` and picks the extreme, correct for either sign. `within` checks the target bounds at both its `smin` and `smax`; since membership bounds are monotone in scale this covers every intermediate scale. Scale fit: the branch is reached only for `RangeNarrowing`, which `classify` gives only when the operand's `smax` <= the target's `smax` (else `ScaleReduction`), and every value reaching the conversion is a member of the operand node's type, so its scale fits the target. Conversions, coercions, negations and other nodes fall to their declared type, which their evaluation guarantees. Every unhandled path returns `None` and falls through to the previous refusal.
- **AC-42 ruling (item 3).** FSpec FR-149 row 238 (`Decimal[c1,c2;s1,s2]` to `Decimal[c3,c4;s3,s4]` needs `s3 <= s1`) makes `[0,1000;1,1]` into `[0,100000;2,2]` a range narrowing and into `[0,100000;1,2]` an exact widening. The test and the AC-42 row change only that one type token; no assertion changed. The companion test pins the `[..;2,2]` case as one `numeric.narrow` and no `convert`.
- **native_literals fixture (item 4).** On origin/main `convert<P>(r)` with `P = [0,1000;2,2]` (to 10.00) and `R = [0,1000;1,1]` (to 100.0) is a range narrowing; `discharge_conversion` has no literal, and no integer interval of `r` admits into `P` (`hi * 100 <= 1000` fails), so it refused `unproved-range` and the `.expect` was red on main. The PR only adds early `Ok` returns, and under the PR the old fixture is still refused (100.0 > 10.00), so the PR did not introduce it. Widening `P` to 100.00 is legitimate: the test's point is mixed-type rejection, and the conversion is now proved from `R`'s declared range.
- **Un-ignored test.** `bytes_only_recovery_combines_modulo_multiplicity_sum_and_branch_selection` is the QSL-2723 target; its ignore reason was exactly the mismatched witness type.

## Scope and evidence

Read the PR diff and, at the PR head, conversions.rs, lowering.rs (`data_at_lowered_types`, `application`, `next_operand`, `equality_identity`), ir.rs `lowered_value_type`, check.rs equality fact construction, emit.rs witness rows, numeric_conversion.rs `classify`, filament-exact b26a8e4 `DecimalType::contains` and `evaluate`, FR-093 "Explicit numeric narrowing" and AC-42/AC-67, FSpec FR-149 equality conversion table and conversion classes. No cargo. Gap-analysis not run (one proportionate pass per the brief).

- FR-093-AC-42 (spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md): examined
- FR-093-AC-67 (spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md): examined
- FR-093 Explicit numeric narrowing (spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md): examined
- FSpec FR-149 equality conversions and conversion classes: examined
- FR-495-AC-24: context_only
