---
id: TC-2701
title: "run_with_providers settles a call's requirement items"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-961
    type: verifies
---
# TC-2701: run_with_providers settles a call's requirement items

## Description

Verify the run-side provider entry: the negotiator parameter, the order of work, one provider request per backend after negotiation, plain `command::run` as the linked-only entry, and the refusing and cancelled cases. Scope: FR-961-AC-1 to FR-961-AC-6.

## Test Procedure

1. Call `run_with_providers` over the `Even` run naming the recording provider, with `deadline_ms` 250.
2. Call `command::run` over the same request, and over the call with `requirements` `{}`.
3. Call `run_with_providers` over a run without `requirements`.
4. Register two providers with one `BackendId`; run a call whose compile refuses.
5. Have the provider report a counterexample the evaluator does not reproduce; run with a cancelled handle.
6. Run the `Even` request through `run_with_providers_and_limits_and_overrides` and through `run_with_limits_and_overrides` with `overrides.limits` holding `s1.nodes` 1, then with that bound at its default; run it with accounting `work_units` 0 through both entries.

Test: `fsl-command/tests/it/run_requirement_items.rs` `run_with_providers_settles_the_call_s_items_after_negotiation`, `plain_run_settles_with_the_linked_engines_only`, `run_with_providers_refuses_and_settles_the_negative_cases`.

## Expected Results

- Step 1: one negotiation per item, then one `settle` call with every item, the admitted package and `deadline_ms` 250; value true and each item `proved` `trusted` naming the provider.
- Step 2: each item `unknown-backend`, value true, no provider call; with `{}`, linked settlement and no provider call.
- Step 3: neither negotiator nor provider called; result equals `command::run`'s.
- Step 4: `invalid-request` naming `providers` before any stage; no item settled.
- Step 5: `inconclusive` `replay-parity`; every item `incomplete` `cancelled` and no request sent.
- Step 6: both entries settle the typed limit outcome naming `s1.nodes`, value 1, `incomplete`, exit 22, with no item settled and no negotiator or provider call; at the default bound the result equals step 1's; with `work_units` 0 both outcomes are `incomplete` on `work_units` and the provider entry settles every item.
