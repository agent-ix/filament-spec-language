---
id: TC-2721
title: "Interval stamp admission and the interval override"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-966
    type: verifies
---
# TC-2721: Interval stamp admission and the interval override

## Description

Verify that a malformed interval stamp, or a nominal stamp outside its interval, refuses at admission at its input-order position, and that an interval stamp overrides the trace's `Bounded` cover.

Scope: FR-966-AC-1, FR-966-AC-2.

## Test Procedure

1. Run `Seen` over a trace whose second observation carries each interval of FR-966-AC-1.
2. Run `Ack` under `Bounded { uncertainty: 5 }` with the intervals `[0, 0]` and `[9, 10]`, then without them.

Test: `tests/it/temporal_uncertain_late.rs` `a_malformed_or_foreign_interval_refuses_at_admission`, `an_interval_stamp_overrides_the_bounded_cover`.

## Expected Results

- Step 1: the five refused intervals report stage `admit`, `invalid_runtime_input`/`invalid-value`, member `interval`, position 1; the two admitted ones evaluate.
- Step 2: `holds`, then `indeterminate` with `straddles-bound`.
