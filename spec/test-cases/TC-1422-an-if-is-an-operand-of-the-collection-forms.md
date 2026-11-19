---
id: TC-1422
title: "An if is the element body or the collection operand of a quantifier"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: verifies
---
# TC-1422: An if is the element body or the collection operand of a quantifier

## Description

Verify that a `forall` or `exists` accepts an `if` as its element body and as
its collection operand, over a parameter sequence and over a model collection
field, and that each lowers to `filament.op.control.if` beneath the quantifier
node, emits and reads back `Verified`.

Scope: FR-093-AC-64.

## Test Procedure

1. Emit the `forall` and `exists` functions of FR-093-AC-64 whose element body
   is an `if`, and read each back through IR's v2 reader.
2. Emit the invariant of FR-093-AC-64 whose element body is an `if` over the
   model collection field `history`.
3. Emit the function of FR-093-AC-64 whose collection operand is
   `if c then s else t`.

Tag the tests `#[trace("FR-093-AC-64", "TC-1422")]`.

## Expected Results

- Step 1: the package reads back `Verified`; the `forall` binding body is a
  `filament.op.integer.le` node over one `filament.op.control.if` node of three
  arguments, and the `exists` binding body is that node.
- Step 2: the package reads back `Verified`; the `forall` binding body is a
  `filament.op.control.if` node.
- Step 3: the package reads back `Verified`; the `forall` collection operand
  is a `filament.op.control.if` node. A lowering that rejects or inlines an
  `if` in these positions fails each step.
