---
id: SR-202611210022442313144108429
title: "Code review of filament-spec-language PR 3260 (QSL-2138 missing aggregate valuation; FR-820 carried timestamped window aggregates)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; git diff origin/main...878a06a2d0b795b7669941248401bd0db1f88e94 (fsl-eval/src/temporal/observed.rs, fsl-eval/src/temporal/observed_aggregate.rs, fsl-replay/src/spine/clause/temporal.rs, fsl-analyze/tests/it/monitor_incremental.rs, tests/it/clock_reach.rs)"
review_set: subset
evaluated_revision: "878a06a2d0b795b7669941248401bd0db1f88e94"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-194
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-820
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: reviews
---

## Summary

Ticket: QSL-2138. One short code-review pass with the rust-review lane, diff
only, against FR-194 (AC-14..17 and "Evaluation"), FR-501 "Verdicts" and
FR-820 "Summarized positions" at origin/main. No build or test run (brief).

## Verdict

Item 1 is right. `valuation_with_window_visits` checks the absent
(position, atom) set before it reads the window, and every other path
(`valuation` for a silent position or without windows) checks it as well, so
a missing aggregate atom always reads `Valuation::Missing`. It cannot be
skipped into a definite verdict. The run_temporal guard now admits a
`missing` entry naming an `Aggregate` node instead of refusing it
`invalid-value`. FR-194-AC-17 is about a window member whose sample has no
field valuation. That is a different input from a (position, atom) `missing`
entry, and this diff does not claim it. A `missing` entry for a member
position names that position's own aggregate atom, not the member's field, so
the anchor still reading the member's present sample is correct.

Item 2 matches one-shot. For an aggregate clause, `read_letter` returns None,
so every sample is `TapeEntry::Held` and `Held::get` never meets a `Read`
entry. Restore maps null letters back to `Held`. The tape order is
re-canonicalised by `reorder_tail`. So carried equals one-shot at every
position, before and after a restore. The cost is FND-001. With the
`tape.is_none()` guard removed, an open timestamped carried state reads
aggregates through the same `WindowState` engine and established-window
logic as the one-shot timestamped route, so it adds no new false verdict.
The new test exercises only completeness equal to the watermark (FND-003).
`a_window_aggregate_over_a_progressed_sample_trace_refuses_and_never_faults`
is at tests/it/run_wire.rs:1361. It uses the `events` clock (event-position
profile) over a `progressed` trace, which is neither `Finite` nor
`Timestamped`, so the guard still refuses `unsupported-window-aggregate` with
the unchanged QSL-2138 message. It still passes.

Item 3: windows are set only under `TimestampedEventFiniteWindow`, so the
event-position carried route keeps `windows: None` and its earlier behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A carried timestamped state for an aggregate clause holds every sample by value forever: `read_letter` returns None for any clause with an `Aggregate` node, and `Reading::Observed` pushes `Held` with no pruning, so `retained()` grows with the trace. This breaks FR-820 "Summarized positions": hold a position only while an open activation's aggregate window can still read it | fsl-replay/src/spine/clause/clause_state.rs:919-926; fsl-eval/src/temporal/tape.rs:278 |
| FND-002 | low | `an_absent_aggregate_valuation_is_a_missing_valuation` loops over atoms 0..3 and passes if any one gives exit 11. It never pins which node is the aggregate, never checks that the other atoms are typed refusals rather than faults, and never asserts the reported position 19 | tests/it/clock_reach.rs:302-322 |
| FND-003 | low | The carried == batch test only uses `progress(stamp)`, which is complete through the watermark. The newly opened carried route under incomplete completeness (FR-194-AC-16, carried clause) is not exercised | fsl-analyze/tests/it/monitor_incremental.rs:3529-3601 |

