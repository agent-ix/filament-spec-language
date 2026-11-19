---
id: TC-2527
title: "Instance summaries, distinct outcome values and a refuted summary claim"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-893
    type: verifies
---
# TC-2527: Instance summaries, distinct outcome values and a refuted summary claim

## Description

Verify that instance summaries, distinct outcome values and a refuted summary claim. Scope: FR-893-AC-3, FR-893-AC-4, FR-893-AC-5.

Catches a summary computed from a label instead of the registrations' facts.

## Test Procedure

1. Explore the two-template protocol; compare the outcome values; explore a saga with no trigger and check the summary claim.

## Expected Results

- Step 1: Summaries `partially-recovered`, `recovered`, `unrecovered` and `untriggered` are reached; all values are distinct, no attempt carries `duplicate` or `pending`, and differing outcomes give different keys; the no-trigger saga reports `untriggered`, and the claim is refuted with a replaying counterexample.
