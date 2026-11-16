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

1. Over the staying Counter (`setTo(0)` a self-loop at `value = 0`), analyze `eventually holds(c.value = 3)` with and without `fair weak M::Counter::inc;`; over the inc-only Counter, analyze it without fairness.
2. Analyze FR-129's mutex fixture under weak each and strong each fairness in one request.
3. Read the replay and the `loop` member of the refuted lasso of step 1.
4. Analyze a `finite-replay` item naming a temporal clause carrying `fair strong M::Counter::inc`, with a supplied trace.

## Expected Results

- Step 1: `proved` `certified`; `refuted` with a lasso whose `loop` repeats `setTo(0)` at `value = 0`; over the inc-only Counter `proved`.
- Step 2: `refuted` then `proved`.
- Step 3: reproduced, the cycle under `loop`, with each constraint recorded fair on the cycle.
- Step 4: `invalid-request` `inapplicable-kind` with `detail.family` `temporal-trace`, no engine run, exit 20.
- Assertions compare the outcome document's typed members and the exit code, never message text.
