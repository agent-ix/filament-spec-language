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

### Where provider manifests come from

- **Linked providers.** The descriptors `Engines::fsl()` exposes through
  `Engines::registrations()` (FR-489), each with origin `Linked`.
- **Process providers.** Only the plugins the driver's provider
  configuration names: the `--provider-config <file>` document,
  FSpec FR-561's `filament.provider-config/v1`, read by the driver (FR-287).
  There is no search path, environment variable or default directory, so
  the providers a run uses are exactly the ones its invocation configured.
  For each entry, in document order, the driver SHALL start the executable
  at `path` with its `arguments` as a child process speaking
  `filament.plugin-wire/v1` (FSpec FR-462), read the FSpec FR-331 manifest
  from its `hello` frame, convert it with FR-288's one conversion under
  origin `Process`, and register the descriptor beside the linked ones
  before any negotiation. A plugin that does not start, sends no valid
  `hello`, or whose manifest is refused is a registration failure under
  FR-288 and FR-291, naming the entry's `path`; the registry never holds it.
- **The analyze entry.** `fsl_command::command::analyze_with_providers(path,
  vocabulary, providers, cancel)` SHALL take the started process providers
  as values, each its `Process` descriptor and the wire handle the item is
  sent over, and SHALL build the run's registry from `Engines::fsl()` and
  those providers; `analyze(path, vocabulary, cancel)` is the same entry
  with none. FSL's command reads no provider configuration of its own
  (FR-287): starting the processes is the driver's.

### How a process provider runs an item

An item settled `Supported` on a process provider's `BackendId` is sent to
that provider as one FSpec FR-462 request frame carrying the FSpec FR-331
request item and the package's v2 bytes, under `deadline_ms` as the
provider deadline and the run's cancel handle. Its result frame is read by
the typed FR-331 reader and settled by FR-290: `proved` with certification
`trusted`, `refuted` only after S6a replay reproduces the counterexample,
`unknown` as `inconclusive` `solver-inconclusive`; a crash, a malformed
frame or an expired deadline settles as FR-291 states.

### Relation to the production layering registry

FR-489's production snapshot is the linked set alone and holds no
registration made only for a test. A configured process plugin is not such
a registration: it is the user's own configuration, reaches the registry
through the same `hello` conversion as every plugin, and is never part of
FR-489's snapshot. The analyze tests therefore register their test provider
as a real process plugin, a Rust executable built from the workspace and
handed to `analyze_with_providers`, and leave `Engines::fsl()` and FR-489's
builder unchanged.

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
| FR-635-AC-1 | With the workspace's test plugin started as a process provider advertising `value-validity` `unbounded` and passed to `analyze_with_providers`, a request naming it as `backend` settles the item `proved` on it with certification `trusted`, and the document's `registry` lists `en-1`, `en-7` and the provider. | Test (TC-1595) |
| FR-635-AC-2 | The Counter item `Bounded` settles byte-equal with and without the test provider registered. | Test (TC-1595) |
| FR-635-AC-3 | `backend` naming an unregistered `BackendId` settles `invalid-request` `unknown-backend`, exit 20. | Test (TC-1595) |
| FR-635-AC-4 | A process provider whose `hello` manifest does not advertise (`value-validity`, `unbounded`) is refused at registration naming its `BackendId`, and the run's `registry` does not list it; a production layering snapshot (FR-489) built in the same process as AC-1's run holds exactly the linked descriptors and not the test plugin. | Test (TC-2294) |

## Dependencies

- FR-281 owns the provider arm; FR-288, FR-289 and FR-489 own the registry.
