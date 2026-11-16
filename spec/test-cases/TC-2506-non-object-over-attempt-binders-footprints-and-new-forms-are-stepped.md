---
id: TC-2506
title: "Non-object over inputs, attempt binders, derived footprints and the newly written forms are stepped"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-882
    type: verifies
---
# TC-2506: Non-object over inputs, attempt binders, derived footprints and the newly written forms are stepped

## Description

Verify the remaining FR-882 rules. Scope: FR-882-AC-3, FR-882-AC-4, FR-882-AC-5, FR-882-AC-6.

## Test Procedure

1. Explore `over (n: Int[0, 3]) on origin` with `check` `n > 1`, and a protocol over `Boolean`.
2. Check attempt binders of type `Boolean` over operations returning `Integer`, nothing and `Boolean`.
3. Under `tso`, explore an attempt whose body defeats footprint derivation, and the same with a hand-written footprint.
4. Explore a post-test `repeat`, a `terminal when` reading `over` under `on each`, and a protocol over `Option<M::Order>`.

## Expected Results

- Step 1: four initial instances, two blocked at the `check`; deadlock freedom refuted first at `n = 0`; two initial instances over `Boolean`.
- Step 2: `ill_typed`/`type-mismatch` at the binder for the first two, with `expected` and `actual`; the third checks and is bound.
- Step 3: both explore, with equal access classes.
- Step 4: each settles; the post-test body's first step is enabled at entry.
