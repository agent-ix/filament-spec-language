---
id: FR-700
title: "Prove rational arithmetic results by numerator and denominator intervals"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-001
    type: implements
  - target: ix://agent-ix/filament-specification/FR-146
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-149
    type: depends_on
---
# FR-700: Prove rational arithmetic results by numerator and denominator intervals

## Description

A `Rational[n1, n2; d1, d2]` type bounds the reduced numerator and the
positive reduced denominator of its members (FSpec FR-149 "Bounded
rationals"). When a rational `+`, `-`, `*`, `/` or unary `-` result meets a
required rational type (a function result, an argument, a narrowing),
`check` proves the range obligation with exactly FSpec FR-146's interval
rule: the raw numerator and denominator intervals are exact interval
arithmetic over the operands' intervals, and the normalized result intervals
follow from the raw ones. The obligation holds when the normalized numerator
interval lies inside `[n1, n2]` and the normalized denominator interval
inside `[d1, d2]`.

This rule is the specified one. It is exact for products and quotients of
positive operands: the product of two `Rational[1, 9; 1, 9]` values has raw
numerator `[1, 81]` over raw denominator `[1, 81]`, which normalize to
numerator `[1, 81]` and denominator `[1, 81]`, so the product is admitted
into `Rational[1, 81; 1, 81]`. A reader never has to widen a numerator or
denominator range beyond what the interval rule gives, and `check` SHALL
NOT apply a stricter or a different rule than FR-146's.

## Inputs

- A checked rational arithmetic expression and the operands' numerator and
  denominator intervals (declared, literal points, or FR-146's promotion of
  an integer operand of `/`).
- The required rational type of the position that receives the result.

## Outputs

- The admitted expression, or `undefined_expression`/`unproved-range` at the
  expression when the normalized intervals do not lie inside the required
  bounds.

## Behavior

- `check` SHALL compute the raw and normalized result intervals of each
  rational operation exactly as FSpec FR-146 states, with mathematical
  integers and no clamping, saturation or machine-width rounding.
- `check` SHALL admit a result whose normalized numerator interval lies
  inside the required `[n1, n2]` and whose normalized denominator interval
  lies inside the required `[d1, d2]`, and SHALL refuse
  `undefined_expression`/`unproved-range` at the expression otherwise.
- The rule takes no account of which members are actually reachable inside
  the intervals: a required type narrower than the normalized intervals
  refuses even when every reachable value would fit, because FR-146 admits
  no other reasoning.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-700-AC-1 | `function f using v(a: Rational[1, 9; 1, 9], b: Rational[1, 9; 1, 9]): Rational[1, 81; 1, 81] pure { a * b }` and the same with body `a * a` each check with no refusal. | Test (TC-1906) |
| FR-700-AC-2 | The same functions with result type `Rational[1, 80; 1, 81]`, and with result type `Rational[1, 81; 1, 80]`, each refuse `undefined_expression`/`unproved-range` at the product. | Test (TC-1906) |
| FR-700-AC-3 | `function g using v(a: Rational[1, 9; 1, 9], b: Rational[1, 9; 1, 9]): Rational[1, 162; 1, 81] pure { a + b }` checks (raw numerator `[2, 162]` over `[1, 81]`), and with result type `Rational[1, 161; 1, 81]` it refuses `undefined_expression`/`unproved-range`. | Test (TC-1906) |

## Dependencies

- FSpec FR-146 owns the interval rule this requirement applies; FSpec
  FR-149 owns the bounded rational type and its membership.
