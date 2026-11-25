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
  - target: ix://agent-ix/filament-spec-language/FR-753
    type: verifies
---
# TC-3018: A verification plan runs from the verification plan member

## Description

Verify that the run request's `verification_plan` member selects one plan,
supplies `universe_snapshot`, `initial` and `seed`, and writes the plan run outcome, and that bad
selections and members refuse at their pointers. Scope: FR-1038-AC-1 and
FR-1038-AC-2, FR-120-AC-37, FR-752-AC-12 and FR-753-AC-6.

## Test Procedure

1. Execute `Release` with `initial`, `seed` 7 and `max_trace_length` 8; then emit it without
   executing.
2. Send `{"max_trace_length": 8}` with one plan and with two; name `Nope` and `Never`; send `seed` `"7"`.
3. For the one-plan `Release`, use `test/no-initial`, whose `NoInitial` model declares populations `cells` and `unbounded_cells`, both of `Cell`, and `Cell.value: Int[0, 1]`. The request has no `initial` and is `{"plan":"Release","max_trace_length":8,"universe_snapshot":"universe.json","seed":7}`. The `universe.json` bytes are:

   ```json
   {"format":"filament.state.snapshot/v1","identity":{"authority":"agent-ix","identity":"ix://test/no-initial/universe"},"observation":"current","anchor":{"kind":"initialization","name":"NoInitial"},"model":{"identity":"test/no-initial"},"populations":[{"population":"ix://test/no-initial/cells","complete":true,"objects":[{"key":"c1","type":"ix://test/no-initial/Cell","fields":{"value":{"integer":"0"}}},{"key":"c2","type":"ix://test/no-initial/Cell","fields":{"value":{"integer":"1"}}}]},{"population":"ix://test/no-initial/unbounded_cells","complete":false,"objects":[]}]}
   ```

   Inspect the FR-752 record, its generated cases and reachability domain;
   repeat the request. Repeat with `cells.complete` false, with an invalid
   `Cell.value` encoding, with numeric `universe_snapshot`, a missing file,
   and with a plan that has no state-subject `model_based_test` step.

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
  `choice` 0; the repeat has a byte-equal record. No key outside `{c1,c2}`
  or other population is present. With `cells.complete` false, the direct
  FR-752 call returns refusal `member: subject`, `code: requires_bound`,
  `cause: requires-bound`, `details.domains` containing the ordered FR-120
  root `{key: DomainKey::Population {member_type: {package:"test/no-initial",path:["model","record_value_type","ix://test/no-initial/Cell"],ordinal:0}, ordinal: 1},
  kind: Population(None)}`; the plan step is `declined` with the same
  code/cause/member/root detail, exit 20, and no case runs. Invalid field
  encoding retains FR-106's refusal code/cause at
  `/request/verification_plan/universe_snapshot`; numeric member is
  `wrong-value-kind`, missing file `io-error`, and a plan without a consuming
  step refuses `unknown-member` at that pointer. The snapshot is admitted
  before any `initial` snapshot, and initial keys cannot enlarge its universe.
