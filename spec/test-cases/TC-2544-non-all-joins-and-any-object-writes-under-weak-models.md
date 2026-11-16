---
id: TC-2544
title: "Non-all joins and any-object writes explore under weak models"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-902
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-219
    type: verifies
---

# TC-2544: Non-all joins and any-object writes explore under weak models

## Description

Verify that non-all joins and any-object writes explore under weak models. Scope: FR-902-AC-5, FR-902-AC-6.

Catches the S3 any-object refusal and the join-policy refusal.

## Test Procedure

1. Explore FR-219-AC-4's `join any … outstanding cancel` under `tso`; check and explore an any-object field write in a `memory tso` parallel; compile and explore the store-buffering litmus with a multi-access `WriteX` under `tso`.

## Expected Results

- Step 1: The cancelled branch's buffered store flushes after the join; the any-object write checks with no refusal and explores; the litmus compiles, explores and settles.
