---
id: TC-2703
title: "A run reports its settled requirement items in all three outcome forms"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-963
    type: verifies
---
# TC-2703: A run reports its settled requirement items in all three outcome forms

## Description

Verify the `requirements` and `registry` members of `spine-run-result/1`, `RunResult.requirements`, and the `execute` document's `items`, the run category over the call and its items, and the unchanged documents of a run without `requirements`. Scope: FR-963-AC-1 to FR-963-AC-4.

## Test Procedure

1. Run `sq` with `requirements` `{}` and read the three forms.
2. Run FR-961-AC-1's `Even` run with the recording provider.
3. Run `sq` naming `fsl.absent`.
4. Run `seven` without `requirements`.

Test: `fsl-command/tests/it/run_requirement_items.rs` `a_run_call_names_the_backend_of_its_requirement_items`, `run_with_providers_settles_the_call_s_items_after_negotiation`.

## Expected Results

- Step 1: three `proved` entries naming `value-checker`, `registry` listing the linked backends; equal entries in the three forms.
- Step 2: each item `proved` `trusted` naming the provider; `registry` lists it.
- Step 3: category `refusal`, exit 20, completed outcome, disposition records with the diagnostic and no `backend`.
- Step 4: no `requirements` or `registry`, `RunResult.requirements` `None`, `items` `[]`.
