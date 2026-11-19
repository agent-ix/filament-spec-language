---
id: TC-3134
title: "Union payload integer arguments widen like function-call arguments"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-317
    type: verifies
---
# TC-3134: Union payload integer arguments widen like function-call arguments

## Description

A union-member construction widens a bounded integer payload argument to the
member's declared payload type by the explicit exact conversion a function
call gives its argument, so the checked package carries the member's payload
type and reads back through the strict Contract IR reader; a payload argument
that does not fit gets the function-call verdict.

Scope: FR-317-AC-7, FR-317-AC-8.

## Test Procedure

Over `union Reading { Raw(Integer), Wide(Int[0, 2000]), Tiny(Int[0, 9]) }`,
`function g using v(a: Integer): Boolean pure { true }` and
`function t using v(a: Int[0, 9]): Boolean pure { true }`, in a function with
parameters `x: Int[0, 1000]`, `n: Integer` and `w: Int[0, 20]`:

1. Check `Reading::Raw(x)`, `Reading::Wide(x)`, `Reading::Raw(n)` and
   `Reading::Raw(7)`, and check `g(x)` beside them.
2. Emit the checked package of a unit holding the step-1 constructions and
   read it through the strict Contract IR reader; read each `union_value`
   payload term's type node.
3. Remove the conversion of `Raw(x)` from the step-2 package by hand, so the
   payload term is typed `Int[0, 1000]`, and read it again.
4. Check `Reading::Tiny(w)`, `Reading::Tiny(n)`, `t(w)`, `t(n)`,
   `if w <= 9 then Reading::Tiny(w) else Reading::Raw(n)`,
   `if w <= 9 then t(w) else false` and `Reading::Tiny(true)`.

Tag each test `#[trace("TC-3134", "<AC id>")]`.

## Expected Results

- Step 1: `Raw(x)`'s payload argument is the exact integer conversion of `x`
  to `Integer`, equal to `g(x)`'s argument node; `Wide(x)`'s is the exact
  conversion to `Int[0, 2000]`; `Raw(n)` and `Raw(7)` carry no conversion.
- Step 2: the reader admits the package, and every payload term's type node
  is its member's declared payload type node.
- Step 3: the reader refuses `ill_typed`/`type-mismatch` at the
  `union_value` node.
- Step 4: `Tiny(w)` and `Tiny(n)` are each refused
  `undefined_expression`/`unproved-range` at the payload argument, matching
  `t(w)` and `t(n)`; both guarded forms are admitted; `Tiny(true)` is
  refused `ill_typed`/`type-mismatch` at `true`.
