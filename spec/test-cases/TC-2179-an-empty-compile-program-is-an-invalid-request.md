---
id: TC-2179
title: "An empty compile program is an invalid request"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-480
    type: verifies
---
# TC-2179: An empty compile program is an invalid request

## Description

An empty compile program is an invalid request.

Scope: FR-480-AC-19.

## Test Procedure

1. Send a `native-compile/1` request with `program` `[]`. Test: `tests/it/compile_command.rs` `compile_empty_program_is_invalid_request`.
2. Call `PackageDeclarations::assemble_units` with an empty unit list. Test: `fsl-semantics/tests/it/dimension_equivalence.rs` `an_empty_unit_list_breaks_the_assemblers_invariant`.

Tag the tests `#[trace("TC-2179", "FR-480-AC-19")]`.

## Expected Results

- Step 1: exit 20, code `invalid-request`, a message naming the program, no package bytes.
- Step 2: one `EmptyUnitList` error, `invalid_package`/`established-invariant-broken`.
