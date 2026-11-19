---
id: TC-3115
title: "Every golden monitor line validates against its published schema"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: verifies
---
# TC-3115: Every golden monitor line validates against its published schema

## Description

Verify that the monitor event and summary schemas accept every golden line
and reject mutated lines at the named instance pointer. Scope:
FR-644-AC-12.

## Test Procedure

1. Rerun each stream of `fsl-command/tests/fixtures/monitor-golden/`;
   compare each event and the summary, canonically encoded, with the file's
   lines; re-encode each line through `quire-canonical`; validate event
   lines against `MONITOR_EVENT_SCHEMA_1` and the summary line against
   `MONITOR_SUMMARY_SCHEMA_1`.
2. Mutate FR-644-AC-7's record-2 event: remove `settlement`; set
   `/settlement/trigger` to `"decide"`; add a member `extra`. Add a
   `settlement` to its record-0 event. Validate each.
3. Add a `ledger` to FR-644-AC-7's summary; remove `registry` from
   FR-644-AC-8's summary. Validate each.
4. Validate FR-644-AC-2's refusal event and FR-644-AC-6's `through`
   refusal event; remove `/disposition/member` from the AC-2 event; set
   its `clause` to a clause name. Validate each.

## Expected Results

- Step 1: every line equals the stream's, is its own re-encoding, and
  validates.
- Step 2: errors at the root pointer (keyword `required`),
  `/settlement/trigger`, the root pointer and the root pointer.
- Step 3: both fail at the root pointer.
- Step 4: both refusal events validate; the two mutations fail at
  `/disposition`.

Test: `fsl-command/tests/it/monitor_schema.rs::every_golden_monitor_line_validates_and_mutations_fail_at_their_pointer`.
