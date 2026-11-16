---
id: TC-2459
title: "A non-Boolean function selected as a claim is refused naming the function and its type"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-867
    type: verifies
---
# TC-2459: A non-Boolean function selected as a claim is refused naming the function and its type

## Description

Verify the `select` refusals of a `function` claim selection. Scope: FR-867-AC-1 to FR-867-AC-3; AC-3 is an inspection of the filament-examples cases.

## Test Procedure

1. Run a clause request selecting `seven` (`Integer`) over FR-108's ConfigVersion snapshot.
2. Run it selecting `yes` (`Boolean`) and the unknown name `nope`.
3. Inspect the filament-examples function-selection cases and run the examples gate.

## Expected Results

- Step 1: exit 20, stage `select`, `ill_typed`/`type-mismatch` with `function` `seven`, `expected` `Boolean`, `actual` `Integer`; the message names all three.
- Step 2: `yes` exits 0 with `truth` true; `nope` exits 20 `missing_declaration`/`missing-name` naming `nope`.
- Step 3: a Boolean case expecting exit 0 and an Integer case expecting exit 20, both run.

Test: `tests/it/function_claim_selection.rs`.
