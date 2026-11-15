---
id: FR-635
title: "Route analyze items to external providers through the backend registry"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-288
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-289
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-489
    type: depends_on
---
# FR-635: Route analyze items to external providers through the backend registry

## Description

The registry an analyze request negotiates against SHALL be `Engines::fsl()`
together with the external providers the driver loaded from their manifests
(FR-288, FR-489), and an item SHALL reach an external provider only by
negotiation, as FR-281's "External provider arm" states. The outcome SHALL
record which backends the registry held, so a result names what it was
decided against.

## Inputs

### Request item

- `backend`: optional; naming a provider's `BackendId` selects it among the
  candidates (FR-075).
- `options`: the provider's option names and string values (FSpec FR-196),
  checked against its manifest's published options.
- The document names no provider binary or path: which providers exist is
  the driver's configuration, not the request's, so the same request means
  the same thing on every machine whose registry holds the same backends.

## Outputs

- The document member `registry`: the sorted list of `{backend, tool}` of
  every registered backend, `tool` the descriptor's FSpec FR-331 tool record.
- Item members: a provider's `proved` record names the provider with
  certification `uncertified` for a linked first-party provider and
  `trusted` for a plugin; `refuted` only after S6a replay reproduced the
  provider's counterexample; `unknown` settles `inconclusive`
  `solver-inconclusive` (FR-281).

## Behavior

- **Unchanged results.** Installing a provider changes no item that does
  not negotiate to it (FR-289).
- **Budgets and cancellation.** `deadline_ms` is the provider deadline
  (FSpec FR-462); cancellation settles a provider item `incomplete` with its
  cause.
- **Refusals.** A named `backend` the registry does not hold settles
  `invalid-request` `unknown-backend`; several candidates and no named backend
  settle `invalid-request` `ambiguous-backend`; an unknown option refuses the
  item `invalid-request` naming it; a provider manifest that does not
  advertise its claimed kind and mode is refused at registration.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-635-AC-1 | With a test provider registered for `value-validity` `unbounded`, a request naming it as `backend` settles the item on it, and the document's `registry` lists `en-1`, `en-7` and the provider. | Test (TC-1595) |
| FR-635-AC-2 | The Counter item `Bounded` settles byte-equal with and without the test provider registered. | Test (TC-1595) |
| FR-635-AC-3 | `backend` naming an unregistered `BackendId` settles `invalid-request` `unknown-backend`, exit 20. | Test (TC-1595) |

## Dependencies

- FR-281 owns the provider arm; FR-288, FR-289 and FR-489 own the registry.
