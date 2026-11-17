---
id: TC-2920
title: "Request a refinement transfer from a native run"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: verifies
---
# TC-2920: Request a refinement transfer from a native run

## Description

Verify the `refinement_transfer` request member and its outcome. Scope: FR-1030-AC-1 to AC-4.

## Test Procedure

1. Run a `refinement_transfer` request for `PrevSafe` and `SoonTwo` through the step-for-step `ThirdRefinesSpec` with no `premises`.
2. Run the same clauses and `always holds(c.value <= 2)` through `CasRefinesCounter`.
3. Supply proved `certified` premise results for the refinement and `PrevSafe`, then a `refuted` claim premise.
4. Run FSpec FR-380-AC-8's liveness clause through `CasRefinesCounter` with no `premises`.

## Expected Results

- Step 1: `step_for_step.holds` `true`; both transfers `proved`, `decisive-witness`, with both premise identities; exit `0`.
- Step 2: `step_for_step.holds` `false` naming `Impl::Counter::beginA`, `stutter`; `inconclusive`, `transfer-not-preserving{reason: fragment}` naming `once[1,1]` and `eventually[0,2]`; the fragment clause `proved`; exit `22`.
- Step 3: no premise settled again; `proved`, `uncertified`; then `premise-unproved` naming slot 1, `refuted`.
- Step 4: settled over the stutter closure; `proved`.
