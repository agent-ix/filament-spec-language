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

### One rule for every provider-routing entry

The verbs that route items to providers are `analyze`, `prove`, `monitor`,
`run`, `replay` and `generate`, the verbs FR-287 gives `--provider-config`
and `--provider-limit`. FSpec FR-561 names the same verbs; `run` is one of
them because a run request's claim items (driver FR-006, FR-014) are
routed and negotiated like analyze items. Every FSL command entry those verbs reach SHALL take the started
process providers as one `providers` parameter, each value its `Process`
descriptor and its `ExternalProvider` transport (the driver's
`ProcessProvider`, "How a process provider runs an item"), and the run's
FSpec FR-462 limits as `AnalyzeOptions.provider_limits`, and SHALL build its
registry from `Engines::fsl()` and those providers, before any negotiation.
The entry with an empty `providers` is the entry with no plugin
configured. No entry reads a provider configuration of its own: the driver
starts the processes from `--provider-config` (FR-287) and passes them in,
so every verb sees the same registry for the same configuration. The entries
are `fsl_command::command::analyze_with_providers(path, vocabulary,
providers, cancel)` for `analyze` (and `prove`, which shares the analyze
items, FR-280), with `analyze(path, vocabulary, cancel)` the same entry with
none; `command::monitor` (FR-644); the `run` entry, whose provider items
go through the same provider request and transport (FR-950); and the FR-280
entries the driver's `prove`, `replay` and `generate` call. Each of those requirements refers to
this rule rather than restating it.

### How a process provider runs an item

FSL owns routing, negotiation and the routed items, so FSL also builds the
complete request a provider receives, and the driver only carries it.
Analyze SHALL work in this order (FR-950):

1. Build every item of the request, route each one (FR-620) and negotiate
   it through `filament-contract-codegen`'s `negotiate_item` (FSpec FR-711).
   No provider is called while items are built.
2. For each backend that holds at least one item settled `Supported`, build
   one `fsl_analyze::provider::ProviderRequest`. It is the typed mirror of
   the FSpec FR-331 request stage, serializes to exactly FR-331's wire, and
   defines no member of its own. Its members come from these sources and
   from nowhere else:
   - `package`: the `EmittedPackage` bytes of the analysed unit's own S4
     emission, read through the admitted read-back (FR-940
     `AdmittedPackageRead`), with the dependency closure FSpec FR-331
     "Package bytes" orders. Neither the driver nor the provider call
     re-emits the package. When `package` refuses to emit the checked
     package, there are no bytes: its process-provider items settle as FSpec
     FR-331 "In-process providers" states, and no request is built for them.
   - `items`: the FSpec FR-331 request items of the `RequirementItem`s that
     routing sent to this backend, in request order. Each item carries its
     `request_index`, claim `{node, occurrence}`, kind, extent, bounds,
     options and candidates.
   - `manifest`: the full registry snapshot the candidates were checked
     against, FR-331's request-stage `manifest`. The `Disposition` that
     `negotiate_item` returned for each item names the negotiated
     descriptor within it.
   - `request.deadline_ms`: the `deadline_ms` of the run's FSpec FR-462
     limits, `AnalyzeOptions.provider_limits`, which the driver fills from
     `--provider-limit` (FR-287). It is the only limit on the FR-331 wire.
   The request also holds two members that are not serialized and are not
   wire members: `transport_limits`, the other six FR-462 limits of
   `AnalyzeOptions.provider_limits`, which the transport enforces and a
   linked provider ignores; and each item's `ValueTarget`, which an
   in-process arm reads.
3. Call the widened provider call, `ExternalProvider::settle(&ProviderRequest)
   -> ProviderVerdict`, once per request, under the run's cancel handle.
   `fsl_analyze::provider::run` takes the analysis view (FR-940) or the
   admitted package and never a bare `CheckedPackage`, so the bytes it sends
   are the bytes that were admitted.

The driver's `ProcessProvider` (FR-287) implements `ExternalProvider` and
only transports. It starts the plugin (FSpec FR-561), converts the FR-331
`hello` manifest into the FSL descriptor (FR-288), sends the request it is
given unchanged as one FSpec FR-462 `request` frame through
`ProcessProviderSession::invoke` under the request's `deadline_ms` and `transport_limits`, and maps the
FR-331 `result` frame back to `ProviderVerdict`. It holds no package and no
routed item of its own, and it builds no request member. The typed FR-331
reader reads the result frame, and FR-290 settles it: `proved` with
certification `trusted`; `refuted` only after S6a replay reproduces the
counterexample; `unknown` as `inconclusive` `solver-inconclusive`. A crash,
a malformed frame or an expired deadline settles as FR-291 states. There is
one provider path. The earlier driver pipeline that rebuilt items and
packages itself (`run_configured_providers` with its own driven call) is
removed with no shim, so a request reaches a provider only by this rule.

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

FR-950 states the acceptance criteria of the provider request and its
transport.

## Dependencies

- FR-281 owns the provider arm; FR-288, FR-289 and FR-489 own the registry.
- [FR-950](FR-950-build-the-complete-provider-request-in-analyze-and-transport-it-unchanged.md):
  the provider request analyze builds and the driver transports.
