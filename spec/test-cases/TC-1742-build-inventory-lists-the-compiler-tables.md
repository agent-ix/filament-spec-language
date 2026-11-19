---
id: TC-1742
title: "Build inventory lists the compiler's own tables"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-661
    type: verifies
---
# TC-1742: Build inventory lists the compiler's own tables

## Description

Verify the build inventory against the tables the compiler consults. Scope: FR-661-AC-1, FR-661-AC-2, FR-661-AC-3.

## Test Procedure

1. Call `build_inventory` over an empty registry and compare `features` and `families` with `CapabilityId::complete_inventory()`.
2. For every listed profile, compile a header selecting it with its `requires` profiles.
3. Call twice; then call over a registry with one linked provider.

Tag the tests `#[trace("TC-1742", "<AC id>")]`.

## Expected Results

- Step 1: equal lists in order; `editions` holds `1-draft`; `consumers` is empty.
- Step 2: no `unknown_profile` refusal; no identity begins with `quire.`.
- Step 3: equal bytes; exactly one added `consumers` entry and no other change.
