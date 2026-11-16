---
id: TC-2324
title: "A timed session settles on a covering stamp or through, and retained identities bound what a clause cites"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: verifies
---
# TC-2324: A timed session settles on a covering stamp or through, and retained identities bound what a clause cites

## Description

Verify timed settling (a window completed by a later stamp or by `through`,
success only at `finish`), the `through` assertion and its refusals, and the
`retained_identities` limit. Scope: FR-643-AC-10, FR-643-AC-11, FR-644-AC-6.

## Test Procedure

1. Open a `Timed` session of FR-642's `Ack` and feed `req` at 0, `ack` at 250.
2. Feed `req` at 0 alone, then assert `through` 250.
3. Feed `req` at 0, `ack` at 150, assert `through` 300, and finish.
4. Run `Bound` (`always[0 ms, 300 ms] (holds(r.req) implies eventually[0 ms, 200 ms] holds(r.ack))`) over `req` at 0, `ack` at 150, `through` 500, and finish.
5. Assert `through` 100 after a stamp 150; `through` 250 after `through` 300; after `through` 300 feed observations stamped 260 and 300; assert `through` on a `Finite` session.
6. Open a `Finite` session with `retained_identities` 3 over levels 0, 0, 2, 0, 0 for `Never` and `Below`.
7. Through `command::monitor`, stream a `timed` `req` at 0, `{"through": 250}` and `{"end": true}`; stream a `finite` request with a `{"through": 5}` record and one with `{"end": "closed"}`.

## Expected Results

- Step 1: `Ack` settles `violation` at position 0 at the `ack` step.
- Step 2: `Pending` until `through` 250, then `violation` at position 0 with settlement `{step "2", trigger "decided", basis "closed-scope", horizon "250", position "0", through "250"}`.
- Step 3: `Pending` throughout; `finish` returns `pending`.
- Step 4: `Pending` until `finish`, which returns `success`.
- Step 5: `timed-through-before-last-stamp`; `timed-stamps-decrease` for the lower `through` and for both observations; `invalid-request` naming `through`; each leaves the session unchanged.
- Step 6: `Never` settles at step 3; `Below` settles `incomplete` naming `monitor.retained_identities` at step 4 with its state dropped and keeps that report at step 5; `Never` keeps its report.
- Step 7: the `through` record gives an `Ack` event with `settled` `true`, category `violation`; the `finite` `through` record and `{"end": "closed"}` each give a `refusal` event naming `through` and `end`.

Test: `fsl-analyze/tests/it/monitor_incremental.rs` and `tests/it/run_wire.rs`.
