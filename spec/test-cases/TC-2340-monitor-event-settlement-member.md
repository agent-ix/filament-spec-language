---
id: TC-2340
title: "A monitor event and summary carry the settlement beside the batch report"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: verifies
---
# TC-2340: A monitor event and summary carry the settlement beside the batch report

## Description

Verify that a settled clause's event and summary entry carry the batch
disposition `B(s)` and, separately, the `settlement` that records the reading
that decided it, and that a `finite` summary has no ledger. Scope:
FR-644-AC-7.

## Test Procedure

1. Through `command::run`, run a `native-monitor/1` request whose `records` file holds `finite` samples 0, 1, 2 for `E` (`eventually[0,2] holds(r.level = 2)`), then `{"end": true}`.
2. Run the native-run/1 clause run of `E` over samples 0 to 2.

## Expected Results

- Step 1: records 0 and 1 give events with `settled` `false` and no `settlement`; record 2 gives an event with `settled` `true`, a `disposition` equal to step 2's (category `success`, basis `closed-scope`, no `finite_window`), and `settlement` `{"step": "3", "trigger": "decided", "basis": "decisive-witness", "horizon": "2", "position": "2"}`; the summary entry carries the same `settlement`; the summary has no `ledger`.

Test: `tests/it/run_wire.rs`.
