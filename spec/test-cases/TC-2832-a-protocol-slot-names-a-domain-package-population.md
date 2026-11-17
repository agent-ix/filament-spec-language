---
id: TC-2832
title: "A protocol slot that reads model state names a domain-package population"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-391
    type: verifies
---
# TC-2832: A protocol slot that reads model state names a domain-package population

## Description

Verify that protocol, registration and activation captures, `send` and event
constraints and `finish` constraints are population positions bound to the
protocol state's model state, and that a slot that bars model state still
refuses the read. Scope: FR-391-AC-11.

Catches a protocol slot that refuses `M::P` as `missing-name`, and an
evaluation that reads a snapshot instead of the protocol state's model state.

## Test Procedure

1. Check FR-391-AC-11's protocol `W` over the `Counter` unit (TC-1076's
   fixture).
2. Evaluate its capture and lookup over a protocol state whose model state
   holds counters `a` and `b`, with `c` = `a`.
3. Check `W` with a `choice` guard that reads `M::counters`.

## Expected Results

- Step 1: no refusal; each read is the `counters` population reference.
- Step 2: the capture binds 2; the lookup binds `a`'s reference.
- Step 3: `ill_typed`/`operator-ineligible` at the read.
