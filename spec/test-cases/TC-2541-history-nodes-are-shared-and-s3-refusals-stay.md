---
id: TC-2541
title: "A shared history-read node, no field on the concrete type, and S3 refusals unchanged"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-901
    type: verifies
---
# TC-2541: A shared history-read node, no field on the concrete type, and S3 refusals unchanged

## Description

Verify that a shared history-read node, no field on the concrete type, and S3 refusals unchanged. Scope: FR-901-AC-3, FR-901-AC-4.

Catches a history written as a concrete field.

## Test Procedure

1. Compile two refinements declaring the same history; compile a refinement whose history update is ill-typed; scan the corpus for `not-yet-implemented`.

## Expected Results

- Step 1: One history-read node, and no `last` field on the concrete declaration; the ill-typed update refuses at S3; no refusal names a row form `not-yet-implemented`.
