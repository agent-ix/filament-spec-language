---
id: TC-2865
title: "An export request applies its fidelity floor and limits"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: verifies
---
# TC-2865: An export request applies its fidelity floor and limits

## Description

Verify the fidelity floor and the export output limit. Scope: FR-1015-AC-1 and AC-2.

## Test Procedure

1. Export `SlowEnough` under the default floor, then under `any`.
2. Export with `limits.output_bytes` 1.

## Expected Results

- Step 1: `refused` with `below-fidelity-floor`, exit 21; then `assembled`.
- Step 2: `incomplete` naming `export.output_bytes`, exit 22.
