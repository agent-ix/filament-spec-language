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
inventory of the standard's shared grammar and its explicitly referenced
normative FR-local EBNF, and fail when a production of either side has no
counterpart and no recorded reason.

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
- The bytes of each normative FR-local EBNF document named by a local-grammar
  correspondence, read through `filament_specification::published_file` of the
  same pinned Cargo package. For the refinement declaration and rows this is
  `spec/functional/analysis/FR-375-declare-model-refinement.md` (FSpec FR-375).
- `fsl_cst::Production::all()`, the parser's closed production inventory.
- The shared-grammar and FR-local correspondence tables, plus the two
  allow-lists held in the test itself.

## Outputs

A passing test, or a failure that lists each unmatched production with its
side.

## Behavior

- The standard inventory is every production name defined in a fenced `ebnf`
  block of the shared grammar, by a line `name =` or `name =/` (FSpec FR-503),
  together with the names defined in each explicitly referenced normative
  FR-local EBNF block. The test reads every source document through the pinned
  crate and holds no copy of it. A local syntax production is not parser-only
  merely because it is outside the shared-grammar document.
- The parser inventory is the `Debug` spelling of each `Production` variant.
- A **correspondence table** maps a standard production name and its source to
  the parser production that recognizes it (`composed-unit` and `complete-unit`
  to `CompleteUnit`, `type-ref` to `TypeReference`, `expr` to `Expression`, and
  so on). A name the table does not list matches the variant whose spelling is
  the name in PascalCase (`delivery-interval` to `DeliveryInterval`). A
  production defined in both the shared grammar and a normative FR-local block
  is one standard production with both sources recorded; it is not a duplicate
  parser production.
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
| FR-600-AC-4 | The pinned standard's shared grammar plus FSpec FR-375's normative `ebnf` block are read through `filament_specification::published_file` and yield one combined production inventory. The FR-375 productions `refinement`, `population-row`, `object-row`, `field-row`, `history-row`, `history-update`, `history-literal` and `fairness-row` each map to their corresponding parser `Production`. `step-argument` maps to the parser production from whichever of the shared grammar or FR-375 defines it in the pinned standard; if both define it, both sources identify the same one standard production. These productions are not parser-only allow-list entries. An absent source document, missing correspondence or stale correspondence fails naming its source and production. | Test (TC-1500) |

## Dependencies

- [FR-002](FR-002-parse-native-units.md) owns the parser and its production inventory.
- FSpec FR-339 incorporates the shared grammar; FSpec FR-503 makes its production inventory readable.
- FSpec [FR-375](ix://agent-ix/filament-specification/FR-375) defines the normative FR-local EBNF productions for refinement declarations and rows.
