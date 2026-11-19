---
id: TC-2725
title: "A provider registers for the kinds it advertises"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-968
    type: verifies
---
# TC-2725: A provider registers for the kinds it advertises

## Description

Verify that a provider advertising `operation-contract` registers, that `value-validity` must be advertised in `unbounded`, that an empty advertisement refuses, and that negotiation reaches the provider only for its advertised pairs.

Scope: FR-968-AC-1, FR-968-AC-2.

## Test Procedure

1. Build engines from the four descriptors of FR-968-AC-1.
2. With the `operation-contract` provider registered, route an unnamed `post` item, the same item naming the provider, and a `value-validity` item handed `Supported` on it.

Test: `fsl-analyze/tests/it/tied_item_resolution.rs` `a_provider_registers_for_the_kinds_it_advertises`; step 2's tie is `an_undecided_tie_settles_ambiguous_backend`.

## Expected Results

- Step 1: the first two register; the last two refuse naming their `BackendId`, never called.
- Step 2: `ambiguous-backend` naming three; sent to the provider; `inconsistent-candidates`, not called.
