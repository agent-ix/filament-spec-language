---
id: TC-2991
title: "The package records the observe-only boundary"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-383
    type: verifies
---
# TC-2991: The package records the observe-only boundary

## Description

Verify that S4 emits the observe-only marks and the `recommend` control and that IR's strict reader admits them. Scope: FR-383-AC-69.

## Test Procedure

1. Emit the FR-383-AC-69 clause over the ConfigVersion model and read it back with IR's strict reader.
2. Emit the same clause without `observe` and compare the node ids and bodies.

## Expected Results

- Step 1: the reader admits the package; exactly the `Auditor` role node and the `Review` control node carry the last argument `boundary` `observe-only`; `Advise` is a `recommend` control with the arguments `role`, `operation`, `contracts`, `binder`, `related`, `constraint`.
- Step 2: the node ids are equal, no node carries `boundary`, and only the two marked nodes' bodies differ.
