---
id: TC-3037
title: "Every hybrid replay refusal settles by its row"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-750
    type: verifies
---
# TC-3037: Every hybrid replay refusal settles by its row

## Description

Verify that each hybrid replay refusal settles on the analyze path and the
replay path as FR-750's "Replay refusals" table states, that a hybrid result
entry's unsupported-construct locus is resolved through the checked graph,
and that every hybrid undefined cause has one catalogued spelling that round
trips. Scope: FR-750-AC-12, FR-750-AC-13 and FR-750-AC-14.

## Test Procedure

1. Over `Braking`, feed a provider `refuted` trace whose `loop_to` is past its
   last state, through analyze and inside a replay request; build a claim whose model index the graph lacks, a call to a
   missing function index and an ill-kinded term, and run each through
   analyze and through a replay request.
2. Over FR-750-AC-12's `Shuttle` and its point lasso `L` (`loop_to` 0, period
   1 s), run as provider `refuted` claims on `L`, through analyze and as
   replay requests: (a) the FSpec FR-193-AC-15-form capture claim
   `on each (q: Shuttle) when (q.v > 0) { capture v0: Rational = q.v;
   eventually [0, 1] holds(h.v >= v0 + 1/2) }`, then with body
   `eventually [0, 1] holds(h.v >= v0)`; (b) `on origin { fair Moving;
   eventually [0, *] holds(h.v > 1) }`, then with `fair Moving::Stopped;`,
   then with `fair Moving::Stopped;` and the `Moving -> Stopped` reset
   `v <= 1/2`. Run FR-450-AC-8's recursive-call guard
   with an unexhausted meter as a provider `refuted` trace, as a
   `solver-inconclusive` trace and as a replay request.
3. Run FR-450-AC-7's trace under a `work_units` bound of 10 through analyze
   and through a replay request.
4. Read FR-750-AC-7's process `expression-form` entry through
   `read_entry(entry, counterexamples, graph, claim, wire)`, its `locus` the
   division's emitted node id; repeat with the division's checked-graph index
   as `locus`, with a node id the wire lacks, with `locus`
   naming a guard node of `Moving`, with a node outside the model, and with
   `mode` `Halted`.
5. Reproduce each `UndefinedCause` variant (division by zero, `drag` called
   with `s = 120`, a `Rational[0, 1]` result exceeded, a capture
   `v0: Rational[0, 10]` initialized at 12), write it with `to_wire` and read
   it back; read a `cause` `undefined`. Compare a provider claim at the
   reproduced position, locus and cause whose `time` contains the reproduced
   irrational instant, then one with another cause and one whose `time`
   excludes the instant.

## Expected Results

- Step 1: on analyze the trace is refused `invalid_result`/`invalid-value`
  at `/loop`, and the replay request is refused `ReplayRefusal::HybridTrace`,
  `invalid_runtime_input`/`invalid-value` at its `/loop` pointer;
  the three faults settle `failed`, `runtime_invariant`/
  `established-invariant-broken` with invariants `hybrid-claim-model`,
  `hybrid-call-expansion` and `hybrid-term-kind`, and each replay request
  ends with exit `30` and no row.
- Step 2: (a) obligations activated at `τ` in `(0, 1/2]` of each period
  hold and those at `τ` in `(1/2, 1)` are false, so the first claim
  reproduces with kind `formula`, settles `refuted` with one counterexample
  and its replay row is `reproduced-with-evaluated-witness`; the `v0` body
  holds for every `τ`, `NotReproduced` at check 5, `inconclusive`,
  `ReplayParity(HybridTrace(NotReproduced{..}))`. (b) `Moving -> Stopped` is
  enabled at every instant of every period; `fair Moving;` is taken every
  period by the self-loop, so `L` is fair and the claim settles `refuted`;
  `fair Moving::Stopped;` is never taken, so `L` is unfair, `NotReproduced`
  at check 5, `inconclusive`; with the reset `v <= 1/2` the enabled set is
  empty and the claim settles `refuted`. The recursive-call trace gives `inconclusive`,
  `ReplayParity(HybridTrace(NotExact{counterexample}))`, `SolverInconclusive`
  with `exact` `not-exact`, and a replay row `inconclusive` with the
  `not-exact` disagreement.
- Step 3: `incomplete`, `LimitReached{work_units, 10, accounting.work_units}`;
  the replay row `incomplete` with that limit.
- Step 4: `Unsupported` with `v`'s flow site in `Moving` and the division's
  `TermLocus`, decoded through the wire graph, never `NotRead`; then
  `invalid_result`/`invalid-value` at `/diagnostic/detail/locus` four times
  (the checked-graph index resolves through the wire to another node, which
  is no term of `v`'s rate) and at `/diagnostic/detail/mode`.
- Step 5: `division-by-zero`, `outside-parameter-range`,
  `outside-result-range` and `outside-capture-range`, each read back to its
  variant; `undefined` refused at `/kind/cause`; the containing claim settles
  `refuted`, the other two `inconclusive`,
  `ReplayParity(HybridTrace(NotReproduced{..}))`.
