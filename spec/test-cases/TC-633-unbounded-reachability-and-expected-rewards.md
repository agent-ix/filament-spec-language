---
id: TC-633
title: "Unbounded reachability and expected rewards are decided by graph precomputation, interval iteration and exact policy iteration"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-198
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-198-AC-6
    type: verifies
---
# TC-633: Unbounded reachability and expected rewards are decided by graph precomputation, interval iteration and exact policy iteration

## Description

Verify the qualitative step, `+∞` expected rewards, interval iteration's two sequences, the exact policy-iteration fallback at equality and its budget, and timed elapsed extrema under an authored fairness set.

Scope: FR-198-AC-1 to FR-198-AC-6.

## Test Procedure

Fixtures: `Coin`; `Link` with `Deliver`'s unbounded variant and the `cost` claims; the `Loop` model of FR-198-AC-3; the `Mec` model of FR-198-AC-5; ADR-028 §15.5's timed `Retx`.

1. Decide `Terminates` (minimum and maximum) and `Link`'s unbounded delivery.
2. Decide the `cost` claims to `delivered or attempts = 2` and to `delivered`.
3. Run interval iteration on `Loop` and read each sweep; run with `max_iterations` 3 at threshold `1/2`; decide under a workload.
4. Run step 3's equality case with `max_policy_iterations` 0; run one request twice.
5. Compute the maximum over `Mec`, its witness policy, and decide `probability <= 0.25`; replay the evidence.
6. Over ADR-028 §15.5 `Retx`, submit both exact claims:
   `probabilistic MeanLow using timed on Retx under every scheduler fair strong Msg::send on origin { expected elapsed until holds(m.delivered) >= 1 ms }`
   and
   `probabilistic MeanHigh using timed on Retx under every scheduler fair strong Msg::send on origin { expected elapsed until holds(m.delivered) <= 3 ms }`.
   Check each emitted `timed-product/v1` document in a separate
   `fsl-probability-check` process, inspect its optimization and fairness
   members, then remove the fairness record from one document and check it
   again.

Tag the tests `#[trace("TC-633", "FR-198-AC-n")]`.

## Expected Results

- Step 1: minimum 0 with no iteration and policy `wait`; maximum 1; minimum `24/25`.
- Step 2: `11/10` and `6/5` exact, the `<= 6/5` bound met at equality; `+∞`.
- Step 3: rising lower and falling upper bounds around `1/2`; `1/2` decided at equality; one exact solve.
- Step 4: `Stopped(MaxPolicyIterations)`; equal results and policies.
- Step 5: maximum `1/2`; the policy takes `b_exit`; `refuted`, and the evidence replays.
- Step 6: `MeanLow` settles `proved` with exact minimum `10/9 ms`, and
  `MeanHigh` settles `proved` with exact maximum `20/9 ms`; each document has
  the `minimum` or `maximum` optimization stated by its claim and exactly the
  checked `[fair strong Msg::send]` record. Each separate checker accepts its
  document. Removing the fairness record causes `objective mismatch` before
  the values are accepted.
