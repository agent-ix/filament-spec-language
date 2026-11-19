---
id: TC-3039
title: "Case binders print under their declared names"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-400
    type: verifies
---
# TC-3039: Case binders print under their declared names

## Description

Verify that a `case` in a `holds` leaf prints its arms in member declaration
order with each binder under its declared name, that nested binders keep
their scopes, that `_` prints as an ordinary binder, that renaming a binder
changes the atom text, and that the source and package routes give the same
names and byte-equal `atoms`. It also verifies that the checker's own IEEE ordering binder never prints
and that an author's `ieee_operand_0` prints as written. Scope:
FR-931-AC-13, FR-931-AC-14, FR-400-AC-13.

## Test Procedure

1. Over `Low` with the unit's `union Shape { Circle(Integer), Rect(Integer, Integer), Empty }`, lower the leaf with arms written `Empty`, `Rect(w, h)`, `Circle(r)` and read `atoms`.
2. Lower the nested leaf with `let k`, the `Rect(_, h)` arm and the inner `case`.
3. Lower step 1's leaf with `w` renamed to `width`.
4. Repeat steps 1 to 3 over each unit's checked package selected as `package`, with the source removed.
5. Read each `case` and `let` binding site through the in-process read and the reader-backed read and compare the declared names; evaluate each leaf with every binder renamed.
6. Lower FR-931-AC-14's three float and `let` leaves from source and from the package.

Tests: `tests/it/request_surface_s1_16.rs` `case_binders_print_under_their_declared_names`
(steps 1 to 3), `case_binders_match_across_the_source_and_package_routes` (steps 4
and 5) and `the_checker_ieee_operand_binder_never_prints` (step 6).

## Expected Results

- Step 1: the atom text FR-931-AC-13 states, arms `Circle`, `Rect`, `Empty`.
- Step 2: the nested atom text FR-931-AC-13 states, `_` printed as written.
- Step 3: `Rect(width, h): width + h <= 4`; the atom text differs from step 1.
- Step 4: byte-equal `atoms` to the source route for each unit.
- Step 5: equal names in equal positions on both reads; equal outcomes and charges after renaming.
- Step 6: the atom texts FR-931-AC-14 states, byte-equal across both routes.
