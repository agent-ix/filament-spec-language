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

- `fsl-analyze` SHALL export a trait `HybridProvider` with one method that
  takes the item's typed request (its claim, model, `hybrid.limits` and
  `deadline_ms`, read from the checked package) and the cancellation token,
  and returns FR-250's `SolverResult`, optionally with an FSpec FR-652
  certificate.
- `Engine::hybrid(descriptor, provider)` SHALL make an engine from a
  descriptor whose `dynamics` holds `hybrid` and that advertises
  `temporal-satisfaction`. A descriptor that does not is refused with a typed
  error naming the backend, and no engine is made.

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
  settle it by the same rules. A clause request (an `analyze` request that
  names clauses rather than items) SHALL offer process-provider composition
  for its hybrid clauses exactly as for an item request.
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
