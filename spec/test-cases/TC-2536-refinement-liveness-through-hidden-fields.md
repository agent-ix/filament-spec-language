---
id: TC-2536
title: "Refinement liveness through hidden abstract fields is decided"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-899
    type: verifies
---
# TC-2536: Refinement liveness through hidden abstract fields is decided

## Description

Verify that refinement liveness through hidden abstract fields is decided,
per candidate run and jointly over every `F_A` constraint, on untimed, timed
and abstract protocol subjects. Scope: FR-899-AC-1, FR-899-AC-2,
FR-899-AC-6 to FR-899-AC-10.

Catches a check that requires every candidate run to be fair, or answers
`unsupported`; a check that counts a class as taken on a product edge because
some candidate takes it there (a splice of candidate runs); a check that
decides the `F_A` constraints one at a time; and a check that counts a class
as enabled because another candidate enables it.

## Test Procedure

1. Check `RegisterHistory` without `prev` and with `ensure fair weak write`, its peek-loop variant, and the variant with one fair candidate run.
2. Build FR-899-AC-6's fixture (CX-1) and check it; replay the counterexample. Then let `fire` also map `h = 2` to `h' = 1` keeping `v`, check again and replay the earlier counterexample.
3. Build FR-899-AC-7's fixture (CX-2) and check it; replay the counterexample, and replay it with the failure renamed `HiddenUnfair{fire1}`. Remove the `ensure ... fire2` row and check again.
4. Build FR-899-AC-8's fixture (CX-3) and check it; hand-build the `HiddenUnfair{fire}` counterexample for the `tick` loop and replay it.
5. Check FR-899-AC-9's four timed refinements (hidden `h` free, hidden `h` initially `false`, `strong` without and with the time invariant); replay the refutations through FR-241.
6. Check FR-899-AC-10's `CasTwice` with `peek` removed and with `peek` kept.

Tag the tests `#[trace("TC-2536", "FR-899-AC-n")]`.

## Expected Results

- Step 1: The first holds; the peek loop is `Violated`, `HiddenUnfair{write}`; the one-fair-candidate variant holds.
- Step 2: `Checked(Violated)`, `HiddenUnfair{fire}`, stem `[tick, tick]`, loop `[tick]`, replay `Reproduced`; after the edit, `Checked(HoldsReduced{reductions: []})` and the old counterexample is not reproduced.
- Step 3: `Checked(Violated)`, `HiddenUnfair{fire2}`, replay `Reproduced`; the `fire1` rename is refused; without the `fire2` row the check holds.
- Step 4: `Checked(HoldsReduced{reductions: []})`; the hand-built counterexample is not reproduced.
- Step 5: holds, `Violated` with `HiddenUnfair{send}` and a one-`poll` loop of positive delay (reproduced), `Violated`, holds; no result is `unsupported`.
- Step 6: liveness `Checked(HoldsReduced{reductions: []})`; with `peek` kept, safety refuted by divergence and `liveness` `None`; neither is `unsupported`.
