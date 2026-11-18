---
id: TC-3011
title: "Eligible hyper reductions execute and report HoldsReduced"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-174
    type: verifies
---
# TC-3011: Eligible hyper reductions execute and report HoldsReduced

## Description

Verify FR-174's hyper reduction eligibility rule: a key symmetry and a state
constraint selected on an eligible HP-2 item execute, keep the verdict and
report `HoldsReduced`, and a "yes" selection on an ineligible form settles
V-8. Scope: FR-174-AC-7.

## Test Procedure

1. Run `NonInterference` over the secure vault with the vaults `v` and `w`,
   once with no selection and once with the key symmetry of `vaults`
   `[[v, w]]` selected; repeat with the leaky vault.
2. Run the secure `NonInterference` with the state constraint `Low`
   (`self.l <= 1`), with `Zero` (`self.l = 0`), and with the symmetry and
   `Low` both selected.
3. Run `Opaque` (HP-3) with the key symmetry selected.

Tag every step `#[trace("TC-3011", "FR-174-AC-7")]`.

## Expected Results

- Step 1: secure: both runs hold; the selected run returns `HoldsReduced`
  with `reductions` `Symmetry{..}` (orbit `[v, w]`) then `CopySwap{[[a, b]]}`
  and stores fewer product states. Leaky: `Violated`, the counterexample
  replays and names no reduction.
- Step 2: `Low`: `HoldsReduced{[CopySwap{[[a, b]]}]}`; `Zero`:
  `ConstraintReached` with its boundary states; both: `HoldsReduced` naming
  `Symmetry` then `CopySwap`.
- Step 3: `Reduction{Unsupported{Symmetry}}` with empty run statistics.
