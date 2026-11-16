---
id: TC-2245
title: "A clocks member and an override together refuse"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
---
# TC-2245: A clocks member and an override together refuse

## Description

Verify that a request carrying `clocks` and handed an override refuses before compiling. Scope: FR-640-AC-14.

## Test Procedure

1. Hand a compile request carrying `clocks` to the compile entry with an override.
2. Hand a run request carrying `clocks` to the run entry with an override.
3. Run each request without the override, and the override without the member.

## Expected Results

- Steps 1 and 2: `invalid-request` at stage `request`, exit 20, a message naming `clocks` and `--clocks`, no program compiled.
- Step 3: each proceeds past the request stage.
