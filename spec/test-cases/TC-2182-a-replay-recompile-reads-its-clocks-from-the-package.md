---
id: TC-2182
title: "A replay recompile reads its clocks from the package"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
---
# TC-2182: A replay recompile reads its clocks from the package

## Description

A replay recompile reads its clocks from the package.

Scope: FR-640-AC-8.

## Test Procedure

Compile FR-640-AC-1's unit with its `clocks`; rebuild a `clocks` object from the package's `temporal_bindings` rows (`clock_name` to `clock`); compile the same program with it. Test: `tests/it/run_wire.rs` `clocks_rebuilt_from_package_rows_compile_the_same_package`.

Tag the tests `#[trace("TC-2182", "FR-640-AC-8")]`.

## Expected Results

- Both compiles exit 0.
- The two packages' bytes are identical.
