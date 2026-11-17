---
id: FR-1019
title: "Ship a first-party reference hybrid provider over the provider seam"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-specification/FR-193 filament-specification/FR-331 filament-specification/FR-652 filament-spec-language/FR-750 filament-spec-language/FR-751
    type: depends_on
---
# FR-1019: Ship a first-party reference hybrid provider over the provider seam

## Description

FSL SHALL ship `fsl-hybrid-reference`, a first-party hybrid provider that implements FR-750's `HybridProvider` over the `fsl_analyze::provider` seam and is reached only through FSpec FR-193 negotiation, so that a hybrid claim whose flows are polynomial gets a verdict with no external solver installed. It does not replace the provider path: an external provider remains a candidate, and a hybrid item with no candidate still settles `unsupported` (FR-750-AC-3).

## Behavior

- **Registration.** The provider SHALL be made by `Engine::hybrid` from a descriptor with backend `fsl-hybrid-reference`, `advertises` `temporal-satisfaction`, `dynamics` `["hybrid"]` and `bounds` holding its published `max_solver_steps` and `max_solver_time` defaults. `Engines::fsl()` does not include it; the driver's default engine set and `Engines::fsl_with_reference_hybrid()` do.
- **Class.** The provider SHALL decide hybrid models whose every flow right-hand side is a polynomial with exact rational coefficients over the continuous variables, and whose guards, resets and invariants are polynomial (in)equalities. For any other model it SHALL return FSpec FR-331 `unsupported` with `unsupported_construct`/`expression-form` located at the first non-polynomial flow, naming its mode and variable; that cause is typed and located, never a bare `unsupported`.
- **Method.** Within its class the provider SHALL compute an outward-rounded interval flowpipe in exact rational arithmetic, rounding each enclosure bound outward to the step's declared precision, up to the claim's horizon (FSpec FR-193 "Proof coverage") and the jump bound, and SHALL report method `flowpipe-outward-rounded`. An enclosure disjoint from the violating states settles `proved`, `bounded-solver`; it SHALL return the flowpipe as an FSpec FR-652 certificate, which FR-751 checks in the core, so an accepted proof is `certified`.
- **Refutations.** An enclosure that meets the violating states SHALL yield a candidate point trace; it refutes only after FR-450 exact replay or an accepted FSpec FR-652 validated witness, otherwise the item settles `inconclusive`, `solver-inconclusive`.
- **Budgets.** `hybrid.max_solver_steps` and `hybrid.max_solver_time` SHALL stop it `incomplete` with `limit-reached` and `timed-out` (FSpec FR-193 "Solver budgets").
- **Examples.** `examples/hybrid-braking/` SHALL hold `Braking` with `SlowEnough` (holds within horizon 10) and `TooSlow` (`always [0, 10] holds(h.v > 5)`, violated), each with an `analyze` request that reaches a verdict through the reference provider.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1019-AC-1 | With the reference provider registered and no other hybrid provider, `SlowEnough` over `Braking` settles `proved`, `bounded-solver`, method `flowpipe-outward-rounded`, certification `certified` after FR-751 accepts its certificate, backend `fsl-hybrid-reference`. | Test (TC-2869) |
| FR-1019-AC-2 | `TooSlow` settles `refuted` with one `HybridCounterexample` that exact replay reproduces. | Test (TC-2869) |
| FR-1019-AC-3 | A model with flow `v' = sin(v)` settles `unsupported` from the reference provider with `unsupported_construct`/`expression-form` located at that flow, naming the mode and `v`. | Test (TC-2869) |
| FR-1019-AC-4 | Under `Engines::fsl()` alone the same item still settles `unsupported`, `HybridProviderAbsent`; with both the reference provider and a fixture external provider registered, a named selection of the fixture routes to the fixture. | Test (TC-2869) |

## Dependencies

- FR-193 filament-specification/FR-331 filament-specification/FR-652 filament-spec-language/FR-750 filament-spec-language/FR-751 (filament-specification).
