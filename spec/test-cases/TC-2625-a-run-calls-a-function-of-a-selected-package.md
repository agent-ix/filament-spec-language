---
id: TC-2625
title: "A run calls a function of a selected checked package"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-945
    type: verifies
---
# TC-2625: A run calls a function of a selected checked package

## Description

Verify a native-run/1 call over a `package` selection, its request refusals,
and FR-650's package-only unsupported refusal. Scope: FR-945-AC-1,
FR-945-AC-2.

## Test Procedure

1. Compile the spine unit, select its bytes as `package`, and call `same` over the healthy current snapshot; repeat with `program` added and with a `{}` package file.
2. Select `tests/fixtures/population-collection.checked-package.json` and call `gives`, `takes` and `missing`.

Test: `tests/it/run_value_spellings.rs`.

## Expected Results

- Step 1: the source request's value and exit; `invalid-request` naming `program`; `invalid_package` at stage `request`, exit 20.
- Step 2: `unknown_required_feature`/`unsupported-feature` at stage `call`, exit 21, `{"function": "gives"}` and `{"position": 0}`; `missing_declaration` at stage `call`.
