---
id: FR-1019
title: "Ship a first-party reference hybrid provider over the provider seam"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-specification/FR-193 filament-specification/FR-331 filament-specification/FR-652 filament-spec-language/FR-750 filament-spec-language/FR-751
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: depends_on
---
# FR-1019: Ship a first-party reference hybrid provider over the provider seam

## Description

FSL SHALL ship `fsl-hybrid-reference`, a first-party hybrid provider that implements FR-750's `HybridProvider` over the `fsl_analyze::provider` seam and is reached only through FSpec FR-193 negotiation, so that a hybrid claim whose flows are polynomial gets a verdict with no external solver installed. It does not replace the provider path: an external provider remains a candidate, and a hybrid item with no candidate still settles `unsupported` (FR-750-AC-3).

## Behavior

### Placement

- The provider SHALL be the library crate `fsl-hybrid-reference`, ADR-011
  layer H. It implements `HybridProvider`, which `fsl-replay`'s `hybrid`
  module defines and `fsl-analyze` re-exports (FR-750), and it depends only
  on the crates of layer H's ADR-011 cell. `fsl-analyze` depends on it and
  builds its engine. The solver sits outside the qualified core: a proof it
  returns is `certified` only after the layer-6 checker accepts its
  certificate (FR-751), so no claim rests on the solver's own correctness.
  Placing the trait in layer 6 lets the provider crate sit below the layer
  that registers it, with no dependency cycle and no second provider seam.

### Registration and selection

- **Registration.** The provider SHALL be made by `Engine::hybrid` from a descriptor with backend `fsl-hybrid-reference`, `advertises` `temporal-satisfaction` in the `bounded` and `unbounded` modes, `dynamics` `["hybrid"]`, `bounds` `{"max_solver_steps": 1000000, "max_solver_time": 60}` and `options` `{"precision_bits": "64", "step": "1/16", "refine_depth": "32"}`. The two bounds are FR-255's published defaults for `hybrid.max_solver_steps` and `hybrid.max_solver_time`; the time budget equals the SMT backend's `smt.max_solver_time`, the other wall-clock solver budget, and the step budget is in the range of the engine step and state budgets beside it. `Engine::hybrid_reference()` SHALL return that engine. `Engines::fsl()` does not include it; `Engines::fsl_with_reference_hybrid()` is `Engines::fsl()` with it added, and is the driver's default engine set. The engine set is a library parameter, never a request member (FR-1021): the `fsl_command::command` entries build their registry from `Engines::fsl()` and the `providers` they take (FR-635), so a caller reaches the reference provider by passing `Engine::hybrid_reference()` among `providers`, as the driver does by default. `analyze`, which takes no providers, settles a hybrid item `HybridProviderAbsent`.
- **Selection.** The descriptor's `dynamics` SHALL be exactly `["hybrid"]`, never `discrete`, so the reference provider is never a candidate for a temporal item without hybrid dynamics and never makes `fsl-model-check` ambiguous (FSpec FR-193-AC-18). For a hybrid item, FSpec FR-290's candidate table then applies unchanged: when the reference provider is the only registered hybrid provider it is the one candidate and the item is `supported` on it; when the request names `fsl-hybrid-reference` (or another registered hybrid backend), the named backend is the one candidate. Neither case settles `ambiguous-backend`. Only a request that registers two hybrid providers and names neither settles `ambiguous-backend`, naming both, as FSpec FR-290 states.
- **What routing already provides.** `fsl-route` computes the candidate set with the dynamics filter (`Registry::candidates_for_temporal_dynamics`, FR-075-AC-19), the request writer writes `requirements` from the checked claim's hybrid-dynamics flag, and `settle_candidates` applies FSpec FR-290's table, the named-backend and `ambiguous-backend` rows included. What this requirement adds is the engine side: `Engine::hybrid` registering a descriptor that carries `dynamics` and its solver `bounds` into `Engines::registrations()`, `ItemForm::Hybrid` built for a hybrid claim with its limits and options (FR-750), and dispatch of a `supported` hybrid item to the engine that serves its candidate.

