---
id: TC-3064
title: "A two-path term guard bounds the same term"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-680
    type: verifies
---
# TC-3064: A two-path term guard bounds the same term

## Description

Verify FR-680-AC-9: a comparison of a two-path `+` or `-` term against an integer literal bounds that term at a call argument, while a too-weak bound, `!=`, a three-path term and a combination of bounds prove nothing.

## Test Procedure

Run `fsl check` over the command-intake collections corpus: one `.native` unit per body of FR-680-AC-9, each declaring `narrow(x: Int[0,9])` and a function over `x`, `y`, `z: Int[0,9]`, compared against its `.expected.json`.

## Expected Results

The `x + y <= 9` and `x - y >= 0` bodies succeed at S4 with exit 0; the other four refuse `undefined_expression`/`unproved-range` at the `narrow` argument with exit 20.
