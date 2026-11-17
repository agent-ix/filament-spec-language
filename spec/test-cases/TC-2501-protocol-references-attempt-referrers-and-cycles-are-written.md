---
id: TC-2501
title: "Protocol references, references to an attempt protocol and reference cycles are written"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-880
    type: verifies
---
# TC-2501: Protocol references, references to an attempt protocol and reference cycles are written

## Description

Verify the `protocol_reference` carrier in every body slot, dependency-ordered writing of an attempt protocol's referrers, and one recursion group for a cyclic component. Scope: FR-880-AC-3, FR-880-AC-4, FR-880-AC-5.

## Test Procedure

1. Compile `Flow` with `protocol Other` in its `repeat` guard, `check` constraint, `each` guard, `terminal when`, a capture initializer and its `finish` constraint.
2. Compile a function and a predicate that refer to a protocol with an `attempt`; compile the protocol alone.
3. Compile `Flow` whose `repeat` guard holds `protocol Flow`, and a predicate-join function that refers to its own protocol, beside an acyclic reference.

## Expected Results

- Step 1: one carrier per reference, each targeting `Other`'s clause; admitted.
- Step 2: the protocol follows its state clauses and precedes its referrers; its key equals the key it has alone.
- Step 3: each cyclic component shares one `recursion_group`, written in ordinal order; the acyclic reference carries none; admitted.