### Class

- The provider SHALL decide hybrid models whose every flow right-hand side, after `let` substitution and expansion of non-recursive calls, is a polynomial with exact rational coefficients over the continuous variables (built from constants, variables, unary `-`, `+`, `-` and `*`), and whose guards, resets, invariants and initial conditions are Boolean combinations of polynomial (in)equalities. A flow whose rates depend on one another cyclically, such as `v' = v`, is in the class. Claim atoms take every FSpec FR-193 expression form; a `/` in an atom is read by interval evaluation, and a box over which its divisor can be 0 is a possible `undefined-evaluation` violation.
- For any other model it SHALL return FSpec FR-331 `unsupported` with `unsupported_construct`/`expression-form` located at the first construct outside the class in source order, naming its mode and, for a flow, its variable; that cause is typed and located, never a bare `unsupported`. The provider answers `HybridVerdict::Unsupported` (FR-750), which settles the item `unsupported` with that diagnostic and runs no replay.

### Options

The item's FSpec FR-331 `options` select the method's settings, each a string; an absent option takes the descriptor's published default, and each option is part of the request identity, since it can change the result.

- `precision_bits`, a decimal integer from 1 to 4096, default `64`: the precision every enclosure bound is rounded to. The precision is the request's, not the model's, because it changes how tightly the solver encloses behaviours and never what the model means: outward rounding never narrows an enclosure.
- `step`, a positive exact rational number of seconds written as `n/d` or a decimal literal, default `1/16`: the flowpipe's initial time step.
- `refine_depth`, a decimal integer from 0 to 1024, default `32`: how many times the provider halves a step or bisects a box before it gives up on a refinement.

The descriptor SHALL publish the three names with their defaults through `with_options`, so routing knows them. An item that names `fsl-hybrid-reference` with an option name the descriptor does not publish settles `invalid-request` `invalid_capability`/`unknown-option` by FR-967, before candidates are computed. A published option whose value lies outside its domain is refused by the provider's `HybridProvider::check_options` (FR-750), which `fsl-analyze` calls before `solve`: the item settles `invalid-request` with `invalid_capability`/`invalid-option-value` (FSpec FR-779), payload `request_index`, `backend`, `option` and `value`, and no solver step runs. The domains are the provider's, not routing's, because only the provider knows what it can compute with: `fsl-route` publishes names and defaults, and a value domain such as "a positive rational" is not a closed set routing could enumerate.

### Method

