---
id: TC-2982
title: "The observation size limit has one name and one result shape at every entry point"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-491
    type: verifies
---
# TC-2982: The observation size limit has one name and one result shape at every entry point

## Description

Verify that the observation document byte ceiling, set by the clause-run
request member or by the run command's caller limits, settles the clause
item with one `stage_limit_exceeded` record naming `observation.input_bytes`,
and that the replay facade reports the same record. Scope: FR-312-AC-9,
FR-312-AC-10, FR-491-AC-8.

## Test Procedure

1. Run FR-108's healthy-parent clause request whose snapshot is `n` bytes
   with `clause.observation_limits.document_bytes` `n - 1`, then `n`.
2. Run the same request without `observation_limits`, with
   `RequestOverrides::limits` naming `observation.input_bytes` `n - 1`.
3. Run step 1's request with that override as well.
4. Evaluate a snapshot one byte over the ceiling through the replay facade.

## Expected Results

- Step 1: exit 22; one clause item `incomplete` with cause
  `stage_limit_exceeded`, `kind` `input bytes`, `bound` `n - 1`, `actual`
  `n`, `setting` `observation.input_bytes`; no `resource_exhausted`
  diagnostic and no refusal envelope; the hint names
  `--limit observation.input_bytes=` and never `observation.document_bytes`.
  At `n` the clause runs, exit 0.
- Step 2: an outcome document byte-equal to step 1's refused run.
- Step 3: `invalid-request` at stage `request` naming `observation_limits`,
  exit 20, no observation file read.
- Step 4: `Incomplete(Observation)` carrying the same `stage_limit_exceeded`
  record.
