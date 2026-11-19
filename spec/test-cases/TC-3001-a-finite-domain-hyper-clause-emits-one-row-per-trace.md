---
id: TC-3001
title: "A finite-domain hyper clause emits one clock row per trace"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-898
    type: verifies
  - target: ix://agent-ix/filament-specification/FR-322
    type: verifies
---
# TC-3001: A finite-domain hyper clause emits one clock row per trace

## Description

Verify that a hyper clause over a declared finite trace domain reads its clock role from the domain binder `t`, takes its profile from the lock's `temporal_profile` selection, and emits one row per trace at its `hyperproperty` node. Scope: FR-898-AC-7.

## Test Procedure

1. Compile `Det` of FR-898-AC-1 under event-position with `clocks` `{"t": {"kind": "event_position", "sequence_authority": "obs"}}`; read `temporal_bindings`.
2. Compile it with no `clocks` member, with an extra unused role, and with a `fixed_sample` configuration for `t`.

Tag the tests `#[trace("TC-3001", "FR-898-AC-7", "FSpec-TC-2028", "FSpec-FR-322-AC-464")]`.

## Expected Results

- Step 1: exit 0; two rows at the `hyperproperty` node and its `claim` occurrence, `trace` `a` then `b`, each with `clock_name` `t` and the bound configuration.
- Step 2: each compile refuses with no package.
