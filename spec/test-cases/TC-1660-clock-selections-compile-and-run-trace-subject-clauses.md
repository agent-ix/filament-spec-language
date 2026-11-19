---
id: TC-1660
title: "Clock selections compile and run trace-subject clauses"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
---
# TC-1660: Clock selections compile and run trace-subject clauses

## Description

Verify FR-640-AC-1, AC-2 and AC-5: a request's `clocks` member configures event-position, fixed-sample and timestamped-event clocks by clock name, the package rows carry them, the run evaluates each clause, and the selection enters the package identity.

## Test Procedure

1. Write a unit with `record Reading { level: Int[0, 9]; }` and three `on origin` clauses over `r: Reading`: `E` under event-position with clock `"events"`, `S` under fixed-sample with clock `"sample"`, `T` under timestamped-event with clock `"plant-ms"`.
2. Compile it through `command::compile` with `clocks` `{"events": {"kind": "event_position", "sequence_authority": "events"}, "sample": {"kind": "fixed_sample", "epoch": {"kind": "integer", "decimal": "0"}, "period": {"kind": "integer", "decimal": "1"}, "unit": "s"}, "plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}}`.
3. Run each clause through `command::run` with the same `clocks` over a sample trace of the kind its profile reads (FR-641).
4. Compile twice more: once with another `sequence_authority`, once with the members reordered.

## Expected Results

- Step 2 exits 0 and the package's `temporal_bindings` rows name `events`, `sample` and `plant-ms` with the configured `clock` values.
- Step 3 reports dispositions from stage `evaluate`, never a compile refusal.
- Step 4: another authority changes the package identity; reordering gives equal bytes.
