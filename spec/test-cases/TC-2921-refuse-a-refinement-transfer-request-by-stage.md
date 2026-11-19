---
id: TC-2921
title: "Refuse a refinement transfer request by stage"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1031
    type: verifies
---
# TC-2921: Refuse a refinement transfer request by stage

## Description

Verify the refusals of a `refinement_transfer` request. Scope: FR-1031-AC-1 to AC-4.

## Test Procedure

1. Combine `refinement_transfer` with `call`, then with `items`; name `PrevSafe` twice in `clauses`.
2. Name `abstract.model` `Ghost`; drop the `models` entry of `Spec`; name `concrete.model` `Impl` for `ThirdRefinesSpec`.
3. Name a state invariant, a bounded-profile temporal clause, and a temporal clause over `Third::Counter`.
4. Supply a claim premise over another abstract package; run `CasRefinesCounter`.

## Expected Results

- Step 1: stage `request`, `invalid-request` naming both members, exit `20`; `invalid-request` at `/refinement_transfer/clauses/1`.
- Step 2: `select` `missing_declaration`/`missing-name`; `compile` `missing_import`/`missing-selection`; `select` `invalid_runtime_input`/`invalid-value` with `declared` `Third`; each exit `20`.
- Step 3: `select` `ill_typed`/`type-mismatch` twice; `invalid_runtime_input`/`invalid-value`.
- Step 4: `admit` `invalid_runtime_input`/`invalid-value` at `/refinement_transfer/premises/claims/PrevSafe`; `CasRefinesCounter` is not refused.
