---
id: TC-2720
title: "A source-declared time unit is the clock unit of a timestamped clause"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-965
    type: verifies
---
# TC-2720: A source-declared time unit is the clock unit of a timestamped clause

## Description

Verify that a timestamped-event clause's clock unit may be a unit its own source declares, that its `duration` window bounds are typed by that unit's node, and that a `timestamp_unit` naming no linear `Time` unit refuses the emission.

Scope: FR-965-AC-1 to FR-965-AC-3.

## Test Procedure

1. Emit FR-965-AC-1's clause under `timestamp_unit` `ms`, then `s`, and read each package back.
2. Emit it under `us`, and with a source `Length` unit `m` under `m`.
3. Emit the clause in a unit that also selects `example/config-version`, under `ms` and under `ix://example/config-version/unit/ms`.

Test: `fsl-package/src/emit/tests/temporal_aggregate.rs` `a_source_declared_time_unit_is_the_clock_unit`, `a_timestamp_unit_naming_no_time_unit_refuses`.

## Expected Results

- Step 1: bounds `0`/`300000` of the source `ms` node and `0`/`300` of the source `s` node; no `["scalar_type", "unit", "ms"]` node; both packages admitted.
- Step 2: `InvalidTemporalClockValue` naming `timestamp_unit`, no package.
- Step 3: the source node under `ms`, the model-owned node under the FCD identity, one unit node each.
