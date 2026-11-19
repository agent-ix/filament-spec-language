---
id: TC-2821
title: "A forged source operation-body row refuses"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-997
    type: verifies
---
# TC-2821: A forged source operation-body row refuses

## Description

Verify the strict read of source operation-body rows. Scope: FR-997-AC-2.

Catches a reader that accepts a missing, mismatched or foreign source body row.

## Test Procedure

1. Read back the FR-997-AC-1 package; then with each FR-997-AC-2 edit.

## Expected Results

- Step 1: the package is admitted; each edit refuses with the stated code, cause and locus.
