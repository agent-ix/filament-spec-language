---
id: TC-1662
title: "Timestamped and progressed traces on the run wire"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: verifies
---
# TC-1662: Timestamped and progressed traces on the run wire

## Description

Verify FR-641-AC-1 and AC-2: request-file `timestamped` and `progressed` traces reach FR-501 and FR-504 with the outcomes of the same in-process inputs, and the result carries the ledger.

## Test Procedure

1. Run TC-1660's `T` over a `timestamped` trace of samples stamped 0, 2, 5, watermark 5, `complete` through 5, scope `closed`, `authoritative-origin` and a binding equal to the clause row; build the same input in process.
2. Run `E` and `S` over `progressed` traces matching FR-504-AC-1 and AC-2.

## Expected Results

- Step 1: result bytes equal the in-process report's document.
- Step 2: FR-504's outcomes, and each result has a `ledger` member.
