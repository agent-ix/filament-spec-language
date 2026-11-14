---
id: TC-1418
title: "A directly solved assignment keeps the successors and counts its candidates"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: verifies
---
# TC-1418: A directly solved assignment keeps the successors and counts its candidates

## Description

Verify that an eligible deterministic assignment generates and counts only
the candidates its solved value admits, keeps the successor set, order,
findings and stops of the enumeration, and that every ineligible form
enumerates. Scope: FR-120-AC-29 to FR-120-AC-33.

## Test Procedure

Integration tests in `qsl-eval/tests/it/` through `ModelSystem::new` and
`explore_model`, over TC-471's compile path and the `Cell` fixture of
FR-120-AC-29 (`a`, `b: Int[0, 99]`).

1. `swap()` from `a` 3, `b` 5; read the successors and the smallest
   `max_candidates` under which the expansion completes.
2. `step()` with `post A { self.a = pre(self.a) + 1 }` and with the
   reference clause `... and true`: compare the successors, effects and
   findings, explore both to depth 3, and run both with `max_candidates`
   300.
3. Each ineligible form of FR-120-AC-31 with `max_candidates` 1,000; then the
   out-of-domain value case.
4. `self.a = pre(self.a) / 0` and the meter budget `work_units` 0 case.
5. The `mc/frame_candidates` axis's smallest-size test over both forms.

Tag each test `#[trace("TC-1418", "FR-120-AC-n")]`.

## Expected Results

- Step 1: one successor (`a` 5, `b` 3), one candidate; completes at the
  charge, stops with `MaxCandidates` one below, and completes with 1,000.
- Step 2: 100 successors in ascending `b`, equal to the reference form's
  after filtering, with equal keys, identities, effects and findings; equal
  explorations to depth 3; the solved form completes at 300 and the
  reference stops with `MaxCandidates`.
- Step 3: each ineligible form stops with `MaxCandidates`; the out-of-domain
  value gives no successor, no candidate and no finding.
- Step 4: the enumeration's `ContractUndetermined` finding, and the
  `EvaluationMeter` stop naming `work_units`.
- Step 5: counts 1 and 100 against 10^4 and equal successor key sequences;
  no timing is asserted.
