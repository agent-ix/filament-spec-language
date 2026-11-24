---
id: TC-720
title: "A reached limit names its kind, bound, count and setting"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: verifies
---
# TC-720: A reached limit names its kind, bound, count and setting

## Description

Verify that every implemented limit in FR-255's setting table reports
the limit, its configured value, the count reached and the setting that
raises it, that the rendered diagnostic says how to raise it, and that each
limits type maps its fields to the table.

Scope: FR-255-AC-1, FR-255-AC-2, FR-255-AC-3, FR-255-AC-21 and FR-255-AC-26.

## Test Procedure

1. For each implemented row of FR-255's setting table, build the smallest input that
   reaches that limit at a configured bound `B` (for the I2 rows, a v2
   artifact; for the observation rows, a snapshot; for the library rows, a
   catalog), and run the row's stage.
2. Check `f`, whose body is `1 + 1 + 1`, with `s3.nodes` at 4, once through a
   native-run/1 request and once through a replay request, and render each
   diagnostic.
3. For each stage's limits type, read the setting name of each field. Set
   each field through its builder method to a value distinct from its
   default.
4. For every setting-table row, set the limit through each listed entry point to the maximum value of its bound type and verify that it is accepted and preserved. Do not apply a smaller implementation ceiling.
5. Give the settings operation `explore.workers=4`, `model_check.workers=2`, `statistical.workers=8`, `explore.workers=0` and `explore.threads=4`, and `explore.workers=2` alone (FR-255-AC-21).

Tag the tests `#[trace("TC-720", "FR-255-AC-26")]`.

Tag the tests `#[trace("TC-720", "FR-255-AC-1")]`, `#[trace("TC-720", "FR-255-AC-2")]`, `#[trace("TC-720", "FR-255-AC-3")]` and `#[trace("TC-720", "FR-255-AC-21")]`.

## Expected Results

- Step 1: each run returns its stage's limit outcome with the row's kind,
  bound `B`, the count the refused charge would have reached and the row's
  setting name; its catalog record carries `kind`, `bound`, `actual` and
  `setting`.
- Step 2: the native-run/1 diagnostic is "S3 node limit 4 reached (5) at
  <locus>; raise it with `--limit s3.nodes=<n>`" and does not mention a
  request entry. The replay diagnostic is "S3 node limit 4 reached (5) at
  <locus>; raise it with `--limit s3.nodes=<n>` or the replay request's
  `stage_limits` entry `s3.nodes`". Both use FR-096's locus of the node whose
  entry failed.
- Step 3: each field maps to one name; the names are distinct; their union
  equals the table and the accounting table, and neither the supplied-value
  ceilings of FR-277 nor the monitor target's `monitor.max_event_rate` is in
  that union; each builder call changes only its own field.
- Step 4: the first three return caller limits with that worker count set and the others 1; `explore.workers=0` returns the usage refusal with cause `invalid_runtime_input`/`invalid-value`; `explore.threads=4` returns the unknown-setting refusal; `explore.workers=2` leaves `explore.states` at 10000000; no worker name is in the engine limit table.
