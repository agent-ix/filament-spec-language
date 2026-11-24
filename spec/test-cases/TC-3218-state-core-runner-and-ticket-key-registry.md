---
id: TC-3218
title: "The state-core source runner accounts for every case and keys every gap"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-350
    type: verifies
---
# TC-3218: The state-core source runner accounts for every case and keys every gap

## Description

Verify the state-core runner, its ticket-key registry and the unproduced-form
list, and the `FSPEC_FULL_QUALIFICATION` behavior. Scope: FR-350-AC-21 to
FR-350-AC-25.

## Test Procedure

1. Run the pinned published state-core fixtures; confirm every case id is
   accounted for.
2. Remove one registry entry for a non-matching case; add an entry for a case
   that matches; add an entry for an unknown case id; blank a ticket key; pin
   a different failure reason for a keyed case.
3. Remove the published fixture.
4. Run with `FSPEC_FULL_QUALIFICATION` unset and set over a corpus with a case
   lacking an adapter and a nonempty registry.
5. Remove an unproduced-list entry for an operation no case emits; add an
   entry for an operation a case emits; blank a key.

## Expected Results

- Step 1: every case matched or keyed, none unaccounted.
- Step 2: each alteration fails the run, naming the case id.
- Step 3: the run fails before any case.
- Step 4: unset lists the case without failing; set fails.
- Step 5: each alteration fails the run naming the operation or form.
