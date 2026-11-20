---
id: TC-1221
title: "Hybrid traces replay statically and exactly against the checked hybrid model"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-450
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-290
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-269
    type: verifies
---
# TC-1221: Hybrid traces replay statically and exactly against the checked hybrid model

## Description

Verify static replay, exact replay and the settlement each gives, for a
trace from a `solver-inconclusive` cause and from a provider's `refuted`
entry.

Scope: FR-450-AC-1 to FR-450-AC-13, FR-290-AC-19 and FR-269-AC-5.

## Test Procedure

Fixtures: FR-447's `Braking` and `SlowEnough`; the `Bounce` declaration of
FR-450-AC-6; AC-8's `descending` wrapper, recursive `descend` function and
`Bounce` variant; a plugin fixture provider (FR-290) and an in-process fixture
provider, each returning a scripted entry with a scripted `HybridTrace`.

1. Return each `solver-inconclusive` trace FR-450-AC-1 lists, then the trace
   with `delta` `-1/100`.
2. Return AC-2's point trace as a `solver-inconclusive` trace and as a
   provider's `refuted` entry, against `always [0, 12] holds(h.v > 0)`; then
   against `SlowEnough`.
3. Return each altered trace FR-450-AC-3 lists, as a provider's `refuted`
   entry.
4. Replay AC-2's trace over each changed flow FR-450-AC-4 lists.
5. Replay AC-2's trace against each claim FR-450-AC-5 lists, and statically replay the guard calling the recursive function.
6. Replay each `Bounce` trace FR-450-AC-6 lists, with and without
   `fair Up;`.
7. Replay the `loop_to` 0 point lasso from AC-6 against
   `eventually [999999, 1000000] holds(h.x = 2)` from a provider's `refuted`
   entry with `work_units` 100000.
8. Statically replay the `Up`-to-`Down` point trace over AC-8's `Bounce`
   variant, after S3 admits `descending(x)` as a hybrid guard call, first with
   `work_units` 100 and then with `work_units` `u64::MAX`.
9. Over `Braking` with `Moving`'s flow `v' = -v`, submit AC-2's point trace
   as a provider's `refuted` entry.
10. For a process provider's `refuted` entry on the hybrid `Braking` claim,
    read AC-2's named `hybrid-counterexample` in FSpec FR-193 wire form and
    settle it; repeat with a malformed trace and on a non-hybrid claim.
11. Serialize Step 3's `NotReproduced` hybrid cause and Step 9's `NotExact`
    hybrid cause through the shared `DisagreementCause` codec with the
    checked `Braking` model, then read each cause with that model and the
    checked package; repeat a read with no model context.
12. Replay each `end`-cut `Braking` trace FR-450-AC-10 lists, and the trace
    carrying both `end` and `loop_to`.
13. Over FSpec FR-652-AC-3's `Grow`, submit its point trace as a provider's
    `refuted` entry with the nine-step `validated-witness`, then with that
    witness cut after its step at `[28, 32]`.
14. Over FR-750-AC-12's `Shuttle` and its point lasso `L`, replay each `on each` claim FR-450-AC-12 lists under `model-time`: the capture `v0: Rational[0, 1; 1, 2] = q.v + 1/2`, out of range exactly for `τ` in `(1/2, 1)`, with provider entries claiming `where.time` `[1/2, 1/2]` and `[3/4, 3/4]`, the `historically` and `once` bodies, and the two-period `not once` body. Then replay FR-450-AC-12 (d)'s `Phase` lasso against `once [0, 1] once [0, 1] holds(h.w = 1)`.
15. Over FSpec FR-652-AC-3's `Grow`, submit its point trace as a provider's
    `refuted` entry with the nine-step witness claiming kind `Formula`, then
    claiming `UndefinedEvaluation`, then claiming `Formula` under a
    `work_units` bound that lets replay finish but stops the witness check;
    then submit FSpec FR-193-AC-17's reproducing `Braking` trace as a
    `refuted` entry carrying a structurally valid witness FR-751 would reject.

Run each step through both fixture providers.

Tag the tests `#[trace("TC-1221", "FR-450-AC-n")]`.

## Expected Results

- Step 1: each static outcome and `inconclusive` record FR-450-AC-1 states;
  the negative `delta` refused at `/delta`.
