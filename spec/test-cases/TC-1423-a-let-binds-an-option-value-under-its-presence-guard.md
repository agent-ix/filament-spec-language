---
id: TC-1423
title: "A let binds an option value under its presence guard"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: verifies
---
# TC-1423: A let binds an option value under its presence guard

## Description

Verify that a presence-guarded `let` over an option `value` checks, lowers to
`control.if`, `option.present`, `control.let` and `option.value`, emits and
reads back `Verified`.

Scope: FR-093-AC-65.

## Test Procedure

1. Emit FR-093-AC-65's `guarded` over `o: Option<Int[0, 9]>` and read it back
   through IR's v2 reader.

Tag the tests `#[trace("FR-093-AC-65", "TC-1423")]`.

## Expected Results

- Step 1: the package reads back `Verified` and holds one
  `filament.op.control.if` node whose condition is a `filament.op.option.present`
  node and whose then-branch is a `filament.op.control.let` node binding a
  `filament.op.option.value` node.
