---
id: TC-2823
title: "A lowered population only evaluates over one, none and two objects"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-998
    type: verifies
---
# TC-2823: A lowered population only evaluates over one, none and two objects

## Description

Verify that a lowered population only evaluates over one, none and two objects. Scope: FR-998-AC-2.

Catches an only that picks an object when none or several match.

## Test Procedure

1. Evaluate O over snapshots with one, no and two counters at v = 2.

## Expected Results

- Step 1: holds for one; undefined only-not-one for none and for two.
