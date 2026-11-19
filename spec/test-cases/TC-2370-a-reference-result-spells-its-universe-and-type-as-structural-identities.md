---
id: TC-2370
title: "A Reference result spells its universe and type as structural identities"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-2370: A Reference result spells its universe and type as structural identities

## Description

A Reference result spells its universe and type as structural identities.

Scope: FR-650-AC-9.

## Test Procedure

Call `same` over the healthy current snapshot with the population reference to `child`, pass the completed value back, then call it with a hex-string universe and with an extra member in `type`. Test: `tests/it/run_value_spellings.rs` `a_reference_result_spells_structural_identities`.

Tag the tests `#[trace("TC-2370", "FR-650-AC-9")]`.

## Expected Results

- `universe` is a `{package, path, ordinal}` object and `type` an `{owner_effective_type, original}` object; `object_identity` is `child`; the passed-back value completes identical.
- The hex-string universe refuses `wrong-value-kind` at `/reference/universe`; the extra member refuses `unknown-member` at `/reference/type/x`; each exit 20.
