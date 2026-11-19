---
id: TC-3077
title: "An IEEE ordering charges the same on the source and package routes"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-397
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-3077: An IEEE ordering charges the same on the source and package routes

## Description

Verify that whether a `numeric_equal` node is an ordering part is read from the expression's shape on both routes, so a function evaluated from its checked graph and from its written package bytes writes equal charge logs.

Scope: FR-397-AC-30, FR-397-AC-31, FR-940-AC-15.

## Test Procedure

1. Check `f(p, q, r: Float64[nearest-even]): Boolean` with each FR-397-AC-30 ordering body, emit it, read the bytes back, and call `f` on the source package and on the `BarePackage` of the read, with `p` `1.0` and with `p` a quiet NaN.
2. Do the same for FR-397-AC-31's hand-written composition, its two near-miss bodies and `p < q && numericEqual(p, p)`.

Test: `fsl-eval/tests/it/ieee_ordering_route_parity.rs` `an_ordering_charges_the_same_on_the_source_and_package_routes` (step 1), `the_ordering_part_rule_reads_the_shape_on_both_routes` (step 2).

## Expected Results

- Step 1: equal outcome and charge log on both routes for every body and input.
- Step 2: the hand-written composition's log equals `p < q`'s; each near miss charges four `ieee.operands`, equal on both routes; the standalone `numericEqual(p, p)` adds exactly one `ieee.operands` to `p < q`'s log on both routes.
