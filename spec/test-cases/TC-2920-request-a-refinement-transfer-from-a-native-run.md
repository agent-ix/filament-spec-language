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

Verify the `refinement_transfer` request member and its outcome. Scope: FR-1030-AC-1 to AC-11.

## Test Procedure

1. Run a `refinement_transfer` request for `PrevSafe` and `SoonTwo` through the step-for-step `ThirdRefinesSpec` with no `premises`.
2. Run the same clauses and `always holds(c.value <= 2)` through `CasRefinesCounter`.
3. Supply proved `certified` premise results for the refinement and `PrevSafe`, then a `refuted` claim premise.
4. Run FSpec FR-380-AC-8's liveness clause through `CasRefinesCounter` with no `premises`.
5. Run `PrevStutter`, `always (holds(c.value = 2) implies once[1,1] holds(c.value = 1))`, through `ThirdRefinesSpec` with no `premises`.
6. Run `StuckTwo` (declared in `TRANSFER_DECLARATIONS`, no fairness) through `CasRefinesCounter` with no `premises`, and read the outcome. The transfer outcome carries no lasso: slot 1 is `refuted` only through the production replay gate, and the lossless equality of a written and read closure lasso and its post-read replay over the source and package routes are asserted by the FR-128-AC-14 and FR-128-AC-15 tests over the same closure-stutter mechanism.
7. Use `temporal PendingTwo using inf over (c: Spec::Counter) clock "model-steps" on origin { always (holds(c.value = 1) implies eventually holds(c.value = 2)) }`, the template of `StuckTwo`, declared in `TRANSFER_DECLARATIONS`, and run it through `CasRefinesCounter` with no `premises`.
8. Declare `PrevSafeEach`, `SoonTwoEach`, `PrevJumpEach` and `PrevStutterEach` as FR-1030-AC-8 and AC-9 give them, and run `PrevSafeEach` and `SoonTwoEach` through `ThirdRefinesSpec` with no `premises`.
9. Run `PrevJumpEach` and `PrevStutterEach` through `ThirdRefinesSpec` with no `premises`.
10. Run `PrevSafeEach` through `ThirdRefinesSpec` with `abstract.initial` naming a snapshot of `value` 1.
11. Settle `PrevSafeEach`'s abstract claim from the checked package S4 emits for the unit, and settle its transfer through `planner::settle` over that package.

## Expected Results

- Step 1: `step_for_step.holds` `true`; both transfers `proved`, `decisive-witness`, with both premise identities; exit `0`.
- Step 2: `step_for_step.holds` `false` naming `Impl::Counter::beginA`, `stutter`; `inconclusive`, `transfer-not-preserving{reason: fragment}` naming `once[1,1]` and `eventually[0,2]`; the fragment clause `proved`; exit `22`.
- Step 3: no premise settled again; `proved`, `uncertified`; then `premise-unproved` naming slot 1, `refuted`.
- Step 4: settled over the stutter closure; `proved`.
- Step 5: the abstract claim premise is not proved at the repeated terminal state; `inconclusive`, `premise-unproved` naming slot 1; exit `22`.
- Step 6: slot 1 `refuted` through the production replay gate, which selects the item's `stutter_closure` and reproduces the closure-only lasso; the outcome carries no lasso; `inconclusive`, `premise-unproved` naming slot 1, `result` `refuted`; exit `22`; not `unsupported`. Step 4's `EventuallyTwo` (weak `each`) stays `proved` and certified, and step 5's terminal `PrevStutter` refutation is unchanged.
- Step 7: slot 1 `refuted` through the same replay gate; the one-step real prefix and closure loop at value 1 is the shape the FR-128-AC-14 test over the Counter world writes and reads back; `inconclusive`, `premise-unproved`, `result` `refuted`; exit `22`.
- Step 8: slot 1 `proved`, `Certified`; both transfers `proved`, `decisive-witness`; exit `0`.
- Step 9: slot 1 `refuted` for each; `inconclusive`, `premise-unproved` naming slot 1; exit `22`.
- Step 10: slot 0 `refuted`, `InitialNotAbstract`; `inconclusive`, `premise-unproved` naming slot 0; exit `22`.
- Step 11: the claim and the transfer equal Step 8's, with the certificate accepted by recompiling the package.

Steps 6 and 7 verify the transfer only. The replay refusal matrix, fairness, true and undefined parity and the mixed-loop limit are FR-128-AC-10 to AC-14 over the `lasso_replay` `Counter` fixtures (TC-523), against FSpec FR-364-AC-25 to AC-29 (FSpec-TC-295); the schema vectors of FSpec FR-364-AC-26 are not evidence of replay.
