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

Verify the reference provider's proofs, refutations, class refusal and coexistence with the provider path. Scope: FR-1019-AC-1 to AC-5.

## Test Procedure

1. Analyze `SlowEnough` with the reference provider.
2. Analyze `TooSlow`.
3. Analyze a model with flow `v' = sin(v)`.
4. Analyze under `Engines::fsl()` alone, then with a fixture provider named.
5. Register `fsl-model-check` with the reference provider, then add a fixture hybrid provider and name each or neither.

## Expected Results

- Step 1: `proved`, `flowpipe-outward-rounded`, `certified`.
- Step 2: `refuted`, the trace replays.
- Step 3: `unsupported`, located at the flow.
- Step 4: `HybridProviderAbsent`; then routed to the fixture.
- Step 5: one candidate each, no `ambiguous-backend`; with two hybrid providers, a named one routes and naming neither settles `ambiguous-backend`.
