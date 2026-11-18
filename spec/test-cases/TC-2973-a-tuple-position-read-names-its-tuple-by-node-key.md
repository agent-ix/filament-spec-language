---
id: TC-2973
title: "A tuple position read names its tuple by node key"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-680
    type: verifies
---
# TC-2973: A tuple position read names its tuple by node key

## Description

Verify that a checked `TupleProject` names its tuple by the tuple type's node
key, for a source tuple and a model tuple alike, and that S4 lowers it to the
package's position member. Scope: FR-680-AC-7.

## Test Procedure

1. Check `tuple Pair(Boolean, Integer);` with `p[1]` and read the checked `TupleProject` node.
2. Check `p[1]` over a model tuple `M::Slot` of a selected domain package and read its node.
3. Emit each package and read the `filament.op.tuple.project` member.

Test: `fsl-semantics/tests/it/type_decisions.rs`.

## Expected Results

- Steps 1 and 2: the node's tuple key equals the operand's `Composite` key, position 1; no `{package, node}` declaration key.
- Step 3: member `{kind: "position", declaration, position: 1}`, `declaration` the emitted tuple node.
