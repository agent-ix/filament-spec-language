---
id: TC-2993
title: "The export limits have the compile lifecycle's defaults"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: verifies
---
# TC-2993: The export limits have the compile lifecycle's defaults

## Description

Verify the three `export.*` settings, their defaults and the export's stop at one of them. Scope: FR-255-AC-22.

## Test Procedure

1. Read the settings operation's defaults for `export.output_bytes`, `export.nodes` and `export.work_units`, and for `emit.output_bytes`, `s3.nodes` and `s3.work_units`.
2. Give the settings operation `export.nodes=7`.
3. Export FR-756-AC-1's counter with `limits` `{"export.work_units": 1}`.

## Expected Results

- Step 1: 16777216, 100000 and 16777216, each equal to its compile counterpart.
- Step 2: only `export.nodes` changes, to 7.
- Step 3: `incomplete` naming `export.work_units` with bound 1, exit 22.
