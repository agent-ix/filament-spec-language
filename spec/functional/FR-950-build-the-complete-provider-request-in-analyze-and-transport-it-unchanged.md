---
id: FR-950
title: "Build the complete provider request in analyze and transport it unchanged"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-462
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-711
    type: depends_on
---
# FR-950: Build the complete provider request in analyze and transport it unchanged

## Description

FSL's analyze SHALL build every request an external provider receives as a
complete FSpec FR-331 request, `fsl_analyze::provider::ProviderRequest`, and
SHALL hand it to the provider through the widened call
`ExternalProvider::settle(&ProviderRequest) -> ProviderVerdict`. That call
happens only after every item has been built, routed and negotiated. FSL
owns routing, negotiation and the routed items (FR-635, FSpec FR-711), so it
alone knows which items a backend received and against which manifest
entry. A request built anywhere else would need a second copy of that
knowledge. The driver's `ProcessProvider` therefore only transports the
request: it holds no package and no routed item. FR-635 "How a process
provider runs an item" gives the member sources and the order of work.

`ProviderRequest` is a typed mirror of FSpec FR-331's request stage. It
serializes to exactly that wire and adds no contract of its own:

| Member | Source |
| --- | --- |
| `package` (and `dependencies`) | the `EmittedPackage` bytes of the analysed unit's own S4 emission, read through FR-940's `AdmittedPackageRead` |
| `items` | the FR-331 request items of the `RequirementItem`s that routing sent to the backend, in request order |
| `manifest` | the negotiated manifest entry named by each item's `negotiate_item` `Disposition`, with the registry snapshot |
| limits | `AnalyzeOptions.provider_limits`, the FSpec FR-462 limits the driver resolves from `--provider-limit` (FR-287); `deadline_ms` is also FR-331's `request.deadline_ms` |

Each item's `ValueTarget` travels with the request for in-process arms. It
is not serialized.

## Inputs

- The analysis view or admitted package of the analysed unit (FR-940).
- The routed and negotiated items of the run.
- `AnalyzeOptions.provider_limits`.
- The registered providers (FR-635 "One rule for every provider-routing entry").

## Outputs

One `ProviderRequest` per backend holding at least one `Supported` item,
and one `ProviderVerdict` per request, which FR-290 and FR-281 settle.

## Behavior

- Analyze SHALL NOT call a provider while it builds an `AnalyzeItem`. It
  SHALL route and negotiate every item first, then build one request per
  backend, then call `settle` once per request.
- The request's package bytes SHALL be the admitted bytes (FR-940). A
  request's `package` decodes with the v2 reader to the package the run
  analysed.
- When `package` refuses to emit the checked package, an item routed to a
  process provider SHALL settle `unsupported`
  `unsupported_projection`/`unsupported-requested-capability`, as FSpec
  FR-331 "In-process providers" states, and no request SHALL be built or
  sent for it.
- `ProcessProvider` SHALL write the request it is given as the FR-462
  `request` frame body, byte-equal to the request's FR-331 serialization,
  and SHALL enforce the request's transport limits. It SHALL build, add or
  remove no request member.
- No driver code path SHALL build an FR-331 request item, read a package's
  bytes for a provider, or route an item. The earlier
  `run_configured_providers` pipeline does not exist, and no shim replaces it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-950-AC-1 | Over FR-635-AC-1's request with a recording provider registered as a process provider, the provider's `settle` is called once, after negotiation, with a `ProviderRequest` whose `package` decodes with the v2 reader to the admitted package of the analysed unit, whose `items` hold exactly the `Even` item with its `request_index`, claim occurrence and candidate, whose `manifest` holds the provider's descriptor as the item's `Disposition` names it, and whose `deadline_ms` is FR-287's default 600000. | Test (TC-2660) |
| FR-950-AC-2 | The same run with `AnalyzeOptions.provider_limits` setting `deadline_ms` 250 and `max_frame_bytes` 1048576 gives a request carrying those two values and FR-287's defaults for the other five limits; the request's FR-331 serialization is schema-valid against `urn:filament:backend-provider:v1` and equals the bytes the transport writes as the `request` frame body. | Test (TC-2660) |
| FR-950-AC-3 | Two items routed to one provider give one request holding both items in request order; an item that negotiates to `en-1` is in no provider request; no provider is called when no item settles `Supported` on it. | Test (TC-2660) |
| FR-950-AC-4 | Negative: an item routed to a process provider over a package that `package` refuses to emit (FR-185) settles `unsupported` `unsupported_projection`/`unsupported-requested-capability` naming the capability kind and the backend, and the provider's `settle` is never called. | Test (TC-2660) |

## Dependencies

- FR-635: the routing rule and the provider path this requirement completes.
- FR-287: `--provider-limit` and the driver's transport.
- FR-940: the admitted read-back the package bytes come from.
- FSpec FR-331, FR-462, FR-711: the request wire, its frame limits and the
  negotiation entry.
