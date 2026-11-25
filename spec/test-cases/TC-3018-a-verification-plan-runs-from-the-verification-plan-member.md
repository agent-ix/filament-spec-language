---
id: TC-3018
title: "A verification plan runs from the verification plan member"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1038
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-752
    type: verifies
---
# TC-3018: A verification plan runs from the verification plan member

## Description

Verify that the run request's `verification_plan` member selects one plan,
supplies `initial` and `seed`, and writes the plan run outcome, and that bad
selections and members refuse at their pointers. Scope: FR-1038-AC-1 and
FR-1038-AC-2, FR-120-AC-37 and FR-752-AC-12.

## Test Procedure

1. Execute `Release` with `initial`, `seed` 7 and `max_trace_length` 8; then emit it without
   executing.
2. Send `{"max_trace_length": 8}` with one plan and with two; name `Nope` and `Never`; send
   `seed` `"7"`. For the one-plan `Release`, use a state subject with
   population `cells`, universe `{c1, c2}`, and `value: Int[0, 1]` on each
   `Cell`, with no initial snapshot. Inspect the run's FR-752 record and its
   generated cases and reachability domain; repeat the same request.

## Expected Results

- Step 1: `safe` refuted, `tested` tested, `timely` blocked by `safe`,
  `failed_steps` `["safe"]`, exit 22; then the plan document, exit 0.
- Step 2: `Release` runs; `missing-member`, then `missing-name` twice, at
  `/request/verification_plan/plan`; `wrong-value-kind` at
  `/request/verification_plan/seed`; each refusal exits 20. The no-initial
  state-subject run settles `tested`, has `inputs.initial` `null`, and its
  cases and reachability exploration use the four-state product domain
  `{((c1,0),(c2,0)), ((c1,0),(c2,1)), ((c1,1),(c2,0)), ((c1,1),(c2,1))}`.
  The first seeded initial state is the FR-190 uniform draw at `step` 0,
  `choice` 0; the repeat has a byte-equal record. No key outside `{c1, c2}`
  or other population is present.
