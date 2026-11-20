---
id: TC-376
title: "Function application checking accepts a well-typed call and refuses wrong arity, an unknown name and a type mismatch"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-065
    type: verifies
---
# TC-376: Function application checking accepts a well-typed call and refuses wrong arity, an unknown name and a type mismatch

## Description

Verify the behavior of `Value`'s function-application check
(`check::family::Application`, which `Typer::infer_form`'s `Call` arm calls):
checking an `Expression::Call` through the S3 typer against a real signature
admits a well-typed call as a typed call node, and refuses an ill-formed
call (wrong arity, an unknown callee name, a mismatched argument type) with
the crate's catalogued refusal cause, and that a tuple construction widens
its integer arguments to the declared position types as a call widens its
arguments. Scope: FR-065-AC-4, FR-065-AC-9, FR-065-AC-10, FR-065-AC-11.

## Test Procedure

1. Build a one-parameter `Boolean -> Boolean` signature named `f` and check
   `f(true)`.
2. Using the same signature, check `f(true, false)` (one argument too many).
3. With no declared function or type, check `nowhere()` (an undeclared
   name).
4. Using the same signature as step 1, check `f(1)` (an `Integer` argument
   against a declared `Boolean` parameter).
5. Declare `g using v(a: Integer)` and `h using v(a: Int[0, 2000])`; check
   `g(x)` and `h(x)` for `x: Int[0, 1000]`, and `g(n)` for `n: Integer`.
6. Declare `tuple Pair(Integer, Int[0, 2000]);` and `tuple Small(Int[0, 9]);`;
   check `Pair(x, x)`, `Pair(n, x)`, `Pair(7, x)` and `g(x)` for `x: Int[0, 1000]`
   and `n: Integer`.
7. Emit the checked package of a unit holding the step-6 constructions and read
   it through the strict Contract IR reader; read each `tuple_value` position
   term's type node.
8. Edit the step-7 package by hand so `Pair(x, x)`'s position-0 term is `x`,
   typed `Int[0, 1000]`; remove the then-unreferenced conversion node and
   re-derive every node's `dependencies` and identity. Read the edited package
   first with the `Pair(x, x)` construction left out, then whole.
9. For `w: Int[0, 20]` and `function t using v(a: Int[0, 9]): Boolean pure { true }`, check `Small(w)`, `Small(n)`, `t(w)`, `t(n)`,
   `if w <= 9 then Small(w) else Small(0)`, `Small(true)` and `Pair(x)`.

## Expected Results

- Step 1: the call is accepted; the produced node's value type is `Boolean`
  and its callee is signature index 0.
- Step 2: the call is refused with a type-mismatch cause (arity does not
  match).
- Step 3: the call is refused with a missing-name cause naming `nowhere`.
- Step 4: the call is refused with a type-mismatch cause.
- Step 5: `g(x)` and `h(x)` carry an exact integer conversion of the argument
  to the parameter's type; `g(n)` carries none.
- Step 6: `Pair(x, x)`'s position-0 argument is the exact integer conversion of
  `x` to `Integer`, equal to `g(x)`'s argument node, and its position-1
  argument the exact conversion to `Int[0, 2000]`; `Pair(n, x)` and
  `Pair(7, x)` carry no conversion at position 0.
- Step 7: the reader admits the package, and every position term's type node is
  its position's declared type node.
- Step 8: without the construction the edited package passes strict wire,
  graph and identity admission; whole, the reader refuses
  `ill_typed`/`type-mismatch` at the `tuple_value` node.
- Step 9: `Small(w)` and `Small(n)` are each refused
  `undefined_expression`/`unproved-range` at the argument, matching `t(w)` and
  `t(n)`; the guarded form is admitted; `Small(true)` is refused
  `ill_typed`/`type-mismatch` at `true`; `Pair(x)` is refused
  `ill_typed`/`type-mismatch`.
