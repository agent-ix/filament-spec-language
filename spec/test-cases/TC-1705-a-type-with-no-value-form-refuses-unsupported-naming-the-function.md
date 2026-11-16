---
id: TC-1705
title: "A type with no value form refuses unsupported naming the function"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-1705: A type with no value form refuses unsupported naming the function

## Description

A type with no value form refuses unsupported naming the function.

Scope: FR-650-AC-6.

## Test Procedure

Run a unit whose function declares a collection over a population as its result, and one as a parameter; run functions over `Quantity<s>`, `Map<String, Int[0, 9]>[0, 2]`, a Reference and a top-level population.

Tag the tests `#[trace("TC-1705", "FR-650-AC-6")]`; they live in `tests/it/run_values.rs`.

## Expected Results

- Each collection-over-population unit refuses at S3 `ill_typed`/`operator-ineligible`, exit 20, and reaches no call.
- No Quantity, Map, Reference or population function refuses `unknown_required_feature`.
