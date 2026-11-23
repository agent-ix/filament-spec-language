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

1. Serialize the outcome document of FR-1047-AC-1's refusal.
2. Serialize the document of FR-1047-AC-5's refusal of a `native-analyze/1` document by `run_with_providers`.
3. Serialize a document whose diagnostics carry neither member.

## Expected Results

- Step 1: `details` is exactly the `missing` object of FR-286-AC-38.
- Step 2: `details` is exactly `{"accepted":[...],"path":"/format"}`.
- Step 3: bytes equal those written before the change.

Test: `fsl-command/tests/it` (outcome golden documents).
