---
id: TC-2373
title: "Model-declared variant and record names are their ix identities"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-2373: Model-declared variant and record names are their ix identities

## Description

Model-declared variant and record names are their ix identities.

Scope: FR-650-AC-12.

## Test Procedure

Extend the ConfigVersion model with `Tier` and `Stamp`; call `id_tier` and `id_stamp` with their `ix://` names, then `id_tier` with `Tier` and with the case identity. Test: `tests/it/run_value_spellings.rs` `model_declared_names_are_ix_identities`.

Tag the tests `#[trace("TC-2373", "FR-650-AC-12")]`.

## Expected Results

- Both calls complete with their argument unchanged.
- `Tier` refuses `wrong-value-kind` at `/variant/type`; `variant/Tier-Gold` refuses `invalid-value` at `/variant/case`.
