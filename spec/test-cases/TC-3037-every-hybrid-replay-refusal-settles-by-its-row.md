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
   last state; build a claim whose model index the graph lacks, a call to a
   missing function index and an ill-kinded term, and run each through
   analyze and through a replay request.
2. Run FR-193-AC-15's `on each` claim under `model-time` with a provider
   `refuted` point trace, as a `solver-inconclusive` trace, and as a replay
   request.
3. Run FR-450-AC-7's trace under a `work_units` bound of 10 through analyze
   and through a replay request.
4. Read FR-750-AC-7's process `expression-form` entry through
   `read_entry(entry, counterexamples, graph, claim)`; repeat with `locus`
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

- Step 1: the trace is refused `invalid_result`/`invalid-value` at `/loop`;
  the three faults settle `failed`, `runtime_invariant`/
  `established-invariant-broken` with invariants `hybrid-claim-model`,
  `hybrid-call-expansion` and `hybrid-term-kind`, and each replay request
  ends with exit `30` and no row.
- Step 2: `inconclusive`, `ReplayParity(HybridTrace(NotExact{counterexample}))`;
  `SolverInconclusive` with `exact` `not-exact`; the replay row
  `inconclusive` with the `not-exact` disagreement.
- Step 3: `incomplete`, `LimitReached{work_units, 10, accounting.work_units}`;
  the replay row `incomplete` with that limit.
- Step 4: `Unsupported` with `v`'s flow site in `Moving` and the division's
  `TermLocus`, never `NotRead`; then `invalid_result`/`invalid-value` at
  `/diagnostic/detail/locus` twice and at `/diagnostic/detail/mode`.
- Step 5: `division-by-zero`, `outside-parameter-range`,
  `outside-result-range` and `outside-capture-range`, each read back to its
  variant; `undefined` refused at `/kind/cause`; the containing claim settles
  `refuted`, the other two `inconclusive`,
  `ReplayParity(HybridTrace(NotReproduced{..}))`.