- **Arithmetic.** Within its class the provider SHALL compute in exact rational interval arithmetic by FR-450's interval evaluator, and SHALL round each enclosure bound outward to a multiple of `2^-precision_bits` after every step: a lower bound to the greatest such multiple not above it and an upper bound to the least not below it. It SHALL report method `flowpipe-outward-rounded` for a bounded proof.
- **Initial boxes.** Each initial entry (FSpec FR-193 "Initial states") gives one start box, the interval hull of the states that satisfy its condition and its mode's invariant, computed by interval constraint propagation from the invariant. A start box with a variable left unbounded settles the item `inconclusive`, `solver-inconclusive`, with no trace, since no finite enclosure exists.
- **Dwells.** From a start box `X` at time interval `T`, each step of length `h` SHALL find an a-priori box `B` with `X + [0, h] * f(B) ⊆ B` (the Picard condition) by iterating `B := X + [0, h] * f(B')`, where `B'` widens `B` by one eighth of each width plus `2^-precision_bits`, up to eight times; when no iteration succeeds it halves `h`, at most `refine_depth` times, and a step that still fails ends the run `inconclusive`, `solver-inconclusive`. The next box is `X + h * f(B)`. Both boxes are contracted by the mode's invariant, and a dwell whose box empties ends there. Every Picard attempt is one solver step.
- **Guards and boundary invariants.** For each outgoing transition, the guarded part of each step's `B` is its contraction by the guard, as FSpec FR-652 "Guards and boundary invariants" defines it. The jumps of one transition from one dwell form one target dwell, whose start box is the hull of the reset's images of every non-empty guarded part and whose start time is the hull of those steps' time intervals. When the guard has an equality conjunct `x = c` whose rate for `x` is built only from variables with rate `0` in the source mode, the provider SHALL narrow that start time to FSpec FR-652's narrowed jump time whenever the divisor's interval excludes 0: the interval value of `(c - x0) / r` over the dwell's start box, offset by its start time, intersected with `[0, horizon]`. That quotient is the exact jump time of each behaviour, `v0 / a` in `Braking`, whose value `[0, 20]` intersects the horizon 10 to `[0, 10]`; a jump after the horizon is outside the proof region. Since the source boxes are contracted by the invariant, a boundary invariant such as `v >= 0` at the guard `v = 0` cuts the source dwell at the guard, so no enclosure straddles the jump; the target dwell starts at the reset image. Dwells are explored in creation order, each completed before its target dwells start, so the exploration and its certificate are deterministic.
- **Bounds.** Under `model-time` a dwell runs until its time interval's lower end passes the claim's horizon (FSpec FR-193 "Proof coverage"), and the proof reports that `horizon`, `jumps` the greatest jump count any dwell reached, and `exhaustive` `["time"]` when every dwell ran to the horizon or emptied. Under `model-steps` with horizon `n` a dwell runs until it reaches its `n`-th jump or until its step box is contained in the previous one and meets no guard, which reaches no further jump; `exhaustive` holds `jumps` when every dwell ended so, and `horizon` is the greatest time reached.
- **Reading the claim.** Each atom SHALL be read over each step's `B` and time interval by three-valued interval evaluation, and the claim by its pointwise meaning over those readings, as FSpec FR-652's `flowpipe` rule reads it. A claim definitely true over every dwell settles `proved`, `bounded-solver`, and the provider returns the dwells as an FSpec FR-652 `flowpipe` certificate, which FR-751 checks in the core, so an accepted proof is `certified`.
- **Claims with no horizon.** For an `always`-form claim with no horizon, or an `on each` claim, the provider SHALL form one box per mode, the hull of that mode's step boxes, explored until a step adds nothing to its mode's hull, and SHALL return `proved`, `inductive-solver{method: inductive-enclosure}` with those boxes as an FSpec FR-652 `inductive-enclosure` certificate when its own check of rules (a) to (d) passes; otherwise it settles `inconclusive`, `solver-inconclusive`, with no trace.

### Refutations

An enclosure that meets the violating states SHALL yield a candidate point trace; it refutes only after FR-450 exact replay or an accepted FSpec FR-652 validated witness, otherwise the item settles `inconclusive`, `solver-inconclusive`. The candidate is chosen deterministically, so the same request always yields the same trace and replay reproduces it:

