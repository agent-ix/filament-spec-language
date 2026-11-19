---
id: TC-2820
title: "Source operations carry operation-body rows"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-997
    type: verifies
---
# TC-2820: Source operations carry operation-body rows

## Description

Verify that operation_bodies carries one row per source operation with its body clause function or no_body. Scope: FR-997-AC-1.

Catches a carrier that omits source operations or joins a candidate to the wrong row.

## Test Procedure

1. Emit fixture S and read `operation_bodies` and `Sized`'s dispatch candidates.

## Expected Results

- Step 1: six source rows; `Order.size` names `C`'s body, `Sub.size` names `B`'s, the other four are `no_body`; each candidate joins its own row.
