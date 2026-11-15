---
id: FR-661
title: "Publish the build inventory of editions, profiles, families and features"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-111
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-288
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: references
  - target: ix://agent-ix/filament-specification/FR-560
    type: depends_on
---
# FR-661: Publish the build inventory of editions, profiles, families and features

## Description

A user who writes a header needs the exact edition and profile spellings the
build admits, and a consumer that reads a package's required-feature set
needs the exact capability ids the build knows (FSpec FR-132-AC-3). The
`fsl-replay` crate SHALL publish that inventory as one library operation,
`fsl_replay::build_inventory(&fsl_route::Registry) -> BuildInventory`, whose
`BuildInventory::to_bytes()` is exactly FSpec FR-560's
`filament.build-inventory/v1` document. The driver's `version` verb writes
that document (FSpec FR-560); it lists nothing of its own.

The inventory is read from the tables the compiler itself consults, never a
second list: the edition table S1 accepts in the `language ... edition`
header, the layer table E3 resolves header profiles against (FR-110), the
complete-V1 capability families and the closed capability inventory
(`CapabilityId::complete_inventory`, FR-111), and the linked providers of the
registry the caller passes (FR-288). A profile or feature therefore appears
in the inventory exactly when the build admits it.

## Inputs

The registry the caller built (FR-288); it supplies the `consumers` list.

## Outputs

`BuildInventory`: the application name and version the caller sets, and the
five lists FSpec FR-560 names (`editions`, `profiles`, `families`,
`features`, `consumers`), each in that requirement's order.

## Behavior

- `build_inventory` SHALL list every edition S1 accepts, every
  header-selectable profile identity of the layer table with its `requires`
  edges, every complete-V1 capability family with its feature ids, every
  capability id of `CapabilityId::complete_inventory()`, and every linked
  provider of the registry with its backend identity and advertised (kind,
  mode) pairs.
- `build_inventory` SHALL read no file, environment variable or clock, and
  two calls over the same registry SHALL give equal bytes.
- A profile identity the inventory lists SHALL resolve at E3 when a unit's
  header selects it, and `build_inventory` SHALL list every identity E3 resolves as a layer.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-661-AC-1 | `build_inventory` over a registry with no provider returns a document whose `features` equal `CapabilityId::complete_inventory()` in order, whose `families` group exactly those ids by family in normative order, whose `editions` hold `1-draft`, and whose `consumers` is `[]`. | Test (TC-1742) |
| FR-661-AC-2 | For every identity in the document's `profiles`, a `1-draft` unit whose header selects it as `profile p = "<identity>";` with its `requires` profiles resolves at E3 with no `unknown_profile` refusal; no listed identity begins with `quire.`. | Test (TC-1742) |
| FR-661-AC-3 | Two calls of `build_inventory` over the same registry give equal bytes; a registry holding one linked provider adds exactly one `consumers` entry naming its backend identity and advertised pairs and changes no other list. | Test (TC-1742) |

## Dependencies

- [FR-110](FR-110-resolve-header-profile-selections-at-e3.md): the layer table.
- [FR-111](FR-111-link-a-complete-v1-definition-bundle.md): the capability inventory.
- [FR-288](FR-288-build-the-registry-from-provider-manifests.md): the registry.
- FSpec FR-560: the document form and the `version` verb that writes it.
