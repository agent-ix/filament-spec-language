---
id: TC-2785
title: "Null outside a record field is ill-typed at check"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-986
    type: verifies
---
# TC-2785: Null outside a record field is ill-typed at check

## Description

Verify that `null` outside a record field value builds at S2 and is refused
`ill_typed`/`type-mismatch` at check, and that the field initializer and the
empty-option spellings are unaffected.

Scope: FR-986-AC-1, FR-986-AC-2, FR-986-AC-3.

## Test Procedure

Build and check the units of FR-986-AC-1 to AC-3. Test:
`fsl-semantics/tests/it/null_value.rs`.

## Expected Results

- AC-1 and AC-2: `ill_typed`/`type-mismatch` at `null`, no
  `UnrepresentedConstruct`, message naming `null`, `none` and `absent`.
- AC-3: each unit checks.
