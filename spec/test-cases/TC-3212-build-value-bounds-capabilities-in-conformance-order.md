---
id: TC-3212
title: "Build value-bounds capabilities in conformance order"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1042
    type: verifies
---
# TC-3212: Build value-bounds capabilities in conformance order

## Description

Verify the remaining value-bounds capabilities from checker roles through
bounded measure and quantile products. Scope: FR-1042-AC-16 to FR-1042-AC-24.
The conformance order is checker role and policy semantics; partial and subset
lower documents; expected-reward, unbounded reachability and fair bases; then
finite-horizon measure and quantile bases.

## Test Procedure

1. Check AC-1's lower and upper documents with the claim-side flag both set
   and unset, including the threshold miss and wrong-kind cases of AC-16.
2. Check AC-3's hinted and unhinted policies, including the rejected
   nonsatisfying hint of AC-17.
3. Stop AC-1's exploration at `max_states`; check and settle the partial
   lower document without a counterpart, then mutate its threshold value and
   identity as specified by AC-18.
4. Check the subset-named reward lower document of AC-19, its full-product
   upper counterpart, and the changed-value negative case.
5. Run the expected-reward claim of AC-20 with `max_rational_bits` 1 and
   `max_precision_bits` 16; pass both emitted documents to a separate
   `fsl-probability-check` process and repeat with one value changed.
6. Run AC-21's unbounded reachability claim with the same limits and check
   both documents in a separate `fsl-probability-check` process; repeat with
   one value changed.
7. Run AC-22's fair eventual claim with the same limits and check both
   documents in a separate `fsl-probability-check` process; repeat with one
   value changed.
8. Run AC-23's finite-horizon mean claim with the same limits and check both
   documents in a separate `fsl-probability-check` process; repeat with one
   value changed.
9. Run AC-24's finite-horizon quantile claim with the same limits and check
   both documents in a separate `fsl-probability-check` process; repeat with
   one value changed.

Tag each assertion `#[trace("TC-3212", "FR-1042-AC-n")]` for its criterion.

## Expected Results

- Steps 1 and 2 enforce the supplied claim-side role and policy hint and
  retain the checker-selected canonical action when no hint is supplied.
- Step 3 settles `proved`, `ValueBounds` with `exploration: Partial`, upper
  1 and no counterpart; its documents are replay-accepted, while either
  mutation prevents proof.
- Step 4 accepts the sound subset lower and its full-product counterpart;
  the value mutation rejects `InequalityFails`.
- Steps 5 to 9 settle `proved`, `ValueBounds` enclosing the exact values 2,
  1/2, 1/2, 5/12 and 3/80 respectively, and each claim-side bound meets its
  threshold. Each document is accepted by the separate checker and a
  one-value mutation is rejected.
