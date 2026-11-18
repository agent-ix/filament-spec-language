---
id: TC-2970
title: "A name in both claim name tables is refused"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-2970: A name in both claim name tables is refused

## Description

Verify that temporal clause names and claim names form one namespace on the
source and package routes. Scope: FR-940-AC-8.

## Test Procedure

1. Analyze, over source, the Counter unit with the temporal clause `Bounded` and `possible Bounded`.
2. Analyze the unit with `Bounded` and `possible ReachThree` over source and over its handoff package.
3. Rename the package's `ReachThree` `claim_names` row to `Bounded` and analyze the package request again.

Test: `fsl-analyze/tests/it/analyze_package_selection.rs`.

## Expected Results

- Step 1: `ambiguous_declaration`/`ambiguous-name`; no item analyzed.
- Step 2: both items settle with equal records on both routes.
- Step 3: `ambiguous_declaration`/`ambiguous-name`; no item settled.
