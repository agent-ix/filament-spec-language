---
id: TC-2191
title: "Every compiling surface takes the supplied law"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-765
    type: verifies
---
# TC-2191: Every compiling surface takes the supplied law

## Description

The `check` operation, the Markdown fence check, a multi-unit program, an
analyze request and a monitor request each take the caller-supplied
integer-division law.

Scope: FR-765-AC-8, FR-765-AC-9, FR-765-AC-10.

## Test Procedure

1. Check FR-670-AC-1's unit without its header through `spine::check_with_integer_division` with floor, and through `spine::check`. Test: `fsl-replay/tests/it/integer_division_check.rs` `check_operation_takes_the_supplied_law`, added with the operation.
2. Check a Markdown document of two `ix:native` fences, each with a `div` function and no header, through `check_native_fences` with floor. Test: `integer_division_check.rs` `every_fence_takes_the_supplied_law`, added with the operation.
3. Compile a two-unit `program` array with `"integer_division": "floor"`, then without it. Test: `fsl-command/tests/it/integer_division_request.rs` `top_level_law_reaches_every_unit_of_a_multi_unit_program`.
4. Analyze a request whose program declares `half` with `floor`, without the member and with `ceiling`. Test: `fsl-analyze/tests/it/analyze_request_wire.rs` `an_analyze_request_supplies_the_integer_division_law`.
5. Monitor a request whose program declares `half` and the `Half` clause, with `floor` and without the member. Test: `tests/it/run_wire.rs` `a_monitor_request_supplies_the_integer_division_law`.

Tag the tests `#[trace("TC-2191", "FR-765-AC-N")]` with the criterion each asserts.

## Expected Results

- Step 1: checks under floor; without the selection `missing-selection` at each operator.
- Step 2: both fences checked.
- Step 3: both units' `div` bind floor; without the member `missing-selection`.
- Step 4: the item settles; without the member the request refuses `missing-selection` with no item; `ceiling` refuses `unsupported-selection`.
- Step 5: a `filament.monitor-run/1` result; without the member exit 20 `missing-selection`.
