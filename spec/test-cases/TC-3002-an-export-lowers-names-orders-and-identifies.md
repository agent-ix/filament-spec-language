---
id: TC-3002
title: "An export lowers what the target states, resolves obligation names, orders its stops and names its generator"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: verifies
---
# TC-3002: An export lowers what the target states, resolves obligation names, orders its stops and names its generator

## Description

Verify the single unrepresentable cause form, the obligation-name rules, the order of floor and size stops, and the generator identity. Scope: FR-1015-AC-5 to AC-8.

## Test Procedure

1. Export the counter's `Bounded` under the default floor, and `SlowEnough` over `Braking` under `any`.
2. Export the counter with `obligations` `[]`, then with `["CanInc", "Bounded", "Nope"]`.
3. Export `SlowEnough` under the default floor with `export.output_bytes` 1.
4. Read the counter export's `package.generator`.

## Expected Results

- Step 1: `Bounded` is `preserved` with exit 0. `SlowEnough` has `target-unrepresentable` causes with `target` `tla`, a `node_tag`/`semantic_form` construct, a package node id and a region locus.
- Step 2: `invalid-value` at `/request/export/obligations` with exit 20. Then three records in order, two `refused` with `not-an-obligation`, exit 21.
- Step 3: `below-fidelity-floor`, exit 21.
- Step 4: `{"identity": "filament-spec-language", "version": VERSION}`.
