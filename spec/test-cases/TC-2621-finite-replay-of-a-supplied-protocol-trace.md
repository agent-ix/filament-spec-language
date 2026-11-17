---
id: TC-2621
title: "A finite-replay item reads the trace before it replays it"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-941
    type: verifies
---
# TC-2621: A finite-replay item reads the trace before it replays it

## Description

Verify bare-claim proof over a supplied trace and prefix, the read-before-replay
order of shape defects, and the stale-step decline. Scope: FR-941-AC-1,
FR-941-AC-2, FR-941-AC-3.

## Test Procedure

1. Run a bare `finite-replay` item over `Placed`, `Got`, `Ship`, and over `Placed`, `Got`.
2. Run it with `send_record` removed from step 0; with one added to the `receive` step; and with a stale step 0 and step 2 missing a member.
3. Run it over a trace beginning with `Got`.

Test: `fsl-analyze/tests/it/protocol_composition.rs`.

## Expected Results

- Step 1: `proved` with 3 and with 2 replayed steps.
- Step 2: `declined` `invalid-input` with `missing-member` at `/prefix/0/send_record`; `unknown-member` at the receive step; the step-2 shape defect.
- Step 3: `declined` `invalid-input`, `invalid-value`, position 0.
