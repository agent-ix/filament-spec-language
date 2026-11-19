---
id: TC-2572
title: "An alias beside a node-bearing declaration compiles"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-910
    type: verifies
---
# TC-2572: An alias beside a node-bearing declaration compiles

## Description

An alias beside a node-bearing declaration compiles.

Scope: FR-910-AC-3.

## Test Procedure

Under the header `language "ix:native" edition "1-draft";` and `profile v = "filament.value.complete/v1";`, compile a unit of `type I = Integer;` and `dimension Length;`, and a unit of `type I = Integer;` and `function f(x: I): I pure { x }`.

Tag the tests `#[trace("TC-2572", "FR-910-AC-3", "FSpec-TC-1812", "FSpec-FR-746-AC-3")]`.

## Expected Results

- Both units compile.
- Neither package holds a node whose declaration is `I`.
