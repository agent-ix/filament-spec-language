---
id: TC-2822
title: "A population only lowers to collection.only"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-998
    type: verifies
---
# TC-2822: A population only lowers to collection.only

## Description

Verify that a population only lowers to collection.only. Scope: FR-998-AC-1.

Catches a lowering that refuses the checked form or lowers it through map.

## Test Procedure

1. Check and emit FR-391-AC-10's Counter unit with the invariant O, then with O written without where.

## Expected Results

- Step 1: one filament.op.collection.only application over the filter of the all_instances value, member and result_type the type of o.v; without where, over the all_instances value; no refusal.