1. The *violating dwell* is the first dwell, in exploration order, with a step over which the claim is not definitely true; its *root box* is the start box of the initial entry its chain of dwells began from.
2. The candidate points of a box are, in order, its lower corner (every variable at its lower bound), its midpoint and its upper corner. A point is tried only when it satisfies the initial entry and the mode invariant exactly.
3. For each tried point the provider builds a point trace. In a mode whose flow is a polynomial flow in FSpec FR-193's sense (its rates have no cyclic dependency), the solution is a polynomial in time computed exactly; the trace jumps at the earliest instant at which an outgoing guard holds, the lowest transition position breaking a tie, and stops at the decisive instant. A jump instant that is not rational fails the point. When the decisive instant is a state's instant the trace ends at that state; otherwise it ends with FSpec FR-193's `end` at the decisive instant, or, when that instant is not rational, at the least multiple of `2^-precision_bits` after it at which the prefix is a bad prefix. In a mode whose flow has cyclic rates, the provider builds the dwell's Picard steps from the point, halving the step at most `refine_depth` times until each jump guard is definitely true over the box at which the jump is taken and a reading of the claim over a step's `box` or `B` is definitely false (FSpec FR-652 "Readings and the decisive box"), and returns them as an FSpec FR-652 `validated-witness` with the trace. The solution there is not a polynomial in time and its values at the decisive instant are in general irrational, so the trace ends with FSpec FR-193's `end` holding the decisive box: `end.time` is the instant of a `box` reading, as a point interval, or the step's `time` for a `B` reading, and `end.enclosure` is that box, with rational bounds. When the decisive instant is also the instant at which a guard holds, the trace stops there without taking the jump, as for a polynomial flow (FR-1019-AC-2). A point whose boxes stay unknown fails: a reading that straddles the atom's boundary never yields a witness, and the provider never refutes from it.
4. The first point whose trace exact replay reproduces, or whose witness the provider's own FSpec FR-652 check accepts, is the candidate, and the provider returns `refuted` with that trace, as `HybridVerdict::Refuted` (FR-750), which settles `refuted` only after FR-450 reproduces it or, for a trace exact replay cannot decide, after FR-751 accepts its witness (FR-450 "Settlement"). When no point of the root box succeeds, the provider bisects it on its widest variable (declaration order breaking a tie) at the midpoint, keeps the halves whose flowpipe still meets the violating states, lower half first, and repeats from step 2 depth-first, at most `refine_depth` levels.
5. When every candidate fails, the item settles `inconclusive`, `solver-inconclusive`, carrying the violating dwell's enclosure trace as unverified evidence.

Over `Braking` against `TooSlow`, the first step of `Moving` meets `v <= 5`, and the root box's lower corner `(Moving, v = 0, a = 1)` at time 0 already violates the claim, so the counterexample is that start state with no jump.

### Budgets

`hybrid.max_solver_steps` and `hybrid.max_solver_time` SHALL stop the provider `incomplete` with `limit-reached` and `timed-out` (FSpec FR-193 "Solver budgets"). Each Picard attempt counts one step, including those of bisection and witness refinement, and the time budget is the wall-clock time of the whole run.

### Examples

