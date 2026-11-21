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

Verify that `call.requirements` asks for settlement and names each item's backend by the call-level default or a per-item entry, with analyze's identity rule, and that malformed members refuse before the call. Scope: FR-960-AC-1 to FR-960-AC-9.

## Test Procedure

1. Run `sq` with `x = 3` and `requirements` `{}`, and without `requirements`.
2. Run it with `{"backend": "value-checker"}`, `{"backend": "fsl.absent"}` and `{"backend": "fsl.absent", "items": [{"claim": C}]}`.
3. Run each malformed `requirements` of FR-960-AC-3 and an entry naming no item of `sq`.
. Run `sq` naming the first item by the claim its outcome reports (held), by that claim in FSpec FR-331's structural spelling, and by a held entry beside a structural entry for the second item.
6. Name the first item's claim in both spellings in two entries.
7. Replace an index segment of the held claim by `{"node": 999999}`, `{"node": "3"}` and `{"node": -1}`.
8. Name a held claim whose occurrence ordinal is 99.
9. Ask the document graph of a package the strict reader refuses (`missing_import`) for a held claim.

Test: `fsl-command/tests/it/run_requirement_items.rs` `a_run_call_names_the_backend_of_its_requirement_items`, `malformed_requirements_refuse_before_the_call`, `a_claim_written_by_a_result_names_its_item_when_supplied_back`, `a_structural_claim_names_its_item`, `a_held_entry_and_a_structural_entry_name_their_own_items`, `one_claim_written_in_both_forms_refuses_as_a_duplicate`, `a_held_claim_with_a_bad_index_refuses_at_its_node_member`, `a_held_claim_naming_no_item_of_the_call_is_an_unknown_claim`; `fsl-analyze/tests/it/package_refusal_route.rs` `the_document_graph_of_a_package_the_reader_refuses_is_the_readers_refusal`.

## Expected Results

- Step 1: value 16 and three entries with `value-checker` as sole candidate; without the member, value 16 and no `requirements`.
- Step 2: named `value-checker`; every item `unknown-backend`; the first item on `value-checker`, the other two `unknown-backend`.
- Step 3: stage `request` `invalid-request` naming the pointer, exit 20; the unknown claim refuses at stage `call` with `missing_declaration`.
- Step 4: each item `invalid-request` `invalid_capability`/`unknown-option` naming `value-checker` and `unknown`; the value unchanged.
- Step 5: each spelling names the first item, which settles on `value-checker`; the held and the structural entry each settle on `value-checker`.
- Step 6: stage `request` `invalid-request` naming `/call/requirements/items/1`, exit 20.
- Step 7: stage `request` `invalid-request` naming `/call/requirements/items/0/claim/node/path/K/node`, exit 20.
- Step 8: stage `call` `missing_declaration`, `details` `claim` the claim as supplied.
- Step 9: the reader's `missing_import`, not `resource_exhausted`.
