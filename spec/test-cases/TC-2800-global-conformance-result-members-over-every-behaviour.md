---
id: TC-2800
title: "Global conformance result members over every behaviour"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-990
    type: verifies
---
# TC-2800: Global conformance result members over every behaviour

## Description

Verify the `obligations`, `closure`, `global` and `projections` members of a `global-conformance` item decided over every behaviour. Scope: FR-990-AC-1, FR-990-AC-2, FR-990-AC-5.

Catches a single label standing in for per-obligation results, a closure record derived from the run finishing, and a limited run reported `proved`.

## Test Procedure

1. Request `global-conformance` with no recording on the FR-636 order fixture.
2. Request it on the fixture with the `receive` removed.
3. Request it on the order fixture with `max_states` 1.

## Expected Results

- Step 1: one entry per obligation, causal/enabledness/channel `satisfied` `by-construction`, the rest `satisfied` `explored`; `closure` `closed`, producer `engine`, one execution and two branch premises; `global` `proved`, `closed-scope`, `complete` true; projections for `Buyer` and `Seller`.
- Step 2: `global` `refuted`, `complete` false; closure obligation `violated`, cause `counterexample`, witness indexing the counterexample; `closure.state` `open`.
- Step 3: undecided obligations `pending`, `closure.state` `incomplete`, `complete` false, never `proved`.
