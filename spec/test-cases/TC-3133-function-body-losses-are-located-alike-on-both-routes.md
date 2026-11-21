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

Verify that a value loss or stop raised at a term of a function body is
located at the checker's location of that position on both routes, read on
the package route from the package's `body_locations` rows, including where
lowering reshaped the body, and that clause function bodies carry their rows.

Scope: FR-531-AC-36, FR-531-AC-37, FR-531-AC-38, FR-531-AC-39, FR-531-AC-40, FR-531-AC-41.

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
5. For each FR-531-AC-37 row, over `Float32[nearest-even]` and
   `Float64[nearest-even]`, call that row's `f` (`r <= e`) and `g` (`e`) on
   both routes with the stated inputs.
6. Call step 1's `f` on both routes with every `work_units` limit from `0` to
   `U - 1`, `U` the source route's completed usage.
7. Emit FR-531-AC-39's `Config` unit (the three `size` bodies, `pre rootCall`
   on `probe` calling `self.size()`, and `pre sqrtPre` on `size`), read the
   clause function's `body_locations` rows, admit the bytes, then remove the
   second `sqrt`'s row and refuse them, and refuse them again with the
   carrier emptied.
8. Emit FR-531-AC-40's unit (`ConfigVersion` a source object type, its
   `SourceOwner` identity `I`), read the clause function's rows' `name`,
   search the bytes for `ix://source/`, then evaluate the clause function at
   `size`'s pre-state on both routes, once to completion and once under every
   `work_units` limit below its spend.
9. Emit FR-531-AC-41's two separate programs (100,000 `+` applications and 100,000 genuinely incrementing nested `let`s) on a 512 KiB stack under finite explicit caller ceilings and linear bounds chosen from the checked input before emission, record each complete canonical byte length, relative entry count and charged native strict-read and lending-cursor work, compare the rows with FSpec's BL-D1 and BL-D2, admit both packages and compare each lent typed path with the checker's path without retaining all expanded paths; repeat emission with only `emit.output_bytes` changed to its published default, and admit one row rewritten with a whole `path`.

Test: `fsl-eval/tests/it/ieee_ordering_route_parity.rs`
`an_ordering_over_square_roots_charges_the_same_on_both_routes` (steps 1 to 3),
`a_function_body_without_its_location_rows_is_refused` (step 4),
`every_flag_raising_ieee_operation_is_located_alike_on_both_routes` (step 5),
`a_denied_charge_in_a_body_is_located_alike_on_both_routes` (step 6);
`tests/it/dispatch_emission.rs`
`a_clause_function_body_carries_its_location_rows` (step 7) and a new
`a_source_clause_function_is_named_alike_on_both_routes` (step 8).

## Expected Results

- Steps 1 and 2: on both routes `Completed(true)`, equal charge logs, and
  exactly two losses, `IeeeFlags` with the inexact flag, located at
  `Body { function: "f", index: 0 }` paths `[0]` then `[1]`; no fault.
- Step 3: the rows' paths are `[0]` and `[1]`, origin `function_body` `f`
  index `0`.
- Step 4: the reader refuses `invalid_package`/`invalid-value` at
  `/checked_term_data/body_locations`; nothing is evaluated.
- Step 5: per call, exactly one `IeeeFlags` loss with exactly the row's flags,
  at `Body { function: "f", index: 0 }` path `[1]` for `f` and
  `Body { function: "g", index: 1 }` path `[]` for `g`; equal outcomes, losses
  and charge logs on both routes; no fault.
- Step 6: at every limit, `Incomplete` naming `work_units` on both routes with
  equal `Evaluation.location` and charge logs; path `[1]` of `f` occurs among
  the locations.
- Step 7: one row per term of the clause function's body walk under its
  `Body` owner, the `sqrt` rows at `[0]` and `[1]` equal to the checked
  graph's locations, no row for any `size` body; admitted, then refused at
  `/checked_term_data/body_locations` without the row.
- Step 8: every row's `name` is one `F` beginning `ix://I/`; no byte holds
  `ix://source/`; two inexact `IeeeFlags` losses at `Body { function: F,
  index: k }` paths `[0]` and `[1]` on both routes, the same `F` and `k`;
  equal `Evaluation.location` at every limit.
- Step 9: both complete packages emit under the predeclared explicit ceilings, with independently recorded byte, relative-entry and native-work totals within their declared linear bounds, their rows equal BL-D1's and BL-D2's, both strict reads admit and the lent typed paths equal the checker's paths; each default-limit emission instead refuses `OutputLimit` with `emit.output_bytes` bound 16,777,216 and first unpaid byte 16,777,217, with no partial package, and the whole-`path` row is `malformed_wire` at its `location`.
