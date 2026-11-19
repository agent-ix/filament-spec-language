---
id: TC-2503
title: "Bounded message types and FIFO-key-stable keys are written, and no checked protocol is refused at S4"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-880
    type: verifies
---
# TC-2503: Bounded message types and FIFO-key-stable keys are written, and no checked protocol is refused at S4

## Description

Verify bounded `carries`, every FIFO-key-stable key, and the corpus inventory. Scope: FR-880-AC-10, FR-880-AC-11.

## Test Procedure

1. Compile channels carrying each bounded type of FR-880-AC-10, and FIFO channels keyed by each listed key type, including `m.part` of type `PartId`.
2. Compile the one-protocol-per-construct corpus of FR-218.

## Expected Results

- Step 1: each compiles with the `bounded_domain` node as `carries` or with its key; every package is admitted.
- Step 2: every checked protocol writes a package; no refusal carries `not-yet-implemented`.
