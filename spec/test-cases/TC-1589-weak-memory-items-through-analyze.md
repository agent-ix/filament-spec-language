---
id: TC-1589
title: "Weak-memory and race-freedom items decide under the selected model"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-629
    type: verifies
---
# TC-1589: Weak-memory and race-freedom items decide under the selected model

## Description

Verify FR-629's memory selection and results. Scope: FR-629-AC-1 to FR-629-AC-5.

## Test Procedure

1. Analyze the store-buffering litmus claim under `sc` and `tso`.
2. Analyze message passing under `ra` with release/acquire and with relaxed accesses, and compare with the RC11 oracle.
3. Analyze race freedom over two unsynchronised non-atomic writes.
4. Analyze a liveness item over a parallel selected `ra`.
5. Analyze with a `memory` selection naming no weak parallel.

## Expected Results

- Step 1: `proved`; `refuted` with both stores buffered, reproduced.
- Step 2: `proved`; `refuted`; both agree with the oracle.
- Step 3: `refuted` naming both accesses.
- Step 4: a terminal record, with a catalog-coded diagnostic when not proved or refuted.
- Step 5: `invalid-request`, exit 20.
- Assertions compare the outcome document's typed members and the exit code, never message text.
