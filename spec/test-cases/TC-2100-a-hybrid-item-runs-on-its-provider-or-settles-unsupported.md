---
id: TC-2100
title: "A hybrid item runs on its negotiated provider or settles unsupported"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-750
    type: verifies
---
# TC-2100: A hybrid item runs on its negotiated provider or settles unsupported

## Description

Verify the hybrid provider interface, dispatch through `analyze` and `provider::run`, process-provider composition and the no-provider result. Scope: FR-750-AC-1 to FR-750-AC-5. Tests live in `fsl-analyze/tests/it/hybrid_provider.rs`, tagged `#[trace("TC-2100", "FR-750-AC-n")]`.

## Test Procedure

1. Register an in-process fixture provider with `Engine::hybrid` and analyze `SlowEnough` over `Braking`.
2. Call `Engine::hybrid` with a descriptor whose `dynamics` lacks `hybrid`.
3. Analyze the hybrid item with an empty candidate set, through `analyze` and `provider::run`.
4. Analyze it routed to `hybrid-fixture` with no engine serving it.
5. Run a clause request with a process fixture provider returning a reproducing and a non-reproducing point trace.

## Expected Results

- Step 1: one provider call; `proved`, `bounded-solver`, `uncertified`, naming the fixture.
- Step 2: a typed error naming the backend; no engine.
- Step 3: `unsupported`, `HybridProviderAbsent`, `unsupported-requested-capability` with the warning naming `temporal-satisfaction` and `hybrid-dynamics`.
- Step 4: `unsupported`, `tool-unavailable` naming `hybrid-fixture`.
- Step 5: `refuted` with one `HybridCounterexample`; `inconclusive`, `replay-parity`.
