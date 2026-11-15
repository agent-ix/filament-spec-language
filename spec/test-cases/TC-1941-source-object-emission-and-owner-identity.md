---
id: TC-1941
title: "Source object emission, I2 read and owner identity"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: verifies
---
# TC-1941: Source object emission, I2 read and owner identity

## Description

Verify that, once ADR-013 QC-30 lands, S4 emits a unit declaring source object
types as a package FSL's I2 reader admits, and that a node mixing the source
and model owner forms is refused. Scope: FR-569-AC-14.

## Test Procedure

1. Emit TC-1408 fixture S and read it back through FSL's I2 reader; read
   `Order`'s node.
2. Replace `Order`'s `owner` with a `ModelOwner` naming fixture D's package and
   IR node of `Order`; read back.
3. Remove `Order`'s `declaration`, keeping its `SourceOwner`; read back.
4. Emit a unit whose object `O` has a field of each FR-569-AC-12 type beside a
   record `R` with the same field; compare the two bindings' type nodes.

## Expected Results

Step 1 is admitted and `Order`'s node has `declaration` `["Order"]` and the
unit's `SourceOwner`. Steps 2 and 3 each refuse `malformed_wire` at the node's
`owner`. In step 4 each binding of `O` names the type node `R`'s binding
names.
