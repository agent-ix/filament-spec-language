---
id: ADR-037
title: "One declared type for each shared wire format"
type: ADR
relationships:
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-032
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-352
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-413
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-412
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-418
    type: depends_on
  - target: ix://agent-ix/filament-contract-ir/FR-070
    type: depends_on
---
# ADR-037: One declared type for each shared wire format

## Status

Proposed, 2026-11-25. Amends the format-ownership consequences of ADR-013 and
ADR-032 for the formats listed here; it does not change their stage boundaries
or the normative wire schemas owned by FSpec.

## Context

Writers and strict readers for the same result and certificate formats have
separate hand-written representations. Several shared values are also copied
between writers and readers. This permits the producer and checker to agree on
a shape that differs from the owner schema, and permits identity, ordering and
derived fields to drift.

Before selecting a canonical definition, the design review searched FSL for
`State`, `TransitionIdentity`, `ProbabilisticBound`, `native-run-result/2`,
`WireDocument`, `*Read`, and the member spellings `key`, `node`, `package`,
`declaration`, `occurrence`, `kind`, `bound`, `value`, `lower`, `upper`,
`settlement`, and `witness`. The current producers and strict readers remain
separate in several cases. In particular, the probability-document reader has
`WireDocument` and `*Read` structures that do not serve as the writer's type;
the package writer and reader separately express FSpec-owned leaf values; and
native-run-result/2 is emitted and read through separate representations.
The existing types therefore do not provide one definition for each of those
interfaces.

The format owner is the requirement that defines the wire contract. FSpec owns
cross-repository formats and shared values. FSL implements those contracts;
Contract IR's `filament-contract-model` supplies one shared Rust definition for
FSpec-owned typed wire values used by multiple repositories. FSL-only formats
have one Rust definition at their owning FSL crate boundary. ADR-013 continues
to own stage-local types and conversions; this decision concerns only values
written by one component and read by another.

## Decision

### Shared definition

For every format in scope, its writer SHALL construct the declared type and
its strict reader SHALL deserialize that same type. The writer and reader
SHALL NOT maintain separate mirror structs, hand-built object maps, or member
walkers that independently encode the format's shape. A type that crosses
repository boundaries SHALL live in the shared model crate owned by that
format's contract, and every writer and reader SHALL import that definition.
The wire schema remains normative; Rust types do not replace or generate it.

The canonical definition SHALL include the complete accepted member shape.
Schema-specific newtypes and closed variants SHALL encode restrictions that
the schema and owner requirements impose. A strict read SHALL reject unknown
members and malformed member values using the format owner's existing
refusal vocabulary. Canonical bytes SHALL continue to use the canonical
encoder required by ADR-013. Bounded and metered byte admission occurs before
deserialization, so using the shared type does not bypass reader limits.

Every derived field, including ordinals, identity segments, and member sets,
SHALL be computed by one function called by both the writer and reader. The
reader SHALL validate the derived value against the admitted document before
returning the typed value. No format member is maintained in a separate
hand-written table when the owner schema or shared type already defines it.

### Format owners and implementation homes

| Format or shared value | Normative owner | Shared Rust definition | FSL requirement links |
| --- | --- | --- | --- |
| State locus `{key}` | FSpec FR-331 | `filament-contract-model` | FR-145, FR-149, FR-169, FR-244, FR-245 |
| `TransitionIdentity` and its typed value | FSpec FR-181; schema form in FR-331 | `filament-contract-model` | FR-120, FR-331, FR-128, FR-237, and the task-product certificate requirement |
| `ProbabilisticBound` | FSpec FR-413 | `filament-contract-model` | FSL requirements that write or strictly read the probability certificate, including FR-201 |
| Certificate identity `CertificateNodeId` and `CertificatePathSegment` | FSpec FR-412-AC-34 | `filament-contract-model` | FR-201 |
| `native-run-result/2` | FSpec FR-352, with witness members from FR-351 | FSL `fsl-replay` wire module | FR-267 and every FSL requirement that writes or reads the document |

For each listed format, a requirement whose implementation writes, strictly
reads, or checks it SHALL declare a `depends_on` relationship to its owner in
frontmatter. The requirement that defines the format has no sideways
relationship to a reader. FSL requirements that implement or check a
cross-repository value also link to the shared model requirement that owns
that value. The links for formats outside this table are added when their
migration step begins; this ADR does not claim their type consolidation is
complete.

For `native-run-result/2`, the one FSL type SHALL cover the extraction record
and the complete document accepted by FSpec FR-352. It SHALL be used by the
production writer and the strict reader. Ad hoc command output and `*Read`
mirror structs do not define alternate forms of that document. FR-267 remains
the FSL implementation requirement and refers to the FSpec schema for the
wire shape.

### Acceptance net

Each shared type SHALL have a schema-conformance test that exercises the real
writer, validates the emitted value against the owner's schema, strictly
reads it through the same type, and checks typed equality and canonical-byte
equality. Where the owner publishes positive vectors, the test SHALL strictly
read every such vector and write it back with the same canonical bytes. Where
the owner publishes no positive vectors, the test SHALL use explicit valid
values from the owner's acceptance criteria and mutations of those values
derived from the owner's schema and refusal criteria; it SHALL not invent a
parallel wire form or claim a vector round trip. The test SHALL not replace a
production writer or reader with a stand-in before schema validation. These
tests provide the acceptance net for each migration; they do not replace the
schema or the implementation tags required by ADR-032.

## Consequences

Writer and reader shape changes now occur at the shared type, and schema
changes are detected by the same conformance tests that exercise the actual
writer and reader. A change to a cross-repository format is made in its owner
schema and the shared model type; consumers adopt that definition rather than
reconstructing it. Bounded read behavior and typed refusal codes remain at the
existing admission boundary.

This decision does not remove identity or semantic disagreements that are
outside the member shape, such as a different node preimage. Those require
their own owner rule and one derivation function, as ADR-013 already requires.

## Alternatives Considered

- **Keep separate writers and strict readers with round-trip tests only.**
  Retained as the per-format acceptance net, but rejected as the definition:
  test fixtures cannot prevent a second reader or writer from acquiring a
  different shape.
- **Generate Rust types from the FSpec schema or generate schemas from Rust.**
  Rejected. Generation would either make Rust normative for FSpec-owned
  formats or would not express canonical bytes, bounded reads, refusal codes,
  and cross-member checks. Tests against the normative FSpec schema provide
  drift protection without changing ownership.
- **Keep a list of canonical type names in a gate.** Rejected because it is a
  second definition that can drift. The type and its owner requirement remain
  the source of the implementation definition.
