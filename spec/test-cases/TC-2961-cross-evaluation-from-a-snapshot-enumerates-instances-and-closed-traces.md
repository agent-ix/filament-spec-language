---
id: TC-2961
title: "Cross-evaluation from a snapshot enumerates instances and closed traces"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: verifies
---
# TC-2961: Cross-evaluation from a snapshot enumerates instances and closed traces

## Description

Verify that `cross_evaluate.initial` takes its universes from the snapshot's
complete populations, evaluates every `over` instance, reads each behaviour
of 1 to `max_steps` steps as its own closed trace, and refuses `over` beside
`initial`. Scope: FR-931-AC-6, FR-931-AC-7.

## Test Procedure

1. Cross-evaluate `Low` from a two-object snapshot with `max_steps` 2; count
   the traces against the instances and the 1- and 2-step paths.
2. Cross-evaluate with `over` beside `initial`, with an incomplete
   population, and with an empty population.

Test: `tests/it/request_surface_s1_16.rs`.

## Expected Results

- Step 1: `traces` equals instances times paths, `agree` true, exit 0.
- Step 2: `invalid-request` naming `tl_lowering.cross_evaluate.over`;
  `incomplete_population`; `traces` 0 and `positions` 0, exit 0.
