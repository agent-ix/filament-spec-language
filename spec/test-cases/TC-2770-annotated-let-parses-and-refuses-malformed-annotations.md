---
id: TC-2770
title: "An annotated let parses and malformed annotations refuse at their token"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-980
    type: verifies
---
# TC-2770: An annotated let parses and malformed annotations refuse at their token

## Description

An annotated let parses and malformed annotations refuse at their token.

Scope: FR-980-AC-1, FR-980-AC-2.

## Test Procedure

Parse and build the bodies of FR-980-AC-1 and parse the three malformed lets of FR-980-AC-2. Tests: `fsl-cst/tests/it/annotated_let.rs` `an_annotated_let_parses_with_its_type_on_the_binder`, `a_malformed_let_annotation_refuses_at_the_offending_token`.

## Expected Results

- The annotated `let` holds one `TypeReference` `Int[0,3]`; S2 builds a `Let` with the annotation, and without it a `Let` with none.
- `invalid_syntax`/`unexpected-token` at `=`, at `1` and at `Int`.
