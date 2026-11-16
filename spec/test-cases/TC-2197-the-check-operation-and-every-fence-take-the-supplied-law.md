---
id: TC-2197
title: "The check operation and every fence take the supplied law"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-767
    type: verifies
---
# TC-2197: The check operation and every fence take the supplied law

## Description

The single-unit `check` operation and the Markdown fence check take the
caller-supplied integer-division law.

Scope: FR-767-AC-11.

## Test Procedure

The two entry points do not exist yet, so their tests land with them.

1. Check FR-670-AC-1's unit without its header through `spine::check_with_integer_division` with floor, and through `spine::check`. Test: `fsl-replay/tests/it/integer_division_check.rs` `check_operation_takes_the_supplied_law`.
2. Check a Markdown document of two `ix:native` fences, each with a `div` function and no header, through `check_native_fences` with floor. Test: `integer_division_check.rs` `every_fence_takes_the_supplied_law`.

Tag the tests `#[trace("TC-2197", "FR-767-AC-11")]`.

## Expected Results

- Step 1: checks under floor; without the selection `missing-selection` at each operator.
- Step 2: both fences checked.
