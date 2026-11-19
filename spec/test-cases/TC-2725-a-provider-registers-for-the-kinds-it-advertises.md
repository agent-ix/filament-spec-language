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

Scope: FR-968-AC-1, FR-968-AC-2, FR-968-AC-3, FR-968-AC-4.

## Test Procedure

1. Build engines from the four descriptors of FR-968-AC-1.
2. With the `operation-contract` provider registered, route the unnamed `post` item `Keeps`, the unnamed `invariant` item over a model subject, the `Keeps` item naming the provider, and a `value-validity` item handed `Supported` on it.
3. With the provider registered, analyze the `invariant` item `Wrong` naming it once per verdict of FR-968-AC-3.
4. Build engines from FR-968-AC-4's descriptors.

Test: `fsl-analyze/tests/it/tied_item_resolution.rs` `a_provider_registers_for_the_kinds_it_advertises` (steps 1 and 4), `a_post_item_with_a_provider_matches_contract_checker_and_the_provider` and `an_undecided_tie_settles_ambiguous_backend` (step 2), `a_provider_contract_verdict_settles_after_replay` (step 3).

## Expected Results

- Step 1: the first two register; the last two refuse naming their `BackendId`, never called.
- Step 2: `[contract-checker, <provider>]` and `[contract-checker, en-1, <provider>]`, each `ambiguous-backend` naming every candidate; the named item is sent as one `operation-contract` request item with `Keeps`'s `post` target; `inconsistent-candidates`, not called.
- Step 3: `proved` `trusted`; `inconclusive` `solver-inconclusive`; `refuted` with the reproduced `preservation` counterexample; `replay-parity`, `replay-refused` and `replay-parity`, each with no counterexample.
- Step 4: each refusal is `invalid_capability` keyed by its `BackendId` with its cause (`absent-kind`, `value-validity-not-unbounded` with `["bounded"]`); the `temporal-satisfaction` and `schedulability` providers register.
