---
id: TC-2702
title: "An unnamed tied item is resolved by the production tie rule before negotiation"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-962
    type: verifies
---
# TC-2702: An unnamed tied item is resolved by the production tie rule before negotiation

## Description

Verify that FR-489's claim-form rule narrows an unnamed item's candidates to one provider before negotiation, writes no named backend, and leaves every undecided multi-candidate set to settle `ambiguous-backend`. Scope: FR-962-AC-1 to FR-962-AC-4.

## Test Procedure

1. Analyze unnamed `operation-contract` items for an invariant with and without a model subject and for a `post` contract over `Engines::fsl()`.
2. Capture the FR-331 request item of the first; repeat with the linked descriptors registered in another order.
3. Register a process provider advertising `operation-contract` and repeat step 1's first item, unnamed and naming `en-1`.
4. Register a process provider advertising `value-validity` and run `sq` with `requirements` `{}` and with `value-checker` named.

Test: `fsl-command/tests/it/run_requirement_items.rs` `an_unnamed_tied_item_reaches_negotiation_with_one_candidate`, `an_undecided_tie_settles_ambiguous_backend`.

## Expected Results

- Step 1: candidates `[en-1]`, `[contract-checker]`, `[contract-checker]`; never `ambiguous-backend`.
- Step 2: no `backend` member, candidates `[en-1]`; equal under both orders.
- Step 3: `ambiguous-backend` naming the three candidates; named, settles on `en-1`.
- Step 4: `ambiguous-backend` naming both; named, settles on `value-checker`.
