---
id: TC-1668
title: "Monitor command streams events"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: verifies
---
# TC-1668: Monitor command streams events

## Description

Verify FR-644-AC-1, AC-2, AC-5 and AC-20 through AC-25: `command::monitor` hands one event per status change, each carrying `settled`, and a summary agreeing with the batch run; a malformed record is reported without disturbing the stream; and public event-position, fixed-sample and timestamped requests settle to the stated success or violation result.

## Test Procedure

Call `command::monitor` with a `native-monitor/1` request over TC-1666's clauses and five snapshot records ending `{"end": true}`; repeat with a record carrying an unknown member inserted after record 1. Through `command::run`, run FR-644-AC-5's `records` request over levels 0, 1, 2, 1 and compare each clause's latest event after each record with the native-run/1 clause run over the samples so far and over the samples up to that event's record.

Run `public_monitor_streams_settle_the_three_clock_families` with each
request/records pair in FR-644-AC-20 through AC-25: success and late controls
for event-position, fixed-sample and timestamped clocks. Preserve each
request's named temporal clause, sample levels, timestamp or watermark, closed
progress record and final `{"end": true}` record.

## Expected Results

- One event per change at the record that caused it; the summary equals the closed batch run; exit 10.
- The bad record yields one `refusal` event naming it; the other events and the summary are unchanged.
- AC-5: each latest event's category equals the run over the samples so far; its disposition equals the run up to its record. `Never` settles at record 2 with `settled` `true` and a three-sample counterexample; `Reaches` is `pending` with `settled` `false` at records 0 and 1 and settles `success` at record 2, its witness, with `settled` `true`. The summary equals the latest events, `settled` included, and its `settlement` is each clause's settling event's, `{"step": "3", "trigger": "decided"}` with its basis; exit 10.
- AC-20, AC-22 and AC-24: the summary clause category is `success`, exit 0, and at least one sink event has category `success`.
- AC-21, AC-23 and AC-25: the summary clause category is `violation`, exit 10, and at least one sink event has category `violation`.
