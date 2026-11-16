---
id: TC-2582
title: "Formatted source emits the same graph and reader verdict"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-913
    type: verifies
---
# TC-2582: Formatted source emits the same graph and reader verdict

## Description

Formatted source emits the same graph and reader verdict.

Scope: FR-913-AC-1.

## Test Procedure

For `tests/fixtures/spine-compile.native` and each other complete-V1 fixture unit that compiles, format the source, compile both under one source identity, package name and path, and read both packages with `filament_contract_model::read_checked_package`.

Tag the tests `#[trace("TC-2582", "FR-913-AC-1", "FR-003-AC-9")]`.

## Expected Results

- The two semantic graphs are equal in every member except source regions.
- The reader admits both, or refuses both with the same code.
