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

Verify that FR-489's claim-form rule narrows an unnamed item's candidates to one provider before negotiation, writes no named backend, and leaves every undecided multi-candidate set to settle `ambiguous-backend`. Scope: FR-962-AC-1 to FR-962-AC-5.

## Test Procedure

1. Analyze unnamed `operation-contract` items for an invariant with and without a model subject and for a `post` contract over `Engines::fsl()`.
2. Capture the FR-331 request item of the first; repeat with the linked descriptors registered in another order.
3. Register a process provider advertising `operation-contract` and repeat step 1's first item, unnamed and naming `en-1`.
. Compute the FR-146 per-step record items of the CAS refinement over `en-1` and an SMT `operation-contract` backend, and the invariant item over a model subject over the same registry.

Test: `fsl-analyze/tests/it/tied_item_resolution.rs` `an_unnamed_tied_item_reaches_negotiation_with_one_candidate`, `an_undecided_tie_settles_ambiguous_backend`; `fsl-command/tests/it/run_requirement_items.rs` `an_unnamed_run_item_with_two_value_validity_candidates_is_ambiguous`; `fsl-analyze/tests/it/refinement_request.rs` `per_step_records_exclude_en1_and_an_invariant_keeps_it`.

## Expected Results

- Step 1: candidates `[en-1]`, `[contract-checker]`, `[contract-checker]`; never `ambiguous-backend`.
- Step 2: no `backend` member, candidates `[en-1]`; equal under both orders.
- Step 3: `ambiguous-backend` naming the three candidates; named, settles on `en-1`.
- Step 4: `ambiguous-backend` naming both; named, settles on `value-checker`.
- Step 5: each per-step item matches `[smt]`; the invariant item still matches `en-1`.
