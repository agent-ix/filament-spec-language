---
id: TC-2522
title: "Replicated roles pin their session instance and compensation regions project"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-890
    type: verifies
---
# TC-2522: Replicated roles pin their session instance and compensation regions project

## Description

Verify that replicated roles pin their session instance and compensation regions project. Scope: FR-890-AC-5, FR-890-AC-6.

Catches a projector that lets any instance act, or that treats a compensation as visible to a role no message reached.

## Test Procedure

1. Project the `Pool` protocol whose thread first receives `Job`, then its variant whose thread first sends; project the `Refund` compensation onto `w` with and without a message telling it of the activation.

## Expected Results

- Step 1: The class graph starts at `spawn` and pins `Done` to the receiving instance; the send-first variant is refuted, `decision` `acting-instance`; `w`'s graph holds the compensation region, and the untold variant is refuted, `decision` `activation`, owner `Seller`; `check` and `fence` map to `global-only`.
