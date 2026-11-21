---
id: SR-20261121041550
title: "code review of PR3253 equality of two integer-division operands compared at Integer"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/check.rs, fsl-semantics/tests/it/native_scalar_equality.rs, fsl-package/src/emit/tests/converted_equality.rs; PR3253 head 7f9b7db26edd6d47fb51712698435c2e43df0cbc"
review_set: subset
---

## Summary

Ticket: QSL-2723. Code-review with the rust-review lane folded in. Diff only, read at the PR head with `git show`, no build. The change is sound: `lowers_at_integer` is true only on shapes that `lowered_integer_types` (ir.rs:1124) also marks as lowered, so no equality is admitted that the graph cannot carry and the Source witness agrees with the graph. The helper is narrower than the lowering, though. Several shapes the graph lowers at `Integer` still take the interval check and refuse `type-mismatch` when their ranges differ.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `lowers_at_integer` is stricter than `lowered_integer_types`: the graph lowers an `if` when ANY branch lowers (FR-495-AC-23 `if c then a mod b else 0`), a `Local` bound to a lowered value, a `let` body and `case` arms, but the helper accepts only `IntegerDivision` and an `if` whose EVERY leaf is one, so `(if c then n mod 3 else 0) = n mod 2`, `let r = n mod 3 in r = n mod 2`, `(let r = n mod 3 in r) = n mod 2` and a `case` over divisions still refuse `type-mismatch` although both operands lower at `Integer` | fsl-semantics/src/check/check.rs:3920-3947 |
| FND-002 | low | The emitter witness test has no row for the `if`-versus-division equality that the check test admits (`(if c then n mod 3 else n mod 2) = n mod 2`), so the Source witness at position 1 of a control-node pair is not pinned | fsl-package/src/emit/tests/converted_equality.rs:1744-1749 |

## Verdict

Mergeable on soundness: there are no high findings. FND-001 is a real completeness gap against the lowering rule and should be fixed in this PR.

- FND-001: make `lowers_at_integer` apply the same rule as `lowered_integer_types` (any lowered branch, `Local` through its `Let` binding, `Let` body, `Case` arms, or a node typed `Integer`), ideally by sharing one predicate. Add check-test rows for the four shapes.
- FND-002: add `("(if c then n mod 3 else n mod 2) = n mod 2", vec![0, 1])` to `integer_division_operands_retain_integer_source_witnesses`.

Checked clean:

- **Helper true implies graph lowered (item 1).** An `IntegerDivision` node is always lowered (ir.rs:1159-1161). An `If` of Int type is lowered when any child is lowered, so an all-division `if` is lowered too. The helper is a strict subset of the lowering, so the stored `Integer` operands always match a lowered child. The Source fact keeps `source_type` (the checked interval). `equality_identity` (lowering.rs:6245-6249) and `data_at_lowered_types` map that fact to `lowered_value_type()` = `Integer` because the child's checked type still equals the fact type. The witness and the graph therefore agree at both positions.
- **Shapes where the two agree.** A parenthesised division creates no node. A converted operand is rejected by `conversion_target.is_some()`, and the lowering does not lower a conversion target either. `Coerce` and `Negate` over a division fall to `_ => false`. `lowered_integer_types` has no dependency arm for them, so neither side lowers them.
- **Soundness against non-integers (item 2).** The helper requires both operands to be unconverted and typed `Int(_)`/`Integer`. An enum, Decimal, Rational, quantity, Text or Float operand never takes the new branch and keeps `check_equality_in`. The Float and CarriedQuantity early returns come before it. The enum negative test is a regression guard only: one side is not a division, so it cannot reach the new branch, and by construction it cannot.
- **One side division, other a plain Int variable or literal (item 3).** The helper is false for a `Local` or a `Literal`, so the pair is unchanged and goes through the existing matrix and conversions.
- **Tests (item 4).** The check test asserts the check passes for `=`, `!=`, `div`/`mod` and an all-division `if`. The emitter test asserts an `Integer` Source witness at positions 0 and 1 for `n mod 3 = n mod 2` and `n div 2 = n mod 3`.

Skipped: `quoin:gap-analysis` was out of the brief's diff-only, one-pass scope.

- fsl-semantics/src/check/check.rs (`lowers_at_integer`, `equality`): examined
- fsl-semantics/src/check/ir.rs (`lowered_integer_types`, `lowered_value_type`): context only
- fsl-semantics/src/check/lowering.rs (`equality_identity`): context only
- FR-495-AC-21, FR-495-AC-23, FR-495-AC-24 (spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md): AC-24 examined, AC-21/23 context only
