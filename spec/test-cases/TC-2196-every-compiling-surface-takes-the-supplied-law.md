---
id: TC-2196
title: "Every compiling surface takes the supplied law"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-767
    type: verifies
---
# TC-2196: Every compiling surface takes the supplied law

## Description

A multi-unit program, an analyze request and a monitor request each take the
caller-supplied integer-division law.

Scope: FR-767-AC-8, FR-767-AC-9, FR-767-AC-10.

## Test Procedure

1. Compile a two-unit `program` array with `"integer_division": "floor"`, then without it. Test: `fsl-command/tests/it/integer_division_request.rs` `top_level_law_reaches_every_unit_of_a_multi_unit_program`.
2. Analyze a request whose program declares `half` with `floor`, without the member and with `ceiling`. Test: `fsl-analyze/tests/it/analyze_request_wire.rs` `an_analyze_request_supplies_the_integer_division_law`.
3. Monitor a request whose program declares `half` and the `Half` clause, with `floor` and without the member. Test: `tests/it/run_wire.rs` `a_monitor_request_supplies_the_integer_division_law`.

Tag the tests `#[trace("TC-2196", "FR-767-AC-N")]` with the criterion each asserts.

## Expected Results

- Step 1: both units' `div` bind floor; without the member `missing-selection`.
- Step 2: the item settles; without the member the request refuses `missing-selection` with no item; `ceiling` refuses `unsupported-selection`.
- Step 3: a `filament.monitor-run/1` result; without the member exit 20 `missing-selection`.
