---
id: TC-3133
title: "Function body losses are located alike on the source and package routes"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-531
    type: verifies
---
# TC-3133: Function body losses are located alike on the source and package routes

## Description

Verify that a value loss recorded while evaluating a function body is located
at the checker's location of the operation's position on both routes, read on
the package route from the package's `body_locations` rows, including where
lowering reshaped the body.

Scope: FR-531-AC-36, FR-531-AC-37.

## Test Procedure

1. Check the unit holding FR-531-AC-36's `f` (`sqrt(p) <= sqrt(q)` over
   `Float64[nearest-even]`) and `g` (`f(p, q, r)`), emit it, read the bytes
   back, and call `f` on the source package and on the `BarePackage` of the
   read, with `p`, `q` and `r` at `2.0`.
2. Do the same for `g`.
3. Read the `body_locations` rows of `f`'s two `sqrt` terms from the written
   bytes.
4. Remove the second `sqrt`'s row from the bytes and admit them with the strict
   reader.
5. For each FR-531-AC-37 case (`+`, `-`, `*`, `/`, `sqrt` and the float
   conversions, over `Float32` and `Float64`), call `f` with body `r <= e` and
   with body `e` on both routes.

Test: `fsl-eval/tests/it/ieee_ordering_route_parity.rs`
`an_ordering_over_square_roots_charges_the_same_on_both_routes` (steps 1 to 3),
`a_function_body_without_its_location_rows_is_refused` (step 4) and
`every_flag_raising_ieee_operation_is_located_alike_on_both_routes` (step 5).

## Expected Results

- Steps 1 and 2: on both routes `Completed(true)`, equal charge logs, and
  exactly two losses, `IeeeFlags` with the inexact flag, located at
  `Body { function: "f", index: 0 }` paths `[0]` then `[1]`; no fault.
- Step 3: the rows' paths are `[0]` and `[1]`, origin `function_body` `f`
  index `0`.
- Step 4: the reader refuses `invalid_package`/`invalid-value` at
  `/checked_term_data/body_locations`; nothing is evaluated.
- Step 5: per case, exactly one `IeeeFlags` loss with the named flags at
  `Body { function: "f", index: 0 }` path `[1]` for `r <= e` and `[]` for `e`,
  equal outcomes, losses and charge logs on both routes, and no fault.
