---
id: TC-1587
title: "Liveness items decide under the clause's own fairness"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-627
    type: verifies
---
# TC-1587: Liveness items decide under the clause's own fairness

## Description

Verify FR-627's fairness behaviour and the supplied-trace limit. Scope: FR-627-AC-1 to FR-627-AC-4.

## Test Procedure

1. Analyze `eventually holds(c.value = 3)` with and without `fair weak M::Counter::inc;`.
2. Analyze FR-129's mutex fixture under weak each and strong each fairness in one request.
3. Read the replay of the refuted lasso of step 1.
4. Analyze a `finite-replay` item of a clause carrying `fair strong M::Counter::inc` over a supplied trace.

## Expected Results

- Step 1: `proved` `certified`; `refuted` with a lasso stuttering below 3.
- Step 2: `refuted` then `proved`.
- Step 3: reproduced, with each constraint recorded fair on the cycle.
- Step 4: `unsupported` `missing-fairness-premise` naming the constraint, exit 21.
- Assertions compare the outcome document's typed members and the exit code, never message text.
