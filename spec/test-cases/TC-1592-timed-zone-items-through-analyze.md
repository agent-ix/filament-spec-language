---
id: TC-1592
title: "Timed zone items prove with zone certificates and refute with exact delays"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-632
    type: verifies
---
# TC-1592: Timed zone items prove with zone certificates and refute with exact delays

## Description

Verify FR-632's request item and result. Scope: FR-632-AC-1 to FR-632-AC-3.

## Test Procedure

1. Analyze `eventually[0,2]` and `eventually[0,1]` over the timer subject.
2. Repeat with `options.method` `digital-clocks`.
3. Analyze `Rpc` provisioned with two calls under `max_clocks` 1, and a request with `max_clocks` 0.

## Expected Results

- Step 1: `proved` `zone-certified` `certified`; `refuted` with exact rational delays, replayed.
- Step 2: the same verdicts on backend `en-1`, the proof with basis `exhaustive`.
- Step 3: `incomplete` `LimitReached{max_clocks}` with value 1; the `max_clocks` 0 request refuses `invalid-request`, exit 20, naming `temporal.limits.max_clocks`.
- Assertions compare the outcome document's typed members and the exit code, never message text.
