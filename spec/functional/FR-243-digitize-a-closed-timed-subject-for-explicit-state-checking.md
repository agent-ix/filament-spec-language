---
id: FR-243
title: "Route timed items to the zone engine or the digital-clock route"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-235
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-238
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-239
    type: depends_on
---
# FR-243: Route timed items to the zone engine or the digital-clock route

## Description

The request writer SHALL route every non-probabilistic timed item (TT-1 to
TT-4, the deadlock-freedom item and the time-lock-freedom item) to EN-6's
zone search (FR-239), so a timed invariant is proved by `Holds` with a CF-1
certificate (ADR-026 EZ-10). Only a probabilistic claim over a timed
subject that names `exact` evidence takes the digital-clock route, EN-5's
reading of a closed subject through integer-valued clocks (ADR-028 TA-1 to
TA-3, FR-204), whose results settle by FR-203.

## Use case

A verification operator's model has only non-strict guards and deadlines.
Its timed invariant is proved by the zone engine with a certificate. A
probabilistic deadline claim over the same model with `exact` evidence is
decided on the digital-clock route with an exact value.

## Inputs

- A timed item, its capability kind, its `options.method` and, for a probabilistic claim, its
  requested evidence kind.

## Outputs

- The routed engine for the item: EN-6, EN-5's digital-clock route for an exact probabilistic claim, or EN-1 over the digital-clock reading for a `temporal-satisfaction` item that names `options.method` `digital-clocks`.

## Behavior

- When a timed item's capability kind is `temporal-satisfaction` and its
  `options.method` is absent or `zones`, negotiation SHALL route it to EN-6,
  whatever its constraints.
- `options.method` is a provider option, negotiated against each
  candidate's manifest like every other provider option (FR-635, FR-075).
  For a timed subject, EN-6's manifest publishes `method` with the one value
  `zones`, its default, and EN-1's manifest publishes `method` with the one
  value `digital-clocks`. An item over a timed subject that names
  `options.method` `digital-clocks` (FR-632) therefore has EN-1 as its only
  candidate and negotiates to EN-1's digital-clock route: EN-1's
  explicit-state search (FR-126) over the subject's integer-clock reading
  (ADR-028 TA-1 to TA-3), settling with basis `digitized-explicit-state`
  (FR-235). The caller asks for it by name, as a cross-check of the zone
  verdict; EZ-10's default route is unchanged, and EN-6 still never
  digitises (FR-239).
- EN-1's digital-clock route SHALL check, before any search, that the
  subject's constraints and the claim's intervals are all closed, the case
  in which digitisation preserves the dense-time verdict. When one is
  strict, the route SHALL decline: the item settles `declined`
  `invalid-input` naming the strict constraint or interval, with
  `settled_by` `result` and `backend` `en-1`, and no search runs.
- When a `probabilistic-satisfaction` item over a timed subject names
  `exact` evidence, the request writer SHALL route it to EN-5's
  digital-clock route (FR-204); when it names `statistical` evidence, to
  EN-4 (FR-254).
- The route SHALL be a function of the capability kind, the requested
  evidence kind and the item's `options` alone, negotiated against the
  published manifests, so one item always reaches one engine.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-243-AC-1 | `NoLateReply` over `Rpc` with `T = 4 ms` (all closed) and the deadlock-freedom item over the same subject route to EN-6 and return `Holds` with a certificate (FR-239-AC-1). | Test (TC-698) |
| FR-243-AC-2 | ADR-028 §15.5's `Deadline` over `Retx` with `exact` evidence routes to EN-5's digital-clock route and settles `proved`, `ExactValue{99/100}` (FR-204-AC-1); with `statistical` evidence it routes to EN-4. | Test (TC-698) |
| FR-243-AC-3 | ADR-026 §9's retry model, with strict constraints, routes `always holds(not retried)` to EN-6 and is refuted (FR-239-AC-2); the all-closed `Rpc` routes the same claim kind to EN-6 too. | Test (TC-698) |
| FR-243-AC-4 | `NoLateReply` over the all-closed `Rpc` with `T = 4 ms` and `options.method` `digital-clocks` routes to EN-1's explicit-state search over the integer-clock reading and settles `proved`, basis `digitized-explicit-state`, as the zone route proves it; the same item over ADR-026 §9's strict-constraint retry model settles `declined` `invalid-input` with `settled_by` `result` and `backend` `en-1`, and no search runs; with `options.method` absent it negotiates to EN-6. | Test (TC-2290) |

## Dependencies

- ADR-026 §8 EZ-10, §8.1 CF-4; ADR-028 TA-1 to TA-3 (EN-5's digital route).
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-235](FR-235-settle-a-timed-verdict-as-a-terminal-record.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md).

## References

- T. A. Henzinger, Z. Manna and A. Pnueli, "What good are digital clocks?",
  1992; J. Ouaknine and J. Worrell, 2003 (ADR-026 References).
