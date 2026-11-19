---
id: TC-1663
title: "Carried progress ledger on the run wire"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: verifies
---
# TC-1663: Carried progress ledger on the run wire

## Description

Verify FR-641-AC-3 to AC-5 and FR-267-AC-21: a ledger written by one run and read by the next refuses a regressing watermark and advances on a later one; malformed traces and ledgers refuse at stage `request`; a ledger round-trips byte for byte.

## Test Procedure

1. Run TC-1662's step 1 at watermark 100; write the result's `ledger` to a file.
2. Run again with that file as `ledger` at watermark 60, then at 120.
3. Send a trace mixing a key and a sample, a sample trace with `over`, an observation without `stamp`, a ledger with a repeated binding.
4. Read and rewrite a ledger from a run that changes nothing; add a `ledger` member to a `finite` result and read it.

## Expected Results

- Step 2: watermark 60 reports stage `evaluate`, `refusal`, `contradicted_dimension` `Watermark`, the supplied ledger unchanged; 120 is accepted and the entry holds 120.
- Step 3: `invalid-request`, stage `request`, exit 20, naming the member.
- Step 4: equal bytes; the `finite` result with `ledger` is refused naming `ledger`.
