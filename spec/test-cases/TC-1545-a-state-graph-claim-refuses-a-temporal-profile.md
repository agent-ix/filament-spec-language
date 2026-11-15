---
id: TC-1545
title: "A state-graph claim refuses a temporal profile"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-165
    type: verifies
---
# TC-1545: A state-graph claim refuses a temporal profile

## Description

Verify that a state-graph claim's `using` alias must name a state-bearing layer.

Scope: FR-165-AC-7.

## Test Procedure

1. Check `possible Bad using inf over (c: Config::ConfigVersion) { c.versionNumber = 2 }` with `inf` the infinite-trace profile.
2. Check the same claim `using v` (`filament.value.complete/v1`).
3. Check `always possible` and `unique path` claims `using inf`.

Tag the tests `#[trace("TC-1545", "FR-165-AC-7")]`.

## Expected Results

- Steps 1 and 3: `unknown_profile`/`wrong-selection-role` at `inf`, retaining alias, identity and required kind; no checked claim.
- Step 2: checks to `Possible`.
