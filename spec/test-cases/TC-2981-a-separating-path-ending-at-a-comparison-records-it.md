---
id: TC-2981
title: "A separating path ending at a comparison records the comparison, its operands and its path"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-265
    type: verifies
---
# TC-2981: A separating path ending at a comparison records the comparison, its operands and its path

## Description

Verify that the separating-witness derivation settles a decisive basis when
the decision path ends at an authored comparison, and that the record names
the comparison's occurrence key, its two operand values and the nodes the
walk visited. Scope: FR-265-AC-2, FR-265-AC-3, FR-265-AC-11 to FR-265-AC-13.

## Test Procedure

1. Derive `MatchesParent` over `high` with `self` `mid`.
2. Derive `AtLeastParent` over `high` with `self` `mid`, and `MatchesParent`
   with `self` `root`.
3. Derive `OrAll` and `EqualsAll` over `high` with `self` `mid`.
4. Evaluate `MatchesParent` and `MemberAll` and read their comparison
   reports; replay `MatchesParent`'s record.

## Expected Results

- Step 1: basis `decisive-counterexample`; one record with
  `deciding_comparison` the `=`'s occurrence key, `operands` `[600, 0]`,
  `decision_path` the `implies` then the `=`, `deciding_element` `false`, a
  `value_path` rooted at `mid` with empty `steps`, and no `quantifier`,
  `index` or `trace_position`.
- Step 2: `AtLeastParent` gives `decisive-witness` with the `>=` record,
  operands `[600, 0]`, `deciding_element` `true`; `MatchesParent` over
  `root` gives `closed-scope` and no record.
- Step 3: `OrAll` gives `decisive-counterexample` naming
  `self.versionNumber = 0` with operands 600 and 0; `EqualsAll` gives
  `decisive-counterexample` naming its root `=` with operands `false` and
  `true` and no quantifier.
- Step 4: `MatchesParent` reports exactly one comparison and `MemberAll`
  none; the replay derives the equal record.
