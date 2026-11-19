---
id: TC-2161
title: "A bare decimal literal parses and builds a decimal value"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-764
    type: verifies
---
# TC-2161: A bare decimal literal parses and builds a decimal value

## Description

A bare decimal literal parses and builds a decimal value.

Scope: FR-764-AC-1, FR-764-AC-2, FR-764-AC-4.

## Test Procedure

1. Parse function bodies `a < 0.9`, `a <= 5.0`, `0.5`, `0.5 M::ms`, `decimal(5, 1)`, `5` and `a.b`. Test: `fsl-cst/tests/it/expression_syntax.rs` `a_bare_decimal_in_a_primary_parses_as_a_decimal_literal`.
2. Build S2 forms for `0.9`, `5.0` and `0.90`. Test: `fsl-forms/tests/it/value_forms.rs` `a_bare_decimal_literal_builds_the_decimal_value_it_spells`.
3. Parse function bodies `1e5` and `1.5E-3`. Test: `fsl-cst/tests/it/expression_syntax.rs` `an_exponent_number_is_the_unsupported_numeric_form`.

The existing `fsl-cst/tests/it/probabilistic.rs` `a_quantity_literal_is_an_expression_and_a_bare_decimal_is_not` asserts the old refusal of `0.5` and is changed to assert the decimal primary when this lands.

Tag the tests `#[trace("TC-2161", "FR-764-AC-1")]`.

## Expected Results

- Step 1: each decimal body is admissible with no diagnostic and a `Primary` node spanning exactly the literal; `0.5 M::ms` is one `QuantityLiteral`; the other three parse as before.
- Step 2: `DecimalValue{9, 1}`, `DecimalValue{50, 1}`, `DecimalValue{90, 2}`.
- Step 3: `invalid_syntax`/`invalid-token` at the whole spelling, the message naming the spelling and the accepted forms.
