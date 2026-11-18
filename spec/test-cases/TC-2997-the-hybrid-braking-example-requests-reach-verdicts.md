---
id: TC-2997
title: "The hybrid-braking example requests reach verdicts"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1019
    type: verifies
---
# TC-2997: The hybrid-braking example requests reach verdicts

## Description

Verify that the `examples/hybrid-braking/` request documents reach a verdict through the reference provider with no external provider configured. Scope: FR-1019-AC-13. The test lives in `fsl-analyze/tests/it/hybrid_reference.rs`, tagged `#[trace("TC-2997", "FR-1019-AC-13")]`.

## Test Procedure

1. Read `examples/hybrid-braking/slow-enough.analyze.json` through `fsl_command::command::analyze`.
2. Read `examples/hybrid-braking/too-slow.analyze.json` the same way.

## Expected Results

- Step 1: one item, `proved`, `certified`, backend `fsl-hybrid-reference`.
- Step 2: one item, `refuted`, backend `fsl-hybrid-reference`, with a reproduced `replay`.
