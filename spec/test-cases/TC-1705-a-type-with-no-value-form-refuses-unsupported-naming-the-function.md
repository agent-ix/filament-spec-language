---
id: TC-1705
title: "A type with no value form refuses unsupported naming the function"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-1705: A type with no value form refuses unsupported naming the function

## Description

A type with no value form refuses unsupported naming the function.

Scope: FR-650-AC-6.

## Test Procedure

Run a function whose declared result type has no FSpec FR-181 form, and one with that type as a parameter.

Tag the tests `#[trace("TC-1705", "FR-650-AC-6")]`; they live in `tests/it/run_values.rs`.

## Expected Results

Each refuses `unknown_required_feature`/`unsupported-feature` at stage `call`, exit 21, non-null `cause`, `details` `{"function": F}` and `{"position": P}` respectively.
