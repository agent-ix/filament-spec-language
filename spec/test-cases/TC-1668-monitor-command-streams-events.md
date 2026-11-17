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

Verify FR-644-AC-1, AC-2 and AC-5: `command::monitor` hands one event per status change, each carrying `settled`, and a summary agreeing with the batch run, and a malformed record is reported without disturbing the stream.

## Test Procedure

Call `command::monitor` with a `native-monitor/1` request over TC-1666's clauses and five snapshot records ending `{"end": true}`; repeat with a record carrying an unknown member inserted after record 1. Through `command::run`, run FR-644-AC-5's `records` request over levels 0, 1, 2, 1 and compare each clause's latest event after each record with the native-run/1 clause run over the samples so far and over the samples up to that event's record.

## Expected Results

- One event per change at the record that caused it; the summary equals the closed batch run; exit 10.
- The bad record yields one `refusal` event naming it; the other events and the summary are unchanged.
- AC-5: each latest event's category equals the run over the samples so far; its disposition equals the run up to its record. `Never` settles at record 2 with `settled` `true` and a three-sample counterexample; `Reaches` stays `pending` with `settled` `false`. The summary equals the latest events, `settled` included, and its `settlement` is `{"step": "4", "trigger": "finish"}` for `Reaches`; exit 10.
