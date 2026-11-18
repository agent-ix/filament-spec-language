---
id: FR-750
title: "Run a hybrid item through its negotiated hybrid provider and settle what it returns"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-447
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-250
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-450
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-404
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-193
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
---
# FR-750: Run a hybrid item through its negotiated hybrid provider and settle what it returns

## Description

`analyze` SHALL run every hybrid item (an FSpec FR-331 `temporal-satisfaction`
item carrying the `hybrid-dynamics` requirement) on the hybrid provider that
negotiation routed it to, in process or in a process of its own, and SHALL
settle the provider's result by FR-250 and FR-450. FSL owns the provider
interface, the dispatch, the replay and the settlement; the solver is the
provider's, by design (ADR-026). A hybrid item with no provider settles
`unsupported`.

## Inputs

A checked package holding the claim's `CheckedHybridModel` and
`CheckedHybridClaim` (FR-447); one negotiated hybrid item; the registered
engines; and the request's limits, deadline and cancellation.

## Outputs

One terminal record per hybrid item and, when exact replay or an accepted
validated witness reproduces a trace, one `counterexamples` entry.

## Behavior

### The in-process provider interface

- `fsl-analyze` SHALL export a trait `HybridProvider`, as
  `fsl_analyze::provider::HybridProvider`, with one method that takes the
  item's typed request `HybridSolveRequest` (its `CheckedHybridClaim`, its
  `CheckedHybridModel`, its resolved `hybrid.limits` as FR-250's
  `SolverBudget`, its FSpec FR-331 `options` and `deadline_ms`, read from the
  checked package and the item) and the cancellation token, and returns a
  `HybridAnswer`: FR-250's `SolverResult`, optionally with an FSpec FR-652
  certificate. The trait, the request and the answer are defined in
  `fsl-replay`'s `hybrid` module beside `SolverResult` and re-exported by
  `fsl-analyze`, so a provider crate below layer A, such as
  `fsl-hybrid-reference` (FR-1019, ADR-011 layer H), implements it without
  depending on `fsl-analyze`.
- `HybridAnswer::read_entry` SHALL read one FSpec FR-331 hybrid result entry
  (its value, basis or cause, trace, counterexamples and `certificate`) into
  a `HybridAnswer`, refusing an entry FSpec FR-331 refuses with that cause
  and pointer. The process path below reads a provider's entry through it,
  and an in-process provider may build its answer through it.
- `Engine::hybrid(descriptor, provider)` SHALL make an engine from a
  descriptor whose `dynamics` holds `hybrid` and that advertises
  `temporal-satisfaction`. A descriptor that does not is refused with a typed
  error naming the backend and the cause `invalid-dynamics`, and no engine
  is made.
- `Engine::hybrid_process(descriptor, tool, exchange)` SHALL make the engine
  of a process provider from its `Process` descriptor, under the same
  descriptor rule, and an exchange that takes the FR-447 process request
  frame bytes and the cancellation token and returns the provider's FSpec
  FR-331 result frame bytes or a typed tool failure. The driver's plugin host
  implements the exchange over the child process (FR-635); FSL writes the
  frame and reads the result, so the transport carries bytes only.
- `HybridItem`, the `ItemForm::Hybrid` settings, SHALL carry the item's
  proof bounds, its `hybrid.limits` (`max_solver_steps` and
  `max_solver_time`, each optional and taking the serving descriptor's
  `bounds` when absent, FSpec FR-193 "Solver budgets") and its FSpec FR-331
  `options`. FR-620's item resolution SHALL build `ItemForm::Hybrid` for
  every item whose claim is a checked hybrid claim, reading the document
  item's `hybrid` and `options` members, so a `native-analyze/1` item over a
  hybrid claim reaches the dispatch below.

### Dispatch

- For a hybrid item whose disposition is `supported` by a candidate that an
  engine made by `Engine::hybrid` serves, `analyze` SHALL call that engine's
  provider once, and SHALL settle its `SolverResult` by FR-250's verdict map
  and coverage rule and replay its traces by FR-450, so a provider's
  `refuted` settles `refuted` only after exact replay or an accepted
  validated witness.
- For a hybrid item whose candidate is a process provider (FR-404's plugin
  replay context), `analyze` SHALL write the FR-447 process request with the
  emitted package's v2 bytes, read the provider's FR-331 result entry, and
  settle it by the same rules. A clause request (a `native-analyze/1`
  request whose items name hybrid clauses by declared name, FR-620) SHALL
  offer process-provider composition for its hybrid clauses exactly as for
  an item request: the process provider is one of the `providers` the
  command entry takes (FR-635), made by `Engine::hybrid_process`.
- A settled hybrid item SHALL be written into FR-621's outcome item: its
  `record` carries `result`, the proof `basis` or the `cause`, the
  `certification` of a proved record and `backend`; a `refuted` item carries
  its one `HybridCounterexample` as `counterexample` with `replay`; and an
  item whose provider returned a certificate carries `certificate`, with
  `rule` and `at` when the certificate was rejected (FR-621).
- A hybrid item with an empty candidate set SHALL settle `unsupported`,
  `HybridProviderAbsent` (`unsupported_projection`/
  `unsupported-requested-capability`, its warning naming
  `temporal-satisfaction` and `hybrid-dynamics`), and no engine runs.
- A hybrid item routed to a candidate that no registered engine or process
  provider serves SHALL settle `unsupported`,
  `unsupported_projection`/`tool-unavailable` naming that backend.
- A provider that returns a certificate SHALL have it checked by FR-751
  before the record is written.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-750-AC-1 | With an in-process fixture provider registered through `Engine::hybrid`, `SlowEnough` over `Braking` is dispatched once to it, and its `delta-complete` `unsat` covering horizon 10 settles `proved`, `bounded-solver`, `uncertified`, naming the fixture backend. | Test (TC-2100) |
| FR-750-AC-2 | `Engine::hybrid` with a descriptor whose `dynamics` lacks `hybrid` returns a typed error naming the backend and makes no engine. | Test (TC-2100) |
| FR-750-AC-3 | With no hybrid provider registered, the hybrid item settles `unsupported`, `HybridProviderAbsent`, `unsupported-requested-capability`, its warning naming `temporal-satisfaction` and `hybrid-dynamics`, through `analyze` and through `provider::run`. | Test (TC-2100) |
| FR-750-AC-4 | A hybrid item routed to `hybrid-fixture` while no engine serves it settles `unsupported`, `tool-unavailable`, naming `hybrid-fixture`. | Test (TC-2100) |
| FR-750-AC-5 | A process fixture provider receiving the v2 bytes of a clause request over `SlowEnough` returns a `refuted` point trace that exact replay reproduces; the item settles `refuted` with one `HybridCounterexample`; a trace that does not reproduce settles `inconclusive`, `replay-parity`. | Test (TC-2100) |

## Dependencies

- FR-447 (request writing), FR-250 (verdict map, budgets), FR-450 (replay),
  FR-404 (process provider context), FR-751 (certificates).
- FSpec FR-193 and FR-331.