`examples/hybrid-braking/` SHALL hold `braking.native`, the `Braking` model without `t` with `SlowEnough` (`always [0, 10] holds(h.v < 30)`, which holds) and `TooSlow` (`always [0, 10] holds(h.v > 5)`, violated); `slow-enough.analyze.json` and `too-slow.analyze.json`, each a `native-analyze/1` document (FR-620) whose `program.source` names `braking.native` relative to the document and whose one item is `{"claim": "SlowEnough"}` or `{"claim": "TooSlow"}`, naming no backend; and a `README.md` stating how to run them. Read through `fsl_command::command::analyze_with_providers` with `Engine::hybrid_reference()` as the one provider, which is what `fsl analyze` passes by default, the first settles `proved`, `certified` and the second `refuted`, each with no external provider configured.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1019-AC-1 | With the reference provider registered and no other hybrid provider, `SlowEnough` over `Braking` settles `proved`, `bounded-solver` with `horizon` 10, `jumps` 1 and `exhaustive` `["time"]`, method `flowpipe-outward-rounded`, certification `certified` after FR-751 accepts its certificate, backend `fsl-hybrid-reference`. | Test (TC-2869) |
| FR-1019-AC-2 | `TooSlow` settles `refuted` with one `HybridCounterexample` that exact replay reproduces, whose trace is the start state `(Moving, time 0, v = 0, a = 1)` with no jump; two runs of the same request give equal counterexamples. | Test (TC-2869) |
| FR-1019-AC-3 | A model whose `Moving` flow is `v' = -a / (1 + v)` settles `unsupported` from the reference provider with `unsupported_construct`/`expression-form` located at that flow, naming the mode and `v`. | Test (TC-2869) |
| FR-1019-AC-4 | Under `Engines::fsl()` alone the same item still settles `unsupported`, `HybridProviderAbsent`; with both the reference provider and a fixture external provider registered, a named selection of the fixture routes to the fixture. | Test (TC-2869) |
| FR-1019-AC-5 | With `fsl-model-check` and the reference provider registered, the hybrid item has the one candidate `fsl-hybrid-reference` and a discrete temporal item the one candidate `fsl-model-check`, neither `ambiguous-backend`; with a fixture hybrid provider also registered, a request naming `fsl-hybrid-reference` routes to it without `ambiguous-backend`, and one naming neither settles `invalid-request`, `ambiguous-backend`, naming both. | Test (TC-2869) |
| FR-1019-AC-6 | `Engines::fsl_with_reference_hybrid()` holds every engine of `Engines::fsl()` and also `Engine::hybrid_reference()`, which registers `fsl-hybrid-reference` with `dynamics` `["hybrid"]`, `bounds` `{max_solver_steps: 1000000, max_solver_time: 60}` and `options` `{precision_bits: "64", step: "1/16", refine_depth: "32"}`; `SlowEnough` with `hybrid.limits.max_solver_steps` 1 settles `incomplete`, `limit-reached{limit: max_solver_steps, value: 1, setting: hybrid.max_solver_steps}`. | Test (TC-2869) |
| FR-1019-AC-7 | `SlowEnough` with `options` `{"precision_bits": "8", "step": "1/4"}` still settles `proved`, `certified`, and its echoed request differs from the default one; `{"step": "0"}` on an item naming `fsl-hybrid-reference` settles `invalid-request` `invalid_capability`/`invalid-option-value` with `option` `step` and `value` `0`, refused by `check_options` before any solver step; `{"colour": "blue"}` on such an item settles `invalid-request` `invalid_capability`/`unknown-option` with `option` `colour` by FR-967, and never `invalid-option-value`. | Test (TC-2869) |
| FR-1019-AC-8 | `SlowEnough`'s flowpipe certificate has exactly one jump step, in `Stopped`, with box `v [0, 0], a [1, 2]` and time `[0, 10]`, the narrowed jump time (`v0 / a` over `v0 [0, 20], a [1, 2]`, `[0, 20]`, intersected with the horizon `[0, 10]`), and no `Moving` step box holds a value of `v` below 0. | Test (TC-2869) |
| FR-1019-AC-9 | `always holds(h.v < 30)` over `Braking` settles `proved`, `inductive-solver{method: inductive-enclosure}`, `certified`, with the boxes `Moving: v [0, 20], a [1, 2]` and `Stopped: v [0, 0], a [1, 2]`. | Test (TC-2869) |
| FR-1019-AC-10 | A model `Coast` with the one mode `Down`, flow `v' = -1`, invariant `v >= 0` and `initial { mode Down when { v = 8 }; }` refutes `always [0, 10] holds(h.v > 5)` with the trace starting at `(Down, time 0, v = 8)`, no jump and `end` `(time 3, v = 5)`, which exact replay reproduces. | Test (TC-2869) |
| FR-1019-AC-11 | A model `Grow` with flow `v' = v`, invariant `v >= 1 and v <= 100` and `initial { mode Grow when { v = 1 }; }` refutes `always [0, 5] holds(h.v < 30)` through a `validated-witness` that FR-751 accepts, and the trace is recorded as its counterexample. | Test (TC-2869) |
| FR-1019-AC-12 | `fsl-hybrid-reference`'s `[dependencies]` name only crates in layer H's ADR-011 cell, `fsl-analyze` depends on it, and no crate of the qualified core depends on it. | Test (TC-2869) |
| FR-1019-AC-13 | `examples/hybrid-braking/slow-enough.analyze.json` and `too-slow.analyze.json`, read through `fsl_command::command::analyze_with_providers` with `Engine::hybrid_reference()` as the one provider and no external provider configured, settle `proved`, `certified` and `refuted` respectively, each with backend `fsl-hybrid-reference`. | Test (TC-2997) |

## Dependencies

- FSpec FR-193, FR-290, FR-331 and FR-652 (filament-specification); FR-750, FR-751, FR-450, FR-620, FR-621, FR-635 and ADR-011.
