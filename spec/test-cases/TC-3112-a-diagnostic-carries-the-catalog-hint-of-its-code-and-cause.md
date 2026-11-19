---
id: TC-3112
title: "A diagnostic carries the catalog hint of its code and cause"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3112: A diagnostic carries the catalog hint of its code and cause

## Description

Verify that the outcome serialization writes `hint` from the hint catalog
exactly when the diagnostic's code and cause have a row. Scope:
FR-286-AC-28.

## Test Procedure

1. Serialize FR-286-AC-2's `check` refusal and read the `ill_typed`
   diagnostic.
2. Serialize FR-286-AC-23's export output failure.
3. Project FR-286-AC-15's blank-label source diagnostic through
   `from_source_diagnostic`, and through a compile, a parse and a format
   refusal.

## Expected Results

- Step 1: `hint` equals `hint_for(Code::IllTyped, Some("type-mismatch"))`,
  is nonempty and ends in `.`.
- Step 2: the `output-failure` diagnostic has no `hint` member.
- Step 3: the four diagnostics carry the same `hint`, or all four none,
  as the pair's catalog row gives.

Test: `fsl-replay/tests/it/outcome_schema.rs::a_diagnostic_carries_the_catalog_hint_of_its_pair`.
