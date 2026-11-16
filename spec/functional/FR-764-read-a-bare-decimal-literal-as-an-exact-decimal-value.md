---
id: FR-764
title: "Read a bare decimal literal as an exact decimal value"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-001
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-002
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-195
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-662
    type: implements
---
# FR-764: Read a bare decimal literal as an exact decimal value

## Description

A bare decimal literal such as `0.9`, `5.0` or `0.90` SHALL be an expression
in every value position, denoting exactly the decimal constructor
`decimal(c, s)` that spells the same number: `c` is the literal's digits with
the point removed, read as an integer, and `s` is the number of digits after
the point. `0.9` is `decimal(9, 1)`, `5.0` is `decimal(50, 1)` and `0.90` is
`decimal(90, 2)`. FSpec FR-662 states the rule for every implementation; this
requirement states how FSL's stages carry it.

## Rationale

Authors write `self.level < 0.9` and `expected ... <= 0.5` and expect them to
mean what they say. The standard's exact-value positions already read the
same spelling as an exact rational, and the literal is exact by
construction, never floating point, so reading it the same way in a value
position keeps one meaning for one spelling and protects nothing by refusing
it. Reading it as the constructor it spells reuses the constructor's typing,
limits and lowering, so the literal adds no new value kind and no new rule a
checker must learn.

## Inputs

Source text holding a `decimal-literal` token, `[0-9]+ '.' [0-9]+`, in a
`primary` position.

## Outputs

A located primary at S1, a `DecimalValue` form at S2 and the constructor's
check, lowering and evaluation at S3 and later, or a located refusal.

## Behavior

- **S1.** `primary` SHALL admit `decimal-literal`. A `decimal-literal`
  followed by a qualified name stays a `quantity-literal` (`0.5 M::ms`), which
  is the longer match; a `decimal-literal` followed by anything else is the
  decimal primary. `decimal-literal` SHALL NOT appear in the expected-token
  list of an expression-start refusal, because it is now a valid start.
- **S2.** S2 SHALL build a `DecimalValue{coefficient, scale}` for a decimal
  primary with `coefficient` the digits without the point read as a
  non-negative integer and `scale` the count of digits after the point, with
  the literal's span and the spelling kind `decimal-literal`. It reduces
  nothing: `0.90` keeps scale 2.
- **S3 and later.** `check`, lowering and the evaluator SHALL treat a decimal
  primary exactly as the `decimal(c, s)` it denotes (FR-093 "Decimal
  constructors"): the position gives its `Decimal[..]` type (a declared result,
  an arithmetic peer, or the other operand of a comparison when that operand
  is not a literal), a position that
  gives none refuses `ill_typed`/`ambiguous-literal`, a position that expects
  a type that is not a `Decimal[..]` refuses `ill_typed`/`type-mismatch`, and
  the scale counts against `s3.decimal_literal_scale` and
  `s3.decimal_literal_scale_total`.
- **Exact positions.** Where a requirement takes an exact rational literal
  threshold (FR-186, FR-195: an `expected accumulate` threshold of an integer
  reward, a fraction or event comparison threshold), a decimal primary SHALL
  be the exact rational it denotes, so `0.5`, `decimal(5, 1)` and
  `rational(1, 2)` give equal thresholds and equal checked claims. In such a
  position the literal is charged FR-093's steps 2 and 3, the written-scale
  limit `s3.decimal_literal_scale` and the per-package total
  `s3.decimal_literal_scale_total`, and FR-093's step 1 does not apply,
  because the position's type is the exact rational, not a `Decimal[..]`.
- **Exponent forms.** A digit string or decimal literal immediately followed,
  with no whitespace, by `e` or `E`, an optional sign and a digit (`1e5`,
  `1.5E-3`) SHALL be one numeric token that S1 refuses `invalid_syntax`/
  `invalid-token` at the whole spelling, with the message `` `<spelling>` is
  not a numeric literal: an exponent is not part of a number; write a decimal
  literal such as 0.5, decimal(c, s) or rational(n, d) ``. This is the
  standard's unsupported numeric-form diagnostic.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-764-AC-1 | `fsl_cst::parse` over function bodies `a < 0.9`, `a <= 5.0` and `0.5` returns an admissible `ParsedSource` with no diagnostic, holding a `Primary` node whose span is exactly the literal; `0.5 M::ms` still parses as one `QuantityLiteral`, and `decimal(5, 1)`, `5` and `a.b` parse as before. | Test (TC-2161) |
| FR-764-AC-2 | S2 builds `DecimalValue{9, 1}` for `0.9`, `DecimalValue{50, 1}` for `5.0` and `DecimalValue{90, 2}` for `0.90`, each with the literal's span and the spelling kind `decimal-literal`. | Test (TC-2161) |
| FR-764-AC-3 | With `type D = Decimal[-1000, 1000; 0, 2; exact];`, `function f using v(): D pure { 0.9 }` checks; `function g using v(n: Integer): Boolean pure { n < 0.9 }` refuses `ill_typed`/`type-mismatch` at `0.9`; `function h using v(): Boolean pure { let x = 0.9 in x > decimal(1, 1) }` refuses `ill_typed`/`ambiguous-literal` at `0.9`. Each result equals the result of the same body with `decimal(9, 1)` in the literal's place. | Test (TC-2162) |
| FR-764-AC-7 | With `type D = Decimal[-1000, 1000; 0, 2; exact];`, `function k using v(l: D): Boolean pure { l < 0.9 }` checks, the literal typed `D` by the other operand (FSpec FR-140), as `self.level < 0.9` does over a field `level: D`; `function m using v(): Boolean pure { 0.9 < decimal(1, 0) }`, where no operand fixes a type, refuses `ill_typed`/`ambiguous-literal` at `0.9`. Each result equals the result with `decimal(9, 1)` in the literal's place. | Test (TC-2186) |
| FR-764-AC-4 | `1e5` and `1.5E-3` as function bodies refuse at S1 `invalid_syntax`/`invalid-token` located at the whole spelling, with the exponent message. | Test (TC-2161) |
| FR-764-AC-5 | Over an integer reward, `expected accumulate R until holds(B) <= 0.5` checks as `ExpectedReward` with `AtMost(1/2)`, and its checked claim equals the claim written with `decimal(5, 1)` and with `rational(1, 2)`. | Test (TC-2163) |
| FR-764-AC-6 | A threshold `0.` followed by 65536 digits refuses `stage_limit_exceeded`/`work-budget-exceeded` naming `s3.decimal_literal_scale`, bound 65535 and actual 65536, located at the literal. | Test (TC-2163) |

## Dependencies

- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md): S2's `DecimalValue`.
- [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md): the decimal constructor's typing and limits.
- [FR-186](FR-186-check-probabilistic-claim-forms-at-s3.md), [FR-195](FR-195-check-exact-only-forms-and-route-exact-evidence.md): exact thresholds.
- FSpec FR-662: the standard's decimal primary.
