---
id: TC-804
title: "The abstraction relation is emitted as a v2 node of the checked package"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-306
    type: verifies
---
# TC-804: The abstraction relation is emitted as a v2 node of the checked package

## Description

Scope: FR-306-AC-1, FR-306-AC-2.

## Test Procedure

1. Compile FR-304-AC-1's unit; read the relation node back from the emitted
   v2 bytes.
2. Recompile the unit unchanged; then change one `ObjectBinding` field's
   `RustField` and recompile.

## Expected Results

1. Exactly one relation node; its keys and binding values equal the
   in-process `CheckedAbstractionRelation`'s.
2. The unchanged recompile gives an equal relation node id and body. The
   changed one gives an equal relation node id and a body whose `objects`
   entry holds the new `RustField` where the first holds the old one, under the
   same package identity, and reads back.
