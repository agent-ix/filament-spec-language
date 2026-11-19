---
id: FR-913
title: "Emit the same package from formatted source and read it back with the same verdict"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-005
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-003
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
---
# FR-913: Emit the same package from formatted source and read it back with the same verdict

## Description

When a unit that compiles is formatted, FSL SHALL compile the formatted source
to a package whose semantic graph equals the original package's in every
member except source regions and that Contract IR's strict reader gives the
same verdict as the original.

## Rationale

FR-003 keeps the checked package identity across formatting. A consumer reads
the emitted bytes, not the in-process checked package, so the agreement must
also hold on the wire and through the reader a consumer uses. A formatter or
emitter change that altered a node, a term or a reader verdict while keeping
the identity would otherwise go unseen.

## Inputs

Each complete-V1 fixture unit of the repository's tests that compiles, and its
formatted source (FR-003).

## Outputs

Two emitted packages and two strict-reader verdicts per unit.

## Behavior

1. The unit and its formatted source are compiled under one source identity,
   one package name and one display path.
2. The two packages' semantic graphs are equal node for node, with equal
   `node_id`s, bodies, declarations, owners, dependency edges and occurrence
   roles and ordinals. They differ at most in source regions, which record
   byte positions that formatting moves.
3. Contract IR's strict reader admits both packages or refuses both with the
   same code, cause and document pointer.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-913-AC-1 | For `tests/fixtures/spine-compile.native` and for each other complete-V1 fixture unit of the repository's tests that compiles, the package compiled from its formatted source has a semantic graph equal to the original's in every member except source regions, and `filament_contract_model::read_checked_package` admits both packages, or refuses both with the same code. | Test (TC-2582) |

## Dependencies

- [FR-003](FR-003-format-native-source.md) owns formatting and the identity
  rule this requirement carries to the wire.
- FSpec FR-322 owns the wire and its reader.
