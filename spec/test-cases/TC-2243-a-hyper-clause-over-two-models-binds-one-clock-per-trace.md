---
id: TC-2243
title: "A hyper clause over two models binds one clock per trace"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
---
# TC-2243: A hyper clause over two models binds one clock per trace

## Description

Verify FR-640's per-trace clock bindings for hyper clauses. Scope: FR-640-AC-12.

## Test Procedure

1. Compile, with models `S` (`example/device-saving`) and `N` (`example/device-normal`) and no `clocks` member, the event-position hyper clause `Saves` over `forall trace s of S forall trace n of N`.
2. Compile the hyper clause `Same` over two traces of one model.
3. Analyze `Saves` over the two models and replay a refuted item's counterexample.
4. Read the step 1 package with the `n` row removed.

## Expected Results

- Step 1: exit 0; two body-wrapper rows, `trace` `s` then `n`, each `event_position` with its own model's `ConfigVersion` declaration identity text as `sequence_authority`.
- Step 2: two rows with equal configurations.
- Step 3: a verdict, never `not-yet-implemented`; a refuted counterexample replays under the two rows' configurations.
- Step 4: the package reader refuses it before any recompile.
