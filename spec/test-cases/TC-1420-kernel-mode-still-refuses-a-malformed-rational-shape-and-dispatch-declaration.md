---
id: TC-1420
title: "Kernel mode still refuses a malformed Rational shape and dispatch declaration"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-495
    type: verifies
---
# TC-1420: Kernel mode still refuses a malformed Rational shape and dispatch declaration

## Description

Verify that `CheckMode::Kernel` refuses the structural faults of what it is
given as `CheckMode::Linked` does. The rational half is an in-crate test of the
`Definedness` walk over an already-checked body, because the typer builds a
rational operation only from `Rational` operands and no package declaration
yields the fault. The dispatch half supplies a package declaration to
`PackageDeclarations::check`, and guards that its refusals stay independent
of the mode. Scope: FR-495-AC-13.

## Test Procedure

1. In-crate, walk with `Definedness` a checked body holding a `Rational`
   operation, and another holding a `RationalNegate`, whose operand node has
   type `Integer`, once under `CheckMode::Linked` and once under
   `CheckMode::Kernel`.
2. Supply a package whose dispatch operation names a dispatch table index
   equal to the table count, and another whose dispatch table names a
   candidate function index equal to the function count. Check each through
   `PackageDeclarations::check` under both modes.

Tag the tests `#[trace("TC-1420", "FR-495-AC-13")]`.

## Expected Results

- Step 1: each walk refuses `undefined_expression`/`unproved-range` at the
  rational operation, under both modes.
- Step 2: each package is refused `invalid_package`/`invalid-value`, under
  both modes.
