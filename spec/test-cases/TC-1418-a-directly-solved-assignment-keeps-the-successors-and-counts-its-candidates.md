---
id: TC-1418
title: "A directly solved assignment keeps the successors and counts its candidates"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: verifies
---
# TC-1418: A directly solved assignment keeps the successors and counts its candidates

## Description

Verify that an eligible deterministic assignment generates and counts only
the candidates its solved value admits, keeps the successor set, candidate
order, findings and stops of the enumeration, and that every ineligible form
enumerates. Scope: FR-120-AC-29 to FR-120-AC-33.

## Test Procedure

Steps 1 to 4 are integration tests in `fsl-eval/tests/it/` through
`ModelSystem::new` and `explore_model`, over TC-471's compile path and the
`Cell` fixture of FR-120-AC-29 (`a`, `b: Int[0, 99]`), each case a model with its one operation, because the budget counts a whole expansion. The charge of an
expansion is read as the smallest `max_candidates` under which it
completes, found by bisection; no counter is exposed. Step 5 is the
`fsl-bench` smallest-size test of the `mc/frame_candidates` axis, which
QSL-1906 creates; it cannot run before that ticket lands, and steps 1 to 4
do not depend on it.

1. `swap()` from `a` 3, `b` 5; read the successors and the charge.
2. `step()` with `post A { self.a = pre(self.a) + 1 }` and with the
   reference clause `... and true`: compare the successor sets of state
   keys, effects and findings, read both charges, explore both to depth 3,
   and run both with `max_candidates` 300.
3. Each ineligible form of FR-120-AC-31, including the `deletes` frame, with
   `max_candidates` 1,000; then the out-of-domain value case.
4. `self.a = pre(self.a) / 0`, and the meter budget `work_units` 0 case,
   each against its `... and true` reference form.
5. The `mc/frame_candidates` axis's smallest-size test over both forms.

Tag each test `#[trace("TC-1418", "FR-120-AC-n")]`.

## Expected Results

- Step 1: one successor (`a` 5, `b` 3); charge 2: it completes with 2 and
  stops with `MaxCandidates` with 1.
- Step 2: 100 successors equal as key sets to the reference form's, with
  equal identities, effects and findings; charges 101 and 10,001; equal
  explorations to depth 3; the solved form completes at 300 and the
  reference stops with `MaxCandidates`.
- Step 3: each ineligible form stops with `MaxCandidates`; the out-of-domain
  value gives no successor, no finding and charge 1.
- Step 4: each result equals its reference form's: the `ContractUndetermined`
  finding with the same charge, and the `EvaluationMeter` stop naming
  `work_units`.
- Step 5: charges 2 and 101 solved, 10,001 enumerated for each, and equal
  successor sets; no timing is asserted.
