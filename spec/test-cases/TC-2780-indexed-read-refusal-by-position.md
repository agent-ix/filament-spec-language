---
id: TC-2780
title: "An indexed read is refused by where it is written"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-984
    type: verifies
---
# TC-2780: An indexed read is refused by where it is written

## Description

An indexed read is refused by where it is written.

Scope: FR-984-AC-1, FR-984-AC-2, FR-984-AC-3.

## Test Procedure

Read the `check` and `parse` outcome documents of FR-984-AC-1 and FR-984-AC-2, and parse FR-984-AC-3's hyper clause. Test: `fsl-replay/tests/it/indexed_read_stage.rs`.

## Expected Results

- S3 `unsupported_construct`/`expression-form`, exit 21, no S1 diagnostic.
- S1 `invalid_syntax`/`unexpected-token` at `@`, exit 20.
- Two `IndexedRead` nodes, no diagnostic.
