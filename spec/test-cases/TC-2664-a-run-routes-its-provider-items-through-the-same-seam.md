---
id: TC-2664
title: "A run routes its provider items through the same provider seam"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-950
    type: verifies
---
# TC-2664: A run routes its provider items through the same provider seam

## Description

Verify that `fsl_command::run_with_providers` settles a run call's provider
items through `provider::settle_items`, with the request built as for
analyze, and returns the run outcome with the provider verdict. Scope:
FR-950-AC-5.

## Test Procedure

1. Call `run_with_providers` over FR-950-AC-5's run request with the recording provider and `deadline_ms` 250.
2. Call `fsl_command::command::run` over the same request with no providers.

Test: `fsl-analyze/tests/it/provider_request_seam.rs`
`a_run_routes_its_provider_items_through_the_same_seam`.

## Expected Results

- Step 1: one `settle` call with a complete `ProviderRequest` whose `request.deadline_ms` is 250; the outcome holds the call's result and the item `proved` naming the provider.
- Step 2: the provider is never called.
