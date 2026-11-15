---
id: FR-095
title: "S-4: occurrence keys, source regions, the package source map and the diagnostic locus"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-005
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-087
    type: traces_to
---
# FR-095: S-4: occurrence keys, source regions, the package source map and the diagnostic locus

## Description

ADR-013 §7 slice **S-4** has no owning FR before this one.
It carries O-07 (source occurrence identity), O-12 (source locations and
provenance: the occurrence-key-keyed package source map) and T-5 (the
foundation diagnostic `Locus`). Its gate is S-3 (FR-087, FR-088), landed as
PR #397.

The types are layer F's (ADR-011 §6.1): `fsl_foundation::source::provenance`
holds the occurrence key, `RawSourceRef`, `SourceRegion` and
`PackageSourceMap`, and `fsl_foundation::diagnostic` holds `Locus`. The
layer-4 I2 reader builds the package source map from a verified
`filament.checked-package/v2` wire's `source_map` (ADR-013 C-14).

## Inputs

- ADR-013 O-07, O-12, T-5, C-14; R-10 (a wire node id never becomes a
  `NodeKey` by conversion).
- FSpec FR-322 `source_map` and the `RawSourceRef`,
  `SourceRegion` and `SourceMapEntry` schema definitions
  (`ix://agent-ix/filament-specification/FR-322`).
- The kernel `filament_exact::{Origin, Location}` (ADR-013 S-1).
- The `filament.checked-package/v2` wire a caller hands the I2 reader.

## Outputs

- `OccurrenceKey`: (wire node id, role, ordinal).
- FSL's `RawSourceRef` (authority, identity)
  and
  `SourceRegion` (`RawSourceRef`, byte start, byte end), with the value rules
  below.
- `PackageSourceMap`: occurrence key → ordered, non-empty regions, carried
  by the I2 reader's verified outcome.
- `Locus`: `Region(SourceRegion)`, `Occurrence(Location)` or
  `Artifact{identity, pointer}`, with an RFC 6901 `JsonPointer`.

## Behavior

### FSL source-mapping types conform to the checked-package contract

FSL's `OccurrenceKey`, `RawSourceRef`, `SourceRegion` and `PackageSourceMap`
SHALL conform to the source-map contract and schemas in FSpec FR-322. FR-322
owns the wire members, occurrence-key shape and half-open interval. FSL owns
the following provenance value rules and their conformance checks, which
FR-322 does not define:

- `RawSourceRef` SHALL carry a non-empty authority and identity, and no
  revision.
- `SourceRegion` SHALL be a half-open byte interval `[start, end)` with
  `start <= end`; an empty region is a point.
- Two `SourceRegion` values SHALL be equal exactly when their authority, identity,
  start and end are equal.

### The package source map is the wire's, keyed by occurrence

When the I2 reader verifies a `filament.checked-package/v2` wire, its outcome
SHALL carry FSL's package source map built from the wire's `source_map`.
The reader SHALL preserve the entry-to-region association and wire order
defined by FSpec FR-322, and SHALL refuse as `invalid_source_map` when the
entry cannot be represented by FSL's provenance types or refers to no
semantic graph node. A node's occurrences SHALL be exactly the entries that
name it.

### A location tag resolves through the map or refuses

A kernel `Location` SHALL resolve through the package source map by its node
id and occurrence, in the emission direction (node key → wire node id). A
tag whose node the package does not map SHALL refuse with an unknown-node
cause, and a tag of a mapped node at an unmapped role and ordinal with an
unknown-occurrence cause.

### A locus names a region, an occurrence or an artifact position

A `Locus::Region` SHALL denote its own region, a `Locus::Occurrence` the
regions its location resolves to through the package source map, and a
`Locus::Artifact` no source region. An artifact locus's pointer SHALL be an
RFC 6901 JSON pointer.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-095-AC-1 | FSL's occurrence-key, raw-source-reference and source-region types satisfy these rules: keys differing only in node id, role or ordinal are unequal; an empty authority or empty identity is refused with its own cause; and a region with start 5 and end 4 is refused while start 5 and end 5 is admitted. FSL's source-map production also gives every node of the semantic graph of a checked package holding a one-parameter function and a caller that calls it inside a conditional at least one source occurrence. | Test (TC-420) |
| FR-095-AC-2 | FSL's source-region equality and ordering are: a region over authority `a`, identity `i`, start 4, end 9 equals and orders equal to a region with the same four members, and is unequal to regions that change only the authority, only the identity, only the start or only the end. | Test (TC-420) |
| FR-095-AC-3 | A verified read of a wire holding two declarations, one of whose entries has two regions, carries a package source map conforming to FSpec FR-322: each entry's key maps to exactly its wire regions in wire order, each with the wire's authority and identity, and each node's occurrences are exactly its one entry. The map refuses an entry with no region and a key mapped twice. An entry with a reversed region or a non-hex node id refuses as `invalid_source_map`. Over FSpec's published positive `filament.checked-package/v2` fixtures, every `source_map` entry of each IR-admitted fixture looks up to exactly its wire regions and each node's occurrence count equals its entry count. | Test (TC-421) |
| FR-095-AC-4 | A location tag of a mapped occurrence resolves to its regions; a tag naming a node the map does not hold refuses with the unknown-node cause, and one naming a mapped node at an unmapped ordinal with the unknown-occurrence cause. A wire whose `source_map` entry names a node absent from its semantic graph refuses at the read as `invalid_source_map`. | Test (TC-421) |
| FR-095-AC-5 | A region locus denotes exactly its region; an occurrence locus denotes the two regions its mapped key holds, and an unmapped one refuses with the map's unknown-occurrence cause; an artifact locus refuses as naming no source region. | Test (TC-422) |
| FR-095-AC-6 | `""`, `/`, `/source_map/0/regions` and `/a~1b/c~0d` parse as JSON pointers and keep their text; `source_map` refuses for a missing leading `/`, and `/a~2b` and `/a~` refuse for the `~` at byte 2. | Test (TC-422) |

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §3 O-07, O-12, T-5; §4 C-14, C-21; §7 (the S-4 slice row).
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 (layer F depends on K; the diagnostic locus is foundation).
- [FR-087](FR-087-typestate-and-cross-package-node-key.md): the I2 reader
  and `WireNodeId` this requirement's package source map is read through.
- [US-005](../usecase/US-005-trust-checked-identity-across-packaging.md).
- FSpec FR-322 (`ix://agent-ix/filament-specification/FR-322`).
