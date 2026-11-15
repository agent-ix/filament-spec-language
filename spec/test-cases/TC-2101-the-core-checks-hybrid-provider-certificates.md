---
id: TC-2101
title: "The core checks hybrid provider certificates"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-751
    type: verifies
---
# TC-2101: The core checks hybrid provider certificates

## Description

Verify the exact interval check of hybrid certificates and its effect on certification and refutation. Scope: FR-751-AC-1 to FR-751-AC-3.

## Test Procedure

1. Check FSpec FR-652-AC-1's two `inductive-enclosure` certificates.
2. Check a `flowpipe` with a Picard violation; read FSpec FR-652-AC-3's trace with an accepted and a straddling witness.
3. Check a certificate under a one-evaluation work limit.

## Expected Results

- Step 1: `Accepted` and `Certified`; `Rejected` at rule (a) and the `Moving` box pointer, `Uncertified`.
- Step 2: `Rejected` at the step's pointer; `refuted` and recorded; `inconclusive`.
- Step 3: `Rejected { rule: work-limit }`.
