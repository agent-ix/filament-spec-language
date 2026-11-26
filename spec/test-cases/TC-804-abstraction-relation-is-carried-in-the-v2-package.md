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

Scope: FR-306-AC-1, FR-306-AC-2, FR-306-AC-5.

## Test Procedure

1. Compile FR-304-AC-1's unit; read the relation node back from the emitted
   v2 bytes.
2. Recompile the unit unchanged; then change one `ObjectBinding` field's
   `RustField` and recompile.
3. Link a unit with a guarantee, and a unit with a step relation, take its
   shared graph, and bind a relation that drops an operation frame, one that
   adds it, and one that targets other nodes. Check, for each, a unit that
   declares the bound relation.

## Expected Results

1. Exactly one relation node; its keys and binding values equal the
   in-process `CheckedAbstractionRelation`'s.
2. The unchanged recompile gives an equal relation node id and body. The
   changed one gives an equal relation node id and a body whose `objects`
   entry holds the new `RustField` where the first holds the old one, and
   reads back.
3. Each guarantee refusal and relation node, and each step relation's node,
   implementation references and requirement records, equal the checked
   unit's; dropping the frame of `inc` refuses the guarantee item
   `missing_declaration`/`missing-name`; the graph shared before the binding
   is unchanged.
