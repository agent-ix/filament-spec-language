---
id: TC-2775
title: "A typed size query runs to the same length as size"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-981
    type: verifies
---
# TC-2775: A typed size query runs to the same length as size

## Description

A typed size query runs to the same length as size.

Scope: FR-981-AC-4.

## Test Procedure

Run FR-981-AC-4's function claim, its sequence bound by an annotated `let`, through `command::run` over the ConfigVersion spine; then run the claim with the literal as the bare `size` argument. Test: `tests/it/typed_size_run.rs`.

## Expected Results

- The claim holds, exit 0.
- The bare-literal claim refuses `ill_typed`/`ambiguous-literal` at the literal.
