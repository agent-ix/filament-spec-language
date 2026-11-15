---
id: TC-1783
title: "Integer division nodes carry their interval and nonzero obligation"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: verifies
  - target: ix://agent-ix/filament-specification/TC-1281
    type: references
---
# TC-1783: Integer division nodes carry their interval and nonzero obligation

## Description

Verify from source that `mod`, `div` and `rem` refuse a possibly-zero divisor
under `CheckMode::Linked`, that their checked node type is the proved
interval, that a remainder is bounded by its divisor even for an unbounded
dividend, and that only `CheckMode::Kernel` reaches a run-time zero divisor.
Scope: FR-495-AC-17 and FR-495-AC-18.

## Test Procedure

1. Under `CheckMode::Linked` and no law header, check `m0`, `m1`, `m2` and
   `m3` of FR-495-AC-17 and read the type of each checked `mod` node.
2. Check `m0` under `CheckMode::Kernel`, link it and call it with
   `(5, 0)`.
3. Under `CheckMode::Linked` and a law header for each law, check `r1` and
   `q1` of FR-495-AC-18, `r1` with result `Int[0, 11]`, and `r1` and `q1`
   with `b: Int[0, 12]`; read each operator node's type.

## Expected Results

- Step 1: `m0` refuses `undefined_expression`/`unproved-nonzero` at the
  `mod`, its only refusal; `m1` and `m2` check with the `mod` node typed
  `Int[0, 11]`; `m3` refuses `unproved-range` at the `mod`.
- Step 2: `Undefined::DivisionByZero` located at the `mod`.
- Step 3: the types and outcomes FR-495-AC-18 states.
