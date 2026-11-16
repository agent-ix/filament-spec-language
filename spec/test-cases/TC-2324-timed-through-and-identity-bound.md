---
id: TC-2324
title: "A timed session settles on through, and retained identities bound what a clause cites"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: verifies
---
# TC-2324: A timed session settles on through, and retained identities bound what a clause cites

## Description

Verify the `Timed` kind's `through` assertion and timed settling, and the
`retained_identities` limit. Scope: FR-643-AC-10, FR-643-AC-11, FR-644-AC-6.

## Test Procedure

1. Open a `Timed` session of `Ack` with `req` at 0 and `ack` at 250; assert `through` 250. Repeat with `ack` at 150 and `through` 300, then finish.
2. Assert `through` 100 after a stamp 150, `through` 250 after `through` 300, and `through` on a `Finite` session.
3. Open a `Finite` session with `retained_identities` 3 over levels 0, 0, 2, 0 for `Never` and `Below`.
4. Through `command::monitor`, stream FR-643-AC-10's records with `{"through": 250}` and then `{"end": true}`; stream a record `{"end": "closed"}`.

## Expected Results

- Step 1: `Pending` until `through` 250, then `violation` at position 0; with `ack` at 150, `Pending` through `through` 300 and `success` only from `finish`.
- Step 2: `timed-through-before-last-stamp`, `timed-stamps-decrease` and `invalid-request` naming `through`, each leaving the session unchanged.
- Step 3: `Never` settles at step 3; `Below` reports `incomplete` naming `monitor.retained_identities` at step 4; `Never` keeps its report.
- Step 4: the `through` record settles `Ack` with an event whose `settled` is `true`; `{"end": "closed"}` gives a `refusal` event naming `end`.

Test: `fsl-analyze/tests/it/monitor_incremental.rs` and `tests/it/run_wire.rs`.
