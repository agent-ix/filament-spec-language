---
id: TC-2818
title: "A timed source model emits its clock places with source rows"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-996
    type: verifies
---
# TC-2818: A timed source model emits its clock places with source rows

## Description

Verify that a timed source model's root names the source type's node and that its clock-place rows name source declarations by their source keys. Scope: FR-996-AC-1.

Catches a root or row that names a selected model declaration or a check-time alias.

## Test Procedure

1. Emit FR-230-AC-65's unit with `time O dense unit ms;` and read the timed root and its two clock places.

## Expected Results

- Step 1: the root's `model` is `O`'s node; `d` has the one row `ix://I/O`/`ix://I/O/d`/Clock; `w.c` has rows through `ix://I/O/w` of type `ix://I/W` and `ix://I/W/c` of type Clock.