- Step 2: `Reproduced`, kind `Formula`, the item `refuted` with the trace as
  its counterexample and one `counterexamples` entry, from both entries;
  `NotReproduced` at `Claim` against `SlowEnough`, the item `inconclusive`.
- Step 3: `NotReproduced` at `Dwell` for each; each settles with
  `ReplayParity(DisagreementCause::HybridTrace(NotReproduced{counterexample}))`
  retaining its input, with no `counterexamples` entry.
- Step 4: `NotExact` and `inconclusive`; exact replay with the stated
  polynomials and `NotReproduced` at `Dwell`; `NotExact`.
- Step 5: `Reproduced`, kind `UndefinedEvaluation` at time 15/2, the item
  `refuted` with cause `UndefinedEvaluation{where, cause}`; `Reproduced`,
  kind `Formula`, decisive at time 1499/200; `NotExact`; static replay
  terminating with the call's result range.
- Step 6: `Reproduced` with and without the fairness constraint;
  `NotReproduced` at `Loop` for `loop_to` 1 and at `Jump` for the `x = 1/2`
  trace; the `loop_to` 2 trace refused at `/loop`.
- Step 7: the 2-second lasso requires at least 500,000 `ClaimSegment`s to
  reach the interval's upper endpoint at 1,000,000 seconds. At
  `work_units` 100000, the meter cannot cover all segment charges and
  exhausts before deciding the formula; replay therefore returns
  `HybridReplayRefusal::ResourceExhausted{counter: WorkUnits}` and the item
  settles `incomplete`, `LimitReached{limit: work_units, value: 100000,
  setting: accounting.work_units}`, with no verdict or counterexample.
- Step 8: S3 admits the hybrid guard call `descending(x)` because the call
  boundary takes a rational argument and `descending` has a rational
  parameter; the wrapper's body calls `descend(20000)`, so the recursive
  integer call remains outside the hybrid-term boundary. At `work_units` 100,
  static replay returns
  `HybridReplayRefusal::ResourceExhausted{counter: WorkUnits}` and the item
  settles `incomplete`, `LimitReached{limit: work_units, value: 100,
  setting: accounting.work_units}`, with no verdict or counterexample; under
  `u64::MAX`, FSpec FR-146 admits the `Int[0, 20000]` measure, the call takes
  20,000 recursive edges (20,001 recursive `descend` calls plus the one
  `descending` wrapper call) and completes on the explicit stack,
  static replay returns `Consistent`, and no stack overflow occurs.
- Step 9: exact replay returns `NotExact`; the item settles with the distinct
  `ReplayParity(DisagreementCause::HybridTrace(NotExact{counterexample}))`
  cause, preserving the submitted counterexample, and writes no
  `counterexamples` entry.
- Step 10: the valid process entry maps to the same typed counterexample as
  the in-process fixture and settles by exact replay; the malformed trace is
  refused before a terminal record is built, and the hybrid arm on a
  non-hybrid claim is malformed.
- Step 11: the two causes round-trip to equal, distinct typed arms; the
  read without a checked model is refused. The existing verdict, no-value
  and witness cause codec round trips remain unchanged.
- Step 12: `Reproduced` with the bad prefix ending at `end`; `NotReproduced`
  at `/end`; `Inconsistent` at check `End`; refused at `/end`.
- Step 13: `refuted` with the trace as its counterexample, one
  `counterexamples` entry;
  `inconclusive`, `ReplayParity(HybridTrace(NotExact))`, with no
  `counterexamples` entry.
- Step 14: never `NotExact`; the capture claim `Reproduced`, `UndefinedEvaluation`, `outside-capture-range`, `where.position` 0, `where.time` `[1/2, 1/2]`, the `[1/2, 1/2]` entry `refuted` and the `[3/4, 3/4]` entry `NotReproduced` at check 5; `historically` `Reproduced` with kind `Formula` and `once` `NotReproduced` at check 5; the `not once` body `Reproduced` with kind `Formula`; the nested `once` claim decided over the prefix and three periods, `Reproduced` with kind `Formula`, the item `refuted`.
- Step 15: `refuted` through `HybridVerdict::Refuted` with one `counterexamples` entry and `reproduced-with-validated-witness`; `inconclusive`, `ReplayParity(HybridTrace(NotExact))`, no `counterexamples` entry; `incomplete`, `LimitReached{work_units}`, with `certificate-rejected{work-limit}`; `refuted` by exact replay, the witness unchecked.
- Both fixture providers give the same outcomes.
