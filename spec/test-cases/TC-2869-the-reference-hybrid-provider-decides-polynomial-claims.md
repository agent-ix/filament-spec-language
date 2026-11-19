---
id: TC-2869
title: "The reference hybrid provider decides polynomial hybrid claims"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1019
    type: verifies
---
# TC-2869: The reference hybrid provider decides polynomial hybrid claims

## Description

Verify the reference provider's proofs, refutations, class refusal, options, budgets, guard handling, placement and coexistence with the provider path. Scope: FR-1019-AC-1 to AC-12 and AC-14. Tests live in `fsl-analyze/tests/it/hybrid_reference.rs`, tagged `#[trace("TC-2869", "FR-1019-AC-n")]`.

## Test Procedure

1. Analyze `SlowEnough` with the reference provider.
2. Analyze `TooSlow` twice.
3. Analyze a model with flow `v' = -a / (1 + v)`.
   3b. Analyze `Braking` variants: an initial entry `v = 40 / 2 and a = 1` with that flow; the flows `v' = -a / (1 + v / 2)`, `v' = if v > 1 then -a else 0` and `v' = -rational(1, 2) * a`; and the guard `if a > 1 then v = 0 else v <= 0`.
4. Analyze under `Engines::fsl()` alone, then with a fixture provider named.
5. Register `fsl-model-check` with the reference provider, then add a fixture hybrid provider and name each or neither.
6. Read the reference descriptor; analyze `SlowEnough` with a step budget of 1.
7. Analyze `SlowEnough` with coarse options, a zero step and an unknown option.
8. Read `SlowEnough`'s flowpipe certificate.
9. Analyze `always holds(h.v < 30)`.
10. Analyze the one-mode `Coast` model.
11. Analyze the `Grow` model with flow `v' = v`.
12. Read the workspace manifests.

## Expected Results

- Step 1: `proved`, `flowpipe-outward-rounded`, horizon 10, jumps 1, `certified`.
- Step 2: `refuted`, the start-only trace at `v = 0, a = 1` replays; equal counterexamples.
- Step 3: `unsupported`, located at the flow.
- Step 3b: `unsupported`, `expression-form` located at the initial entry's division, then at the outer division, then at the `if`, each naming `Moving`; the rational coefficient and the Boolean `if` guard do not settle `unsupported`.
- Step 4: `HybridProviderAbsent`; then routed to the fixture.
- Step 5: one candidate each, no `ambiguous-backend`; with two hybrid providers, a named one routes and naming neither settles `ambiguous-backend`.
- Step 6: the published bounds and options; `incomplete`, `limit-reached` naming `hybrid.max_solver_steps`.
- Step 7: `proved`, `certified` with a different request; two `invalid-request` items naming the option.
- Step 8: one `Stopped` jump step at time `[0, 10]`; no `Moving` box below `v = 0`.
- Step 9: `proved`, `inductive-solver`, `certified`.
- Step 10: `refuted` with `end` at time 3.
- Step 11: `refuted` through an accepted `validated-witness`.
- Step 12: the layer-H dependency rule holds.
