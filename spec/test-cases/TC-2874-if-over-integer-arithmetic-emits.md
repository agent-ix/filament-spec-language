---
id: TC-2874
title: "An if over integer arithmetic emits and reads back without an orphan interval"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: verifies
---
# TC-2874: An if over integer arithmetic emits and reads back without an orphan interval

## Description

Verify that an `if` over integer arithmetic (the shape of codegen's FR-015-AC-168 and AC-169 clauses) emits, is admitted by Contract IR's independent reader, and builds no interval type node that nothing names. Scope: FR-495-AC-25.

## Test Procedure

1. Compile each source of FR-495-AC-25 through the public `compile_package` with the `example/config-version` model selected.
2. Read each package with `CheckedPackageV2::read`, supplying the package's required features and the model document as evidence.
3. Count the emitted `bounded_domain` nodes whose interval maximum is the `if`'s hull maximum.

## Expected Results

- Step 1: every source compiles; none refuses `UnlocatedOccurrence`.
- Step 2: every read admits.
- Step 3: zero such nodes.

Test: `fsl-replay/src/application/division_tests.rs`
`an_if_over_integer_arithmetic_emits_and_reads_back_without_an_orphan_interval`, tagged `#[trace("TC-2874", "FR-495-AC-25")]`.
