---
id: TC-2704
title: "Provider-routing entries share one provider options type"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-964
    type: verifies
---
# TC-2704: Provider-routing entries share one provider options type

## Description

Verify that analyze and run take the run's FR-462 limits as `ProviderOptions.provider_limits` and that `Engine::external` holds no limits. Scope: FR-964-AC-1, FR-964-AC-2.

## Test Procedure

1. Call `analyze_with_providers` and `run_with_providers` with a recording provider and `deadline_ms` 250, `max_frame_bytes` 1048576; then with `ProviderOptions::default()`.
2. Register two recording providers in one run.

Test: `fsl-command/tests/it/run_requirement_items.rs` `run_with_providers_settles_the_call_s_items_after_negotiation`.

## Expected Results

- Step 1: `deadline_ms` 250 and the frame limit 1048576 with the other defaults; then `deadline_ms` 600000.
- Step 2: both requests carry the run's limits.
