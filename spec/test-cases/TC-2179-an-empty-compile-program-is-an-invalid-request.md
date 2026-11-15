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

Send a `native-compile/1` request with `program` `[]`. Test: `tests/it/compile_command.rs` `compile_empty_program_is_invalid_request`.

Tag the tests `#[trace("TC-2179", "FR-480-AC-19")]`.

## Expected Results

- Exit 20, code `invalid-request`, a message naming the program, no package bytes.
