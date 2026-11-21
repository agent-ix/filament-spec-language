---
id: TC-3137
title: "A let binding links its binder node"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: verifies
---
# TC-3137: A let binding links its binder node

## Description

Verify that S4 writes every `let` binding term with the `binder` link FSpec
FR-322 "The binder link" requires, for a read and an unread binder and for a
checker-introduced `let`. Scope: FR-093-AC-68.

## Test Procedure

1. Emit `let y: Integer = x in g(y)` over `x: Int[0, 1000]` and read the
   `let` application's binding term, its `dependencies` and the call
   argument; admit the package with the strict reader.
2. Emit `let y: Int[0, 1000] = x in true` and read the same members.
3. Emit `sqrt(p) <= sqrt(q)` over two `Float64[nearest-even]` parameters and
   read both `#ieee_operand` `let`s.
4. Emit step 2's unit with the annotation `Int[0, 2000]` and compare the
   `let` node ids.

Tag the tests `#[trace("TC-3137", "FR-093-AC-68")]`.

## Expected Results

- Step 1: `binder` names `y`'s `Integer`-typed parameter node, which is in
  `dependencies` and is the call argument's target; admitted.
- Step 2: `binder` and the dependency present although the body reads no
  `y`.
- Step 3: each `let` carries the binder node of its own level.
- Step 4: the two `let` node ids differ.
