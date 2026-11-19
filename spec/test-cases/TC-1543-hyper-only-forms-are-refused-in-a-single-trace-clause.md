---
id: TC-1543
title: "Hyper-only forms are refused in a single-trace clause"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-612
    type: verifies
---
# TC-1543: Hyper-only forms are refused in a single-trace clause

## Description

Verify that indexed reads and step labels in a non-hyper temporal clause refuse with one named refusal in the formula and in the `on each ... when` guard.

Scope: FR-612-AC-1, FR-612-AC-2.

## Test Procedure

Fixture: the config-version model `V` with operations `step` and `mix`, a non-hyper clause over `v: V::ConfigVersion`.

1. Check formulas `always holds(v.versionNumber @ a = 0)` and `always holds(a.step.mix)`.
2. Check activation `on each (w: V::ConfigVersion) when (w.versionNumber @ a = 0)`.

Tag the tests `#[trace("TC-1543", "FR-612-AC-n")]`.

## Expected Results

- Step 1: `unsupported_construct`/`expression-form` at the read or label, message containing `hyper`.
- Step 2: the same refusal at `w.versionNumber @ a` and no S1 syntax error.
