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

Verify the reference provider's proofs, refutations, class refusal, options, budgets, guard handling, placement and coexistence with the provider path. Scope: FR-1019-AC-1 to AC-12 and AC-14 to AC-16. Tests live in `fsl-analyze/tests/it/hybrid_reference.rs`, tagged `#[trace("TC-2869", "FR-1019-AC-n")]`.

## Test Procedure

1. Analyze `SlowEnough` with the reference provider.
2. Analyze `TooSlow` twice.
3. Analyze a model with flow `v' = -a / (1 + v)`.
   3b. Analyze `SlowEnoughC` over each of FR-1019's class fixtures C1 to C6 (the `BrakingC` template with its whole initial condition `I`, `Moving` flow `F` for `v` and guard `G`).
   3c. Analyze FR-1019-AC-15's model whose `Moving` flow block writes `a'` before `v'`, with its variables declared `v` then `a`, and again declared `a` then `v`.
4. Analyze under `Engines::fsl()` alone, then with a fixture provider named.
5. Register `fsl-model-check` with the reference provider, then add a fixture hybrid provider and name each or neither.
6. Read the reference descriptor; analyze `SlowEnough` with a step budget of 1.
7. Analyze `SlowEnough` with coarse options, a zero step and an unknown option.
8. Read `SlowEnough`'s flowpipe certificate.
9. Analyze `always holds(h.v < 30)`.
10. Analyze the one-mode `Coast` model.
11. Analyze the `Grow` model with flow `v' = v`.
12. Read the workspace manifests.
13. Read `SlowEnough`'s jump step and check its certificate rule by rule; analyze `Drop` under `clock "model-steps"` and `Fork` under `model-time`.

## Expected Results

- Step 1: `proved`, `flowpipe-outward-rounded`, horizon 10, jumps 1, `certified`.
- Step 2: `refuted`, the start-only trace at `v = 0, a = 1` replays; equal counterexamples.
- Step 3: `unsupported`, located at the flow.
- Step 3b: C1 `unsupported`, `expression-form` at the `/` of `40 / 2`, mode `Moving`; C2 at the outer `/` and C3 at the `if`, naming `Moving` and `v`; C4 at the `/` in `half`'s body, naming `Moving`; C5 and C6 not `unsupported`.
- Step 3c: declared `v` then `a`: `unsupported`, `expression-form` at the outer `/` of `-a / (1 + v)`, naming `Moving` and `v`, not at `a`'s equation; declared `a` then `v`: at the `/` of `1 / (1 + a)`, naming `a`.
- Step 4: `HybridProviderAbsent`; then routed to the fixture.
- Step 5: one candidate each, no `ambiguous-backend`; with two hybrid providers, a named one routes and naming neither settles `ambiguous-backend`.
- Step 6: the published bounds and options; `incomplete`, `limit-reached` naming `hybrid.max_solver_steps`.
- Step 7: `proved`, `certified` with a different request; two `invalid-request` items naming the option.
- Step 8: one `Stopped` jump step at time `[0, 10]`; no `Moving` box below `v = 0`.
- Step 9: `proved`, `inductive-solver`, `certified`.
- Step 10: `refuted` with `end` at time 3.
- Step 11: `refuted` through an accepted `validated-witness`.
- Step 12: the layer-H dependency rule holds.
- Step 13: `from` `{"dwell": 0, "transition": 0}`, accepted; `Drop` `proved`, `jumps` 1, `exhaustive` holding `jumps`, `certified`, with its depth-1 dwell; `Fork` `proved`, `certified`, with two jump steps.
