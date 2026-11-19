---
id: TC-3069
title: "Relationship endpoint ends and cycles through an attempt-listed state clause are written"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-880
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: verifies
---
# TC-3069: Relationship endpoint ends and cycles through an attempt-listed state clause are written

## Description

Verify that a relationship whose ends are endpoints binds its ends to the interface object type with or without an admitted Connection, and that a cycle through a state clause an attempt lists is written as one recursion group. Scope: FR-880-AC-12, FR-880-AC-13.

Catches a checker that refuses endpoint ends without a Connection as not implemented, records a connection that was not admitted, or orders an attempt protocol after its state clauses inside a cycle instead of writing the group.

## Test Procedure

1. Compile `relationship Pipe = M::pipe;` over a model whose `pipe` joins endpoints `out` and `inp` of interface `Port`, without and then with the Connection admitted; read both back. Add `related by Pipe (a, b)` with `a: Reference<Other>`.
2. Compile a protocol `P` whose `attempt` lists the precondition `S`, `S` calling `f`, `f` holding `protocol P`; read it back. Compile the same unit with `f` not holding `protocol P`. Remove `S`'s `recursion_group` from the first package and read it.

## Expected Results

- Step 1: ends `Reference<Port>` twice, no `connection` without the Connection and the full connection with it; both admitted; no `not-yet-implemented`; the `related by` clause refuses `ill_typed`/`type-mismatch`, `expected` `Reference<Port>`.
- Step 2: `P`, `S`, `f` and the carrier share one `recursion_group`, FR-092 ordinals, written in ordinal order, admitted; the acyclic unit has no group and `P` follows `S`; the edited package refuses `invalid_semantic_graph` at `S`'s node.
