---
id: TC-2508
title: "Protocol check refusals are located at their subject and name it"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-884
    type: verifies
---
# TC-2508: Protocol check refusals are located at their subject and name it

## Description

Verify the refusal region and message for reads, decisions, duplicate members and missing names. Scope: FR-884-AC-1, FR-884-AC-2, FR-884-AC-3.

## Test Procedure

1. Check a `choice` guard reading `input.ready`, then one reading the unlisted `got.ok`.
2. Check `choice Pick by R` with guards `true` and `true`; two roles `R`; a channel to the undeclared `Buyer`.
3. Read each refusal's message.

## Expected Results

- Step 1: `operator-ineligible` at `input.ready`; `wrong-observation` at `got.ok`; no region holds `choice` or the protocol name.
- Step 2: `unproved-exhaustiveness` at `choice Pick by R`; two `ambiguous-name` at `R`, in source order; `missing-name` at `Buyer`.
- Step 3: each message holds its subject's text.
