---
id: TC-2526
title: "Every per-attempt outcome and aggregate recovery fact is reached"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-893
    type: verifies
---
# TC-2526: Every per-attempt outcome and aggregate recovery fact is reached

## Description

Verify that every per-attempt outcome and aggregate recovery fact is reached. Scope: FR-893-AC-1, FR-893-AC-2.

Catches a stepper with no failure or timeout step, or one that relabels an earlier attempt.

## Test Procedure

1. Explore ADR-027 §7.1's `Pay` with `attempts 2`; explore its variant whose failure event has `failed` false.

## Expected Results

- Step 1: States hold a `failed`, a `timed-out` and a `successful` retry with `retry_of` 1, never three attempts; aggregates `successful`, `failed`, `timed-out` and `pending` are reached; the untriggered variant's registration closes with no aggregate fact.
