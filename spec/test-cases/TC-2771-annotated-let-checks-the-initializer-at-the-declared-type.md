---
id: TC-2771
title: "An annotated let checks its initializer at the declared type"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-980
    type: verifies
---
# TC-2771: An annotated let checks its initializer at the declared type

## Description

An annotated let checks its initializer at the declared type.

Scope: FR-980-AC-3, FR-980-AC-4.

## Test Procedure

Check the bodies of FR-980-AC-3 and FR-980-AC-4 under `CheckMode::Linked`. Test: `fsl-semantics/tests/it/annotated_let.rs`.

## Expected Results

- AC-1's function and the annotated `p * q` check; `y: Int[0,3]` and the unannotated `p * q` refuse `undefined_expression`/`unproved-range`.
- `ill_typed`/`type-mismatch` at `1`; the `none` binding checks; an unresolved type name at `Undeclared`.
