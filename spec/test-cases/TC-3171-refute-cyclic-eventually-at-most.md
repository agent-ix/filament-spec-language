---
id: TC-3171
title: "Refute cyclic eventually at most"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3171: Refute cyclic eventually at most

## Description

Verify FR-202-AC-41: an unwindowed `eventually` claim with a `<= θ` bound
on a cyclic workload chain is refuted with an accepting-union `lower`
certificate.

## Test Procedure

Use a workload chain: `s` does not satisfy E and moves to `s` and to `e`
with probability `1/2` each; `e` satisfies E and returns to `s`. Analyze
`probability <= 1/2 [eventually holds(E)]` with the path-set search
instrumented, and replay the counterexample. Repeat with the certificate
value at `s` set to `1/2`.

## Expected Results

The value is 1. The path-set search is never called. The counterexample
has no scheduler and a `lower` certificate with `x(s) = 1`, `x(e) = 1`,
ranks 1 and 0. Replay derives the accepting union `{e}`, checks `1 > 1/2`,
and the item settles `refuted`. The `1/2` certificate rejects
`ThresholdMissed` at the initial key and settles `inconclusive` without a
token.
