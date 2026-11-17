---
id: TC-2241
title: "A half-open watermark of zero refuses at the request stage"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: verifies
---
# TC-2241: A half-open watermark of zero refuses at the request stage

## Description

Verify FR-641's reading of a half-open watermark. Scope: FR-641-AC-6.

## Test Procedure

1. Through `command::run`, run the timestamped-event clause `T` over a `timestamped` trace of three samples stamped 0, 2 and 5 whose progress watermark is `{"half_open": 0}`.
2. Run the same request with the watermark `{"half_open": 6}`, and again with the inclusive watermark 5.

## Expected Results

- Step 1: `invalid-request` at stage `request`, exit 20, the refusal naming `trace.progress.watermark.half_open`, and no result document.
- Step 2: the two runs report equal dispositions and exit codes.
- Assertions compare the refusal envelope's typed members and the exit code, never message text beyond the member name.
