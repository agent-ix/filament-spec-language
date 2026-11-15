---
id: TC-1781
title: "Conflicting law headers and law aliases are refused"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-670
    type: verifies
---
# TC-1781: Conflicting law headers and law aliases are refused

## Description

Verify the refusals of a law header: two different laws in one unit, and a
law alias used where a layer or site profile is required. Scope: FR-670-AC-3
and FR-670-AC-4.

## Test Procedure

1. Compile a unit with law headers `d1` (floor) and `d2` (truncating) and
   FR-670-AC-1's `q`; then with `d1` and `d2` both naming floor.
2. Compile a unit with a law header `d` and
   `function q using d(a: Int[0, 9], b: Int[1, 9]): Int[0, 9] pure { a div b }`;
   then one with `check c using d { true }`.

## Expected Results

- Step 1: `invalid_package`/`conflicting-definition` at `d2`'s identity
  literal, retaining both definition references, and no checked package;
  the second unit checks with the floor law.
- Step 2: `unknown_profile`/`wrong-selection-role` at the `using` alias `d`
  in each unit.
