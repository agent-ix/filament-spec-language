---
id: TC-2507
title: "allInstances and lookup evaluate under a state clause and a protocol slot"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-883
    type: verifies
---
# TC-2507: allInstances and lookup evaluate under a state clause and a protocol slot

## Description

Verify population reads in the `Clause` regime and in protocol slots that may read model state, and the S3 refusal in slots that bar it. Scope: FR-883-AC-1, FR-883-AC-2, FR-883-AC-3, FR-883-AC-4.

## Test Procedure

1. Explore a protocol with capture `n = size(allInstances<M::Order>())` and a `send` constraint over the same count, from states with two orders and with none.
2. Bind `lookup(orders, k)` for a present and an absent key under `empty` and `undefined`, and compare with the same lookup in a state clause.
3. Evaluate a state clause `size(allInstances<T>(p))` at `own`, under `Pre(...)`, and under `Pre(...)` with no pre side.
4. Check a `choice` guard and a `check` condition that call `allInstances`.

## Expected Results

- Step 1: 2 and enabled; 0 and not enabled.
- Step 2: the order; for an absent key, `none` and undefined, equal to the clause results.
- Step 3: the two counts; `WrongAnchor`; never `unsupported-feature`.
- Step 4: `ill_typed`/`operator-ineligible` at the call, naming the slot.
