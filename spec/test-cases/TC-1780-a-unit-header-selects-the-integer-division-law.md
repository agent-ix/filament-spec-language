---
id: TC-1780
title: "A unit header selects the integer-division law"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-670
    type: verifies
  - target: ix://agent-ix/filament-specification/TC-1280
    type: references
---
# TC-1780: A unit header selects the integer-division law

## Description

Verify that a header `profile` naming an integer-division law admits `div`
and `rem` from source under that law, and that a unit with no law refuses
them while a `mod`-only unit needs none. Scope: FR-670-AC-1 and FR-670-AC-2.

## Test Procedure

1. For each law `euclidean`, `floor` and `truncating`, assemble from source a
   unit with `profile v = "filament.value.complete/v1";` and
   `profile d = "filament.value.integer-division.<law>/v1";` declaring the
   functions `q` and `r` of FR-670-AC-1, with no caller-supplied selection;
   check it under `CheckMode::Linked`, read
   `PackageDeclarations::integer_division`, link it and call `q(-7, 3)` and
   `r(-7, 3)`.
2. Assemble and check the same unit with the law header removed.
3. Assemble and check a unit with no law header declaring only FR-670-AC-2's
   `m`.

## Expected Results

- Step 1: each unit checks; the selection holds the law the header names;
  the calls return `(-2, -1)` under truncating, `(-3, 2)` under floor and
  `(-3, 2)` under Euclidean.
- Step 2: `missing_declaration`/`missing-selection` for the
  `IntegerDivision` role at the `div` and at the `rem`.
- Step 3: the unit checks with no law selected.
