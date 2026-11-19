---
id: TC-2494
title: "Elapsed-quantile verdicts are certified"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-877
    type: verifies
---
# TC-2494: Elapsed-quantile verdicts are certified

## Description

Elapsed-quantile verdicts are certified.

Scope: FR-877-AC-1, FR-877-AC-2, FR-877-AC-3, FR-877-AC-4.

## Test Procedure

Decide the four quantile claims of FR-877 over `Retx` under every scheduler and tamper the proved ones. Tests: `fsl-analyze/tests/it/timed_product_certificate.rs` `elapsed_quantile_at_most_is_certified`, `elapsed_quantile_at_most_refutes` (negative), `elapsed_quantile_at_least_is_certified_over_regions`, `elapsed_quantile_tight_search_rejects` (negative).

Tag each test with `#[trace("TC-2494", ...)]` and the FR-877 criterion it asserts.

## Expected Results

- `<= 2 ms` proved, digital exact initial `2/5`; `<= 1 ms` refuted, tampered `1/2` rejected.
- `>= 1 ms` proved, region upper `−1/2`; `quantile 0.9 >= 2 ms` refuted.
