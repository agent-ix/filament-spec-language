---
id: TC-709
title: "Timed runs sample with exact rational delays, measure timed events and replay"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-254
    type: verifies
---
# TC-709: Timed runs sample with exact rational delays, measure timed events and replay

## Description

Verify timed statistical measurement, exact delays on the `2^-q` grid, witness replay with draw recomputation, latency measures, regeneration, the exact route and time windows.

Scope: FR-254-AC-1 to FR-254-AC-9.

## Test Procedure

Fixtures: the stochastic `Rpc` variant and its `uniform` reply variant; a model that never regenerates; the strict-guard variant; the variant with no `delay` declaration on `timeout`; the `Lock` model (FR-237-AC-6) and its variants.

1. Measure `RareLate` over both variants, twice with one seed.
2. Inspect sampled delays, replay a witness, change one delay and replay; repeat with `q` 8.
3. Measure the latency, run the regenerative method on the non-regenerating model, and request `exact` evidence over the strict-guard variant; measure `RareLate` over the free-`timeout` variant; measure `RareLate` with `max_sample_steps` 1.
4. Sample the stochastic `Lock` variant.
5. Measure the claims of FR-254-AC-5 to FR-254-AC-9 with Okamoto, `alpha 0.4, beta 0.4, indifference 0.05` (`N = 184`) and seed 7, over these fixtures (one object `c: Call` with `phase: Int[0, 2]`, initially 0; `send` moves `phase` 0 to 1; each claim over `(c: M::Call)` on origin): `Send`, dense, `send` `discrete { 2 ms: 1 }`, `reply` moving 1 to 2 with `discrete { 10 ms: 1 }`, weight 1 each, and its variant with `reply` `discrete { 1 ms: 1 }`; `Idle`, `Send` without `reply`, and its variant with `send` at 5 ms; `Tick`, `time tick` on an operation `tick` (pre and post `phase = 2`) with period 2 ms, no `delay` declaration, weight 1 on `send` and `tick`.

Tag the tests `#[trace("TC-709", "FR-254-AC-n")]`.

## Expected Results

- Step 1: `measured`, `Rejected`, `q` 64 in the basis; `measured`, `Accepted`; equal verdicts.
- Step 2: exact rationals; replay and draw recomputation succeed; recomputation fails; basis records 8 and delays lie on the grid's rounded images.
- Step 3: per-sample time-stamp differences; `incomplete`, `limit-reached` naming `statistical.max_cycle_steps`; `unsupported`; `unsupported`, `NotStochastic` naming the state after `send` and `timeout`; `Incomplete(LimitReached{limit, value, setting})` naming `max_sample_steps` and 1.
- Step 4: `unsupported`, `TimeLockedSample` naming `(B, x = 1)`, no witness kept.
- Step 5: the estimates, decisions and `NotStochasticIdleTail` states FR-254-AC-5 to FR-254-AC-9 state.
