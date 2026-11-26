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
Every writer of a transition operation identity SHALL call the same writer
function: it emits the identity text, except that a name identity is emitted
as the bare name. FR-120 owns this operation-identity rule.

### Format owners and implementation homes

| Format or shared value | Normative owner item(s) | Shared Rust definition |
| --- | --- | --- |
| State locus `{key}` | FSpec FR-331 | FSL wire module |
| `TransitionIdentity` and its typed value | FSpec FR-181; its wire schema is in FR-331 | FSL wire module; thin wrapper over `filament_semantic_value::ModelTransition` |
| `ProbabilisticBound` | FSpec FR-413 for probabilistic counterexamples and the statistical verdict | FSL wire module |
| Certificate identity path segment | FSpec FR-412-AC-34 | Existing `filament_exact::NodeKey` plus the `node_wire` codec; add no type |
| Certificate rejection locus and rule (`CertificateLocus`, `CertificateRule`) | FSpec FR-331 | `filament-contract-model` |
| Zone certificate | FSpec FR-418 | `filament-contract-model` |
| Task-product certificate | FSpec FR-418-AC-11..14; certificate envelope and rejection vocabulary in FSpec FR-331; transition identities use FSpec FR-181 | `filament-contract-model` |
| Hybrid certificate | FSpec FR-331 | `filament-contract-model` |
| Probability certificate document | FSpec FR-412; certificate-member vocabulary is FSpec FR-331 and Contract IR FR-070; FSL FR-201 implements the checker | FSL `fsl-replay` probability-certificate document module |
| `filament.checked-package/v2` node forms and protocol-location ordinals | FSpec FR-322 and FR-375; in particular FR-322-AC-291 and FR-375-AC-36 | `filament-contract-model::checked_package::v2` (implementation home) |
| Certificate-member vocabulary | FSpec FR-331; Contract IR FR-070 | `filament-contract-model` |
| `native-run-result/2` | FSpec FR-352 `Members`, `Outcome axes`, `Temporal disposition` and `Protocol-analysis result` sections, plus AC-1–26; FSpec FR-351 owns witness members; FSL FR-267 implements the FSL boundary | FSL `fsl-replay` wire module |
| `native-replay/1` request | FSpec FR-336; FSL FR-862 | FSL `fsl-command` replay-request module |
| `filament-outcome/1` | FSpec FR-300 format identity, FR-520 analyze document, FR-301 CLI contract and FR-331 terminal records; FSL FR-286 publishes the schema | FSL `fsl-replay::outcome` |

For each listed format, every requirement whose implementation writes,
strictly reads, or checks it SHALL declare a `depends_on` relationship to its
owner in frontmatter. The requirement that defines the format has no sideways
relationship to a reader. FSL requirements that implement or check a
cross-repository value also link to the shared model requirement that owns
that value. The owner and reader relationships are recorded only in each
requirement's frontmatter; this ADR does not keep a second list of those links.

A rule found only in a reader is a gap in the format owner's specification.
Before consolidating that reader and its writer, the reader-only rule SHALL be
added to the owner's acceptance criteria. A writer-to-reader relationship does
not replace that owner criterion.

The `cargo xtask canonical-types` gate SHALL apply its wire rule to flag new
hand copies outside the owning module: tagged transition or State-locus JSON
objects, literal `WireFormat` labels, and `Deserialize` structs with the same
member set as a canonical wire type. Rejecting a hand-maintained name list as
the source of truth does not remove this duplicate-definition gate.

For `native-run-result/2`, the one FSL type SHALL cover the extraction record
and the complete document accepted by FSpec FR-352. FR-352 defines the shape
in its `Members`, `Outcome axes`, `Temporal disposition` and
`Protocol-analysis result` sections and AC-1–26; it does not
publish a JSON Schema for this format. The type SHALL be
used by the production writer and the strict reader. Ad hoc command output and
`*Read` mirror structs do not define alternate forms of that document. FSL
FR-267 remains the implementation requirement; this ADR does not claim that
FR-267 refers to an FSpec JSON Schema.

The FSL one-type rule preserves the existing reader's observable behavior.
Bounded admission at the byte stage SHALL remain. Each shared type SHALL have
one hand `Deserialize` implementation in its own module that returns the
member pointer and kind; each format maps those to its own refusal codes, and
`validate()` performs checks in the owner's specified order. Reads SHALL keep
charging resource meters as members are consumed and enforce the
`NativeRunLimits` budgets at the same admission boundary. They SHALL keep
member pointers and the existing typed refusal codes, including
`TaskProductShapeReason`, the non-empty extraction segments required by
FR-267-AC-17, and the certificate loci required by FR-245. In the
Contract-IR checked-package body and details, input-depth nodes SHALL retain
their hand `Encode` boundary. Step 1 removes the legacy object-operation and
`ix://`-rebuild read forms; until that step lands, those forms remain accepted.
This ADR does not change their acceptance before step 1.

### Acceptance net

Each shared type SHALL have a conformance test that exercises the real writer,
validates the emitted value against the owner's normative schema where that
schema exists, strictly reads it through the same type, and checks typed
equality and canonical-byte equality. Where no schema exists, the test SHALL
check the owner's normative member tables and acceptance criteria instead.
Where the owner publishes positive vectors, the test SHALL strictly
read every such vector and write it back with the same canonical bytes. Where
the owner publishes no positive vectors, the test SHALL use explicit valid
values from the owner's acceptance criteria and mutations of those values
derived from the owner's schema and refusal criteria; it SHALL not invent a
parallel wire form or claim a vector round trip. The test SHALL not replace a
production writer or reader with a stand-in before schema validation. These
tests provide the acceptance net for each migration; they do not replace the
schema or the implementation tags required by ADR-032.

## Consequences

Writer and reader shape changes now occur at the shared type, and changes to
an owner's schema or acceptance criteria are detected by the same conformance
tests that exercise the actual writer and reader. A change to a
cross-repository format is made in its owner requirement and shared model
type; consumers adopt that definition rather than reconstructing it. Bounded
read behavior, incremental meter charging, `NativeRunLimits`, member pointers
and typed refusal codes remain at the existing admission boundary. Input-depth
Contract-IR body/details retain their hand `Encode`. Step 1 removes the legacy
object-operation and `ix://`-rebuild read forms.

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
- **Keep a list of canonical type names in a gate.** Rejected as the source of
  the implementation definition because it is a second definition that can
  drift. The `canonical-types` wire rule remains a gate and flags new hand
  copies of the canonical shapes; it does not own or replace those definitions.
