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

Verify FR-644-AC-1 and AC-2: `command::monitor` hands one event per status change and a summary equal to the batch run, and a malformed record is reported without disturbing the stream.

## Test Procedure

Call `command::monitor` with a `native-monitor/1` request over TC-1666's clauses and five snapshot records ending `{"end": "closed"}`; repeat with a record carrying an unknown member inserted after record 1.

## Expected Results

- One event per change at the record that caused it; the summary equals the closed batch run; exit 10.
- The bad record yields one `refusal` event naming it; the other events and the summary are unchanged.
