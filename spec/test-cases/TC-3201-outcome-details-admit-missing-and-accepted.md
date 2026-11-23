---
id: TC-3201
title: "The outcome document admits the missing and accepted details members"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3201: The outcome document admits the missing and accepted details members

## Description

Verify that the request-refusal documents of FR-1047 serialize their details in the closed vocabulary. Scope: FR-286-AC-38.

## Test Procedure

1. Serialize the outcome document of FR-1047-AC-1's refusal and validate it against `OUTCOME_SCHEMA_1`.
2. Do the same for FR-1047-AC-5's refusal of a `native-analyze/1` document by `run_with_providers`.

## Expected Results

- Step 1: `details` is exactly `{"missing":["/request/models",["/request/program","/request/package"],["/request/call","/request/clause","/request/items"]]}`; the document validates.
- Step 2: `details` is exactly `{"accepted":["native-run/1","native-monitor/1","native-replay/1"],"path":"/format"}`; the document validates.

Test: `fsl-command/tests/it` (outcome documents) and `fsl-replay` schema tests.
