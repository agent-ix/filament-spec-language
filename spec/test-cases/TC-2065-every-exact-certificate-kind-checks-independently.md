---
id: TC-2065
title: "Every exact certificate kind checks independently"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-201 FR-203 FR-282
    type: verifies
---
# TC-2065: Every exact certificate kind checks independently

## Description

Verify that `fsl-probability-check`, a separate process with no `fsl-analyze` linkage, accepts every certificate kind EN-5 emits over an emitted probabilistic package, and that analyze results carry the certificate and witness.

Scope: FR-201-AC-51, FR-203-AC-8, FR-282-AC-20.

## Test Procedure

1. Produce, over packages S4 emits, an `Exact` certificate (FR-201-AC-1), `Lower` and `Upper` certificates (a value-bounds cyclic DTMC eventual proof), a `LongRun` certificate (FR-201-AC-27), an unbounded fair eventual `Exact` certificate (`FairTerminates` over a cyclic MDP), a finite-horizon quantile certificate under every scheduler and a digital-clock certificate (FSpec FR-421-AC-1).
2. Run `fsl-probability-check` on each from its bytes, the emitted package's byte provision and the subject binding; then on each with one value altered.
3. Read the `proved` record of FR-203-AC-1 and the `refuted` record of FR-203-AC-5 from an analyze result.

Tag the tests `#[trace("TC-2065", "FR-201-AC-51")]` (and `FR-282-AC-20`, `FR-203-AC-8`).

## Expected Results

- Step 2: each accepted; each altered one rejected with its FR-201 rule and locus.
- Step 3: the certificate bytes in the item result are accepted later by `fsl-probability-check`; the witness replays.
