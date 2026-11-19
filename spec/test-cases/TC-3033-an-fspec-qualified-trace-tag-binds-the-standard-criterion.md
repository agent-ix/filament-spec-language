---
id: TC-3033
title: "An FSpec-qualified trace tag binds the standard criterion"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-140
    type: verifies
---
# TC-3033: An FSpec-qualified trace tag binds the standard criterion

## Description

Verify that the two-repository matrix binds an FSL test tagged `FSpec-<id>`
or `ix://agent-ix/filament-specification/<id>` to the filament-specification
artifact `<id>`, and never to the FSL artifact with the same bare id.
Scope: NFR-140-AC-1 to NFR-140-AC-3.

## Test Procedure

1. Run `quire matrix --format tsv --repo
   agent-ix/filament-specification=<fspec> --repo
   agent-ix/filament-spec-language=<fsl>` and read the rows of both
   repositories' `FR-149-AC-22`, `AC-25`, `AC-26`, `AC-28`, `AC-29` and
   filament-specification `TC-194`.
2. In a scratch copy, retag one test with the qualified form; run the matrix
   over both repositories, then over FSL alone; add a tag naming an id
   filament-specification does not mint.
3. Inspect FSL's traceability declaration; in a scratch copy remove the
   qualifier and rerun step 1.

## Expected Results

- Step 1: each filament-specification row lists the FSL tests carrying its
  `FSpec-` tag; FSL's `FR-149-AC-22` row lists none of them.
- Step 2: the qualified tag binds the same row; over FSL alone the tag is a
  reference outside the corpus and binds no FSL row; the unminted id is
  reported as unknown in filament-specification.
- Step 3: the declaration names `FSpec-` and
  `agent-ix/filament-specification`; without it every `FSpec-` tag is
  unresolved and binds no bare-id row.
