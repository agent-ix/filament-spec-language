---
id: TC-3161
title: "Value-validity emission shares nested guard frames"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-062
    type: verifies
---
# TC-3161: Value-validity emission shares nested guard frames

## Description

Verify FR-062-AC-17's bounded emission and memory use for the 1,000-level
nested-body case. The reference pre-record run measured 424,912 KiB peak RSS;
the qualified run bounds wire output at 8,388,608 bytes and peak test-process
RSS at 524,288 KiB.

## Test Procedure

1. Compile the single filtered test before taking any measurements.
2. Run only
   `emit::tests::deep_bodies::every_nested_form_emits_in_the_stratified_grammar_at_2_and_1000_levels`
   in the `fsl-package` test target; exclude compiler processes and build
   time from the memory measurement.
3. Read the emitted package before the test helper parses its bytes. Count
   the unique condition-frame rows and terminal frame references in records;
   record the wire byte length.
4. Measure the isolated test process's maximum RSS with the reference
   qualification host's process accounting. Record the host, Rust toolchain,
   filtered test name, elapsed test time, wire bytes and peak RSS.

## Expected Results

- The 1,000-level body emits at most 1,000 unique path-condition frames; each
  guarded value-validity record contains one terminal frame id and no expanded
  guard array.
- The emitted package is at most 8,388,608 bytes.
- The isolated test process peaks at or below 524,288 KiB RSS, compilation
  excluded. The baseline is 424,912 KiB, leaving a maximum 99,376 KiB increase.
- The run completes without the quadratic growth seen when each record
  repeats its full guard prefix.
