---
id: TC-1701
title: "Integers beyond i64 pass only as tagged decimal strings"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-1701: Integers beyond i64 pass only as tagged decimal strings

## Description

Integers beyond i64 pass only as tagged decimal strings.

Scope: FR-650-AC-2.

## Test Procedure

Run `id_big` with `{"integer": "9223372036854775808"}`; then with each `value` `9223372036854775808`, `"7"`, `1.5`, `true`, `null` and `[1]`.

Tag the tests `#[trace("TC-1701", "FR-650-AC-2")]`; they live in `tests/it/run_values.rs`.

## Expected Results

The first completes `{"integer": "9223372036854775808"}`, exit 0. Each other refuses `invalid-request` at stage `request`, exit 20, no result document, naming `/call/arguments/0/value`.
