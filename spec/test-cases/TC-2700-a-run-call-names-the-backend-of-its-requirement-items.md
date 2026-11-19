---
id: TC-2700
title: "A run call names the backend of its requirement items"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-960
    type: verifies
---
# TC-2700: A run call names the backend of its requirement items

## Description

Verify that `call.requirements` asks for settlement and names each item's backend by the call-level default or a per-item entry, with analyze's identity rule, and that malformed members refuse before the call. Scope: FR-960-AC-1 to FR-960-AC-4.

## Test Procedure

1. Run `sq` with `x = 3` and `requirements` `{}`, and without `requirements`.
2. Run it with `{"backend": "value-checker"}`, `{"backend": "fsl.absent"}` and `{"backend": "fsl.absent", "items": [{"claim": C}]}`.
3. Run each malformed `requirements` of FR-960-AC-3 and an entry naming no item of `sq`.
4. Run with an option `value-checker` does not publish.

Test: `fsl-command/tests/it/run_requirement_items.rs` `a_run_call_names_the_backend_of_its_requirement_items`, `malformed_requirements_refuse_before_the_call`.

## Expected Results

- Step 1: value 16 and three entries with `value-checker` as sole candidate; without the member, value 16 and no `requirements`.
- Step 2: named `value-checker`; every item `unknown-backend`; the first item on `value-checker`, the other two `unknown-backend`.
- Step 3: stage `request` `invalid-request` naming the pointer, exit 20; the unknown claim refuses at stage `call` with `missing_declaration`.
- Step 4: each item `invalid-request` `invalid_capability`/`unknown-option` naming `value-checker` and `unknown`; the value unchanged.
