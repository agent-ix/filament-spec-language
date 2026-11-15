---
id: FR-600
title: "Keep the parser grammar in agreement with the standard grammar"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-002
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-339
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-760
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-503
    type: depends_on
---
# FR-600: Keep the parser grammar in agreement with the standard grammar

## Description

When the local qualification checks run, FSL SHALL compare the production
inventory of its parser (`fsl_cst::Production::all()`) with the production
inventory of the standard's shared grammar and fail when a production of
either side has no counterpart and no recorded reason.

## Use case

A maintainer adds `delivery [a, b]` to the parser and forgets the standard, or
the standard makes `clock` optional and the parser keeps it required. Today
nothing notices: the parser is a hand transcription of the standard and the two
drift silently. With this check the change fails a test that names the
production, and the maintainer either adds the counterpart or records why there
is none.

## Inputs

- The bytes of `proposals/filament-v1/shared-grammar.md`, read through
  `filament_specification::published_file` of the standard's pinned Cargo
  package (FR-760); no environment variable, sibling checkout or other
  filesystem path locates the standard.
- `fsl_cst::Production::all()`, the parser's closed production inventory.
- The correspondence table and the two allow-lists held in the test itself.

## Outputs

A passing test, or a failure that lists each unmatched production with its
side.

## Behavior

- The standard inventory is every production name defined in a fenced `ebnf`
  block of the shared grammar, by a line `name =` or `name =/` (FSpec FR-503).
  The test reads the document through the pinned crate and holds no copy of it.
- The parser inventory is the `Debug` spelling of each `Production` variant.
- The **correspondence table** maps a standard production name to the parser
  production that recognizes it (`composed-unit` and `complete-unit` to
  `CompleteUnit`, `type-ref` to `TypeReference`, `expr` to `Expression`, and
  so on). A name the table does not list matches the variant whose spelling is
  the name in PascalCase (`delivery-interval` to `DeliveryInterval`).
- The **standard-only allow-list** names each standard production with no
  parser production, and the **parser-only allow-list** names each parser
  production with no standard production. Every entry carries a one-line
  reason: an inlined alternative (the parser writes the alternatives in place),
  a lexical terminal, a parser-internal helper with no source meaning, or an
  extension-only production.
- The test fails when a standard name has neither a match nor an allow-list
  entry, when a parser production has neither a match nor an allow-list entry,
  and when a table or allow-list entry names a production that no longer
  exists on its side, so a stale entry cannot hide a later drift.
- The check runs in `make ci` in a clean clone, because the standard arrives
  through Cargo. If the pinned package does not publish the shared grammar
  (`published_file` returns `None`), the test fails naming the path rather
  than passing vacuously.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-600-AC-1 | Over the shared grammar read through the pinned `filament_specification` crate, every production name defined in a shared-grammar `ebnf` block matches a `Production` variant through the correspondence table or PascalCase spelling, or is on the standard-only allow-list with a reason; every `Production::all()` variant is matched by some standard name or is on the parser-only allow-list with a reason. | Test (TC-1500) |
| FR-600-AC-2 | Adding the name `invented-form` to the standard inventory, or removing `DeliveryInterval` from the parser inventory, makes the comparison fail naming that production and its side; a correspondence or allow-list entry naming a production absent from its side fails naming the entry. | Test (TC-1500) |
| FR-600-AC-3 | `filament_specification::published_file("proposals/filament-v1/shared-grammar.md")` returns the grammar text holding its `ebnf` blocks and `published_file` of a path the standard does not publish returns `None`; no file of `xtask` reads `FSPEC_DIR` or names a sibling `filament-specification` path. | Test (TC-1500) |

## Dependencies

- [FR-002](FR-002-parse-native-units.md) owns the parser and its production inventory.
- FSpec FR-339 incorporates the shared grammar; FSpec FR-503 makes its production inventory readable.
