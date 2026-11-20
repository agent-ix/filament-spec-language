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
A bare id binds the artifact of the repository holding the test only.
Scope: NFR-140-AC-1 to NFR-140-AC-4.

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
4. Over a two-repository corpus whose FSL and filament-specification members
   both mint `FR-001-AC-1`, and only filament-specification mints
   `FR-001-AC-3`, tag an FSL test and a filament-specification test bare
   `FR-001-AC-1` and another FSL test bare `FR-001-AC-3`; build the matrix and
   read both `FR-001-AC-1` rows, the `FR-001-AC-3` row and `untracked_symbols`.
   Then read every filament-specification row of FSL's own two-repository
   matrix (step 1) for FSL binders tagged with a bare id.

## Expected Results

- Step 1: each filament-specification row lists the FSL tests carrying its
  `FSpec-` tag; FSL's `FR-149-AC-22` row lists none of them.
- Step 2: the qualified tag binds the same row; over FSL alone the tag is a
  reference outside the corpus and binds no FSL row; the unminted id is
  reported as unknown in filament-specification.
- Step 3: the declaration names `FSpec-` and
  `agent-ix/filament-specification`; without it every `FSpec-` tag is
  reported in `untracked_symbols`, never as an unresolved qualified id, and
  binds no row.
- Step 4: each bare `FR-001-AC-1` tag binds only its own repository's row; the
  FSL `FR-001-AC-3` tag binds no row and is in `untracked_symbols`; no
  filament-specification row of FSL's matrix lists an FSL test by a bare id.
