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

Verify that `OutcomeDocument::to_bytes` writes RFC 8785 canonical bytes and
that the member order of a request never reaches the output. Scope:
FR-286-AC-26, FR-286-AC-27.

## Test Procedure

1. Re-encode each golden outcome document through `quire-canonical` and
   compare with its `to_bytes` output; read the member order of
   FR-286-AC-2's diagnostic.
2. Run the canonical check over the bytes
   `{"code":"ill_typed","cause":"type-mismatch"}`.
3. Write FR-286-AC-5's `seven` `execute` request and FR-286-AC-18's request
   naming the parameter `zz` with the members of every object reversed;
   serialize the outcome of each beside the outcome of the request as
   written.
4. Serialize FR-056-AC-73's three-defect refusal.

## Expected Results

- Step 1: every re-encoding equals the bytes; the diagnostic's members are
  `cause`, `code`, `details`, `hint`, `locus`, `message` in that order.
- Step 2: the check refuses the bytes, whose re-encoding differs.
- Step 3: each pair of documents is byte-equal.
- Step 4: the three diagnostics are in intake order.

Test: `fsl-replay/tests/it/outcome_schema.rs::outcome_bytes_are_canonical_and_ignore_request_member_order`.
