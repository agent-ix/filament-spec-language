---
id: TC-3211
title: "On-each settlement states the profile boundary"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-875
    type: verifies
---
# TC-3211: On-each settlement states the profile boundary

## Description

Verify that unsupported on-each forms are refused for their evidence/profile
boundary and not for the activation. Scope: FR-875-AC-4 to FR-875-AC-8.

## Test Procedure

1. Run FR-875's `E` over a `Link` start snapshot holding one undelivered
   message and use the admitted exact workload profile as the control.
2. With the same nonempty snapshot, request `E` under every scheduler and
   under a workload whose subject includes a free delay.
3. Request statistical evidence for `E`.
4. Give `E` a cyclic continuation graph.
5. Separately replace `E` with `always`, `until`, bounded eventuality,
   expected-reward, long-run-fraction, finite-horizon measure and quantile
   claims.
6. Separately give `E` a timed subject and an authored fairness set.

Tag each assertion `#[trace("TC-3211", "FR-875-AC-n")]` for its criterion.

## Expected Results

- Step 1 settles `proved` at `rational(9,10)` with exact value `391/400`
  for `m` and an independently accepted conditional `Ratio` certificate.
- Steps 2 to 6 each settle `unsupported` with
  `unsupported_construct`/`not-yet-implemented`, no `cause`, numeric value or
  certificate. The nonempty instance is retained, and none of these results
  names the activation or `no-instance` as the reason for being unsupported.
