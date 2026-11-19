---
id: TC-3035
title: "A hybrid provider answer settles by its own arm"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-750
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-1021
    type: verifies
---
# TC-3035: A hybrid provider answer settles by its own arm

## Description

Verify that a hybrid provider's answer is read and settled by the arm it
belongs to: a solver result by FR-250's map, a claimed refutation by
FR-450's replay with replay parity when it does not reproduce, and a
construct outside the provider's class as a located `unsupported`. Verify
that every exact number on the hybrid wire is a `Rational` object, that an
undefined-evaluation counterexample is written and read back, that a
refuted hybrid item's replay target is `state_clause` naming its claim, and
that `read_entry` reads against the model and the frame's counterexamples.
Scope: FR-750-AC-6 to FR-750-AC-11 and FR-1021-AC-3.

## Test Procedure

1. Read `proved` entries over `SlowEnough` with `horizon` in object form,
   as `"10"` and as `10`, and a trace interval with `lower` `"0"`; run an
   item with `options` `{"step": "1/16"}` against a fixture that records
   its request.
2. Register an in-process fixture answering `Unsupported` for the flow
   `v' = -a / (1 + v)` of `Moving`, then a process fixture returning the
   same as an `unsupported` entry, then an in-process fixture naming a mode
   the model lacks.
3. Register an in-process fixture answering `Refuted` with the FR-450-AC-2
   point trace against `always [0, 12] holds(h.v > 0)`; then with `before`
   `v = 1`; then a fixture answering `Solved` `Unknown(Some(trace))`.
4. Replay the FR-450-AC-5 trace against
   `always [0, 12] holds(1 / (h.v - 5) != 0)`, write the counterexample by
   `to_wire`, read it back through the plugin reader and replay it; write
   one whose instant is irrational.
5. Analyze `TooSlow` refuted; build its replay request and execute it, then
   with a trace that does not reproduce.
6. Call `read_entry` on a `refuted` entry naming counterexample 0 and 1
   with one `counterexamples` entry, on a `solver-inconclusive` trace naming
   mode `Halted`, and on a `failed` `tool-failure` entry.
7. Plan the `Fill`/`Drain` tank with `dwell_grid` 1 under the reference
   provider; repeat with a fixture provider whose trace bound is `"8"`.

## Expected Results

- Step 1: the object form reads horizon 10; the others refuse
  `invalid_result`/`wrong-value-kind` at `/basis/horizon` and at the
  interval's member; the provider receives `step` as the string `1/16`.
- Step 2: both settle `unsupported`, `unsupported_construct`/
  `expression-form`, `detail` naming `form`, `declaration`, `mode`
  `Moving`, `variable` `v`, the backend and `locus`, the flow's term
  node as a `Locus`, with no counterexample and no replay; the unknown mode settles
  `failed`, `runtime_invariant`.
- Step 3: `refuted` with one `HybridCounterexample`; then `inconclusive`,
  `ReplayParity(HybridTrace(NotReproduced{counterexample}))` with the
  fixture's complete counterexample; then `SolverInconclusive{trace,
  replay}`.
- Step 4: `where` is `{"position": 0, "time": 15/2 as a point interval,
  "locus": the division's term node}` with `cause` `division-by-zero`; it
  reads back and reproduces; the irrational instant writes `lower < upper`
  rational ends containing it; no item settles `failed` for an unspelled
  counterexample.
- Step 5: `replay` `{"target": {"state_clause": "TooSlow"}, "reproduced":
  true}`; the executor writes the `hybrid` row
  `reproduced-with-evaluated-witness`, then `inconclusive` with the
  `hybrid-trace` disagreement.
- Step 6: `Refuted`; `invalid-value` at `/counterexample`; `invalid-value`
  at `/cause/trace/start/mode`; `NotRead`, and the item settles `failed` by
  FR-290.
- Step 7: the validation item carries dwell and horizon as `Rational`
  objects; the string bound refuses `wrong-value-kind` at its member and
  the candidate is not validated.
