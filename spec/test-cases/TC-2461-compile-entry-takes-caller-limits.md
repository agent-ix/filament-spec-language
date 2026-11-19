---
id: TC-2461
title: "The compile entry takes the caller's intake and stage limits"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-869
    type: verifies
---
# TC-2461: The compile entry takes the caller's intake and stage limits

## Description

Verify `compile_outcome_with_limits_and_overrides`. Scope: FR-869-AC-1, FR-869-AC-2.

## Test Procedure

1. Compile a request file one byte over 1 MiB with the default limits and with `request_bytes` at its size; repeat FR-027-AC-13's `dependent_bytes` case.
2. Compile a 6,000-deep parenthesis nest under the default limits and with `RequestOverrides::limits` raising `s1.nodes`; set `emit.output_bytes` one below the package length.

## Expected Results

- Step 1: the refusals name the limit, its maximum and `--request-bytes` or `--dependent-bytes`; the raised ceilings compile.
- Step 2: `stage_limit_exceeded` naming `s1.nodes`, then success; the emit refusal names `emit.output_bytes`.

Test: `tests/it/verb_limits.rs`.
