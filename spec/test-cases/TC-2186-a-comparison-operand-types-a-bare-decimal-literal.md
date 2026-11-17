---
id: TC-2186
title: "A comparison operand types a bare decimal literal"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-764
    type: verifies
---
# TC-2186: A comparison operand types a bare decimal literal

## Description

A comparison operand types a bare decimal literal.

Scope: FR-764-AC-7.

## Test Procedure

With `type D = Decimal[-1000, 1000; 0, 2; exact];`, check `function k using v(l: D): Boolean pure { l < 0.9 }` and `function m using v(): Boolean pure { 0.9 < decimal(1, 0) }`; repeat with `decimal(9, 1)`. Test: `fsl-semantics/tests/it/probabilistic_declarations.rs` `a_comparison_operand_types_a_bare_decimal_literal`.

Tag the tests `#[trace("TC-2186", "FR-764-AC-7")]`.

## Expected Results

- `k` checks.
- `m` refuses `ill_typed`/`ambiguous-literal` at `0.9`.
- Each outcome equals the `decimal(9, 1)` variant's.
