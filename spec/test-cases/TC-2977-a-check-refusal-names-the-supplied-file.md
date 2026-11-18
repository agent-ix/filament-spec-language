---
id: TC-2977
title: "A check refusal names the supplied file by authority filament and its path"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-866
    type: verifies
---
# TC-2977: A check refusal names the supplied file by authority filament and its path

## Description

Verify that `check_outcome` names the checked source by authority `filament`
and the path as given, on the parse-refusal path and past it. Scope:
FR-866-AC-6.

## Test Procedure

1. Call `check_outcome` over a source at path `P` that does not parse.
2. Parse the same bytes through `parse` under `{"authority": "filament", "identity": P}` and build the check document of that refusal.
3. Call `check_outcome` over a source that parses and refuses at S3.

## Expected Results

- Step 1: a refusal whose every diagnostic locus names `{"authority": "filament", "identity": P}`, whose admitted sources hold that identity, and whose document contains neither `fsl-command` nor `check-source`.
- Step 2: the document is byte-equal to step 1's.
- Step 3: the same identity, for that file's path.

Test: `fsl-command/tests/it/check_source_identity.rs`.
