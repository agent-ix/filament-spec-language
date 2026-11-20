---
id: TC-3111
title: "Outcome bytes are canonical and independent of request member order"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3111: Outcome bytes are canonical and independent of request member order

## Description

Verify that every FSL document writer encodes through the one RFC 8785
encoder, with UTF-16 code-unit key order, and that request member order
reaches the output only where a refusal is defined by it. Scope:
FR-286-AC-26, FR-286-AC-27.

## Test Procedure

1. Re-encode each golden document (the `analyze`, `monitor` and `execute`
   documents included) through `quire-canonical` and compare with its
   bytes; read the member order of FR-286-AC-2's diagnostic and of an
   `unsupported_construct`/`declaration-form` diagnostic carrying
   `construct`, `declaration`, `profile` and `text`.
2. Run the canonical check over the bytes
   `{"code":"ill_typed","cause":"type-mismatch"}`.
3. Encode `{"\u{E000}": 1, "\u{1F600}": 2}` with
   `fsl_foundation::canonical_json::encode`.
4. Write FR-286-AC-5's `seven` `execute` request, FR-286-AC-18's request
   naming the parameter `zz`, and a `refinement_transfer` request with the
   one unknown member `zz`, each with the members of every object reversed;
   encode the outcome of each beside the outcome of the request as written.
5. Write the `refinement_transfer` request with two unknown members as `yy`
   then `zz`, and as `zz` then `yy`.
6. Encode FR-056-AC-73's three-defect refusal.

## Expected Results

- Step 1: every re-encoding equals the bytes; the diagnostics' members are
  `cause`, `code`, `details`, `hint`, `locus`, `message`, and `cause`,
  `code`, `construct`, `declaration`, `details`, `hint`, `locus`,
  `message`, `profile`, `text`.
- Step 2: the check refuses the bytes, whose re-encoding differs.
- Step 3: the `U+1F600` key is written first.
- Step 4: each pair of documents is byte-equal.
- Step 5: the refusals name `/refinement_transfer/yy` and
  `/refinement_transfer/zz` respectively.
- Step 6: the three diagnostics are in intake order.

Test: `fsl-command/tests/it/outcome_schema.rs::outcome_bytes_are_canonical_and_ignore_request_member_order`.
