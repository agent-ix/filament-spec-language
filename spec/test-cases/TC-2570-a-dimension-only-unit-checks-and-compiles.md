---
id: TC-2570
title: "A dimension-only unit checks and compiles"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-910
    type: verifies
---
# TC-2570: A dimension-only unit checks and compiles

## Description

A dimension-only unit checks and compiles.

Scope: FR-910-AC-1.

## Test Procedure

Under the header `language "ix:native" edition "1-draft";` and `profile v = "filament.value.complete/v1";`, run S3 check and then compile through the spine over a unit whose only declaration is `dimension Length;`, and over a unit of `dimension Length;` and `unit m : Length = rational(1, 1);`. Read each package's bytes with Contract IR's strict reader.

Tag the tests `#[trace("TC-2570", "FR-910-AC-1", "FSpec-TC-1810", "FSpec-FR-746-AC-1")]`.

## Expected Results

- Both units pass check and compile.
- The strict reader admits both packages.
- The first graph holds exactly one source `scalar_type`/`dimension` node.
