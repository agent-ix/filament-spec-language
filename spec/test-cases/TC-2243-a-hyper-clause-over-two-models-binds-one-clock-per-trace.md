---
id: TC-2243
title: "A hyper clause over two models binds one clock per trace"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-173
    type: verifies
---
# TC-2243: A hyper clause over two models binds one clock per trace

## Description

Verify FR-640's per-trace clock bindings for hyper clauses. Scope: FR-640-AC-12 and FR-173-AC-6.

## Test Procedure

1. Compile, with models `S` (`example/device-saving`) and `N` (`example/device-normal`) and no `clocks` member, the event-position hyper clause `Saves` over `forall trace s of S forall trace n of N`.
2. Compile the hyper clause `Same` over two traces of one model.
3. In `fsl-analyze`'s tests, analyze `Saves` over the two models through the `native-analyze/1` entry, which runs the layer-A hyper model checker, and replay a refuted item's counterexample through `fsl_replay::replay`. The step runs there because the hyper checker is a layer-A engine that `fsl-replay` and `fsl-semantics` cannot reach (ADR-011 §6.1).
4. Read the step 1 package with the `n` row removed.

## Expected Results

- Step 1: exit 0; two rows keyed by the `hyperproperty` node and its `claim` occurrence, none by the body wrapper, `trace` `s` then `n`, each `event_position` with its own model's `ConfigVersion` declaration identity text as `sequence_authority`.
- Step 2: two rows with equal configurations.
- Step 3: `proved` or `refuted`, never `not-yet-implemented`, `FormNotDecided` or V-8; a refuted counterexample holds one trace for `s` and one for `n`, neither with a loop, each ending at a terminal state or covering positions 0 to the body's horizon `h`, and replays as closed traces under the two rows' configurations.
- Step 4: the package reader refuses it at the `hyperproperty` node's body before any recompile.
