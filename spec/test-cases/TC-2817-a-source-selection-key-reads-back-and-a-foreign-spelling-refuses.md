---
id: TC-2817
title: "A source selection key reads back and a foreign spelling refuses"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-995
    type: verifies
---
# TC-2817: A source selection key reads back and a foreign spelling refuses

## Description

Verify that the strict read admits source selection keys as emitted and refuses a bare or foreign spelling. Scope: FR-995-AC-3.

Catches a reader that derives the key from a name, or accepts another owner's key.

## Test Procedure

1. Read back the FR-995-AC-1 package; replace one candidate's node with `Order`, then its package with another unit's identity.

## Expected Results

- Step 1: the package is admitted; each edit refuses `invalid_package`/`invalid-value` at the candidate's `selection_key`.
