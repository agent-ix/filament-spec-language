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

1. Encode FR-286-AC-2's `check` refusal and read the `ill_typed`
   diagnostic.
2. Run `fsl run nosuch.json` through the command library and read the
   `io-error` diagnostic.
3. Encode FR-286-AC-23's export output failure, and the refusal of a
   request document that is not JSON.
4. Project FR-286-AC-15's blank-label source diagnostic through
   `from_source_diagnostic`, and through a compile, a parse and a format
   refusal.

## Expected Results

- Step 1: `hint` equals `hint_for(Code::IllTyped, Some("type-mismatch"))`,
  is nonempty and ends in `.`.
- Step 2: `hint` equals `hint_for(Code::IoError, None)`.
- Step 3: neither the `output-failure` diagnostic nor the cause-less
  `invalid-request` diagnostic has a `hint` member.
- Step 4: none of the four diagnostics has a `hint` member.

Test: `fsl-command/tests/it/outcome_schema.rs::a_diagnostic_carries_the_catalog_hint_of_its_pair`.
