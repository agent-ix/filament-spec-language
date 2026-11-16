---
id: TC-2294
title: "A misadvertising process provider is refused and no plugin joins the layering snapshot"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-489
    type: verifies
---
# TC-2294: A misadvertising process provider is refused and no plugin joins the layering snapshot

## Description

Verify that a process provider whose manifest does not advertise its claimed
kind and mode is refused at registration, and that a configured plugin never
enters the production layering snapshot. Scope: FR-635-AC-4.

## Test Procedure

1. Start the workspace test plugin with a `hello` manifest that advertises only (`value-validity`, `bounded`) and pass it to `analyze_with_providers`.
2. In the same process as FR-635-AC-1's run, build FR-489's production snapshot.

## Expected Results

- Step 1: the registration is refused naming the plugin's `BackendId`; the document's `registry` does not list it.
- Step 2: the snapshot holds exactly the `Engines::fsl()` descriptors and not the test plugin.

Test: `fsl-analyze/tests/it/analyze_reach_rulings.rs`
`a_misadvertising_plugin_is_refused_and_never_joins_the_layering_snapshot`.
