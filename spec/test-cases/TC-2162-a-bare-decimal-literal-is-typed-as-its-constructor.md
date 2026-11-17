---
id: TC-2162
title: "A bare decimal literal is typed as its constructor"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-764
    type: verifies
---
# TC-2162: A bare decimal literal is typed as its constructor

## Description

A bare decimal literal is typed as its constructor.

Scope: FR-764-AC-3.

## Test Procedure

With `type D = Decimal[-1000, 1000; 0, 2; exact];`, check `function f using v(): D pure { 0.9 }`, `function g using v(n: Integer): Boolean pure { n < 0.9 }` and `function h using v(): Boolean pure { let x = 0.9 in x > decimal(1, 1) }`; repeat each with `decimal(9, 1)`. Test: `fsl-semantics/tests/it/probabilistic_declarations.rs` `a_bare_decimal_literal_is_typed_as_its_constructor`.

Tag the tests `#[trace("TC-2162", "FR-764-AC-3")]`.

## Expected Results

- The function checks; `g` refuses `ill_typed`/`type-mismatch` at `0.9`; `h` refuses `ill_typed`/`ambiguous-literal` at `0.9`.
- Each outcome's code and cause equal the `decimal(9, 1)` variant's.
