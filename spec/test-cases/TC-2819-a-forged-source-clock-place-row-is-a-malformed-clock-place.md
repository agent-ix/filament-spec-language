---
id: TC-2819
title: "A forged source clock-place row is a malformed clock place"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-996
    type: verifies
---
# TC-2819: A forged source clock-place row is a malformed clock place

## Description

Verify that the strict read joins source clock-place rows through the source rows. Scope: FR-996-AC-2.

Catches a reader that admits a source root unchecked or resolves it through a selected document.

## Test Procedure

1. Read back the FR-996-AC-1 package; then with `d`'s member identity, declaring type and member type forged, and with `w.c`'s second row naming a selected model type.

## Expected Results

- Step 1: the package is admitted; each forged row refuses `invalid_package`/`malformed-clock-place` at that row.
