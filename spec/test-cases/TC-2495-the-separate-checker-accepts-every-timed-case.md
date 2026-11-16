---
id: TC-2495
title: "The separate checker accepts every timed case"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-921
    type: verifies
---
# TC-2495: The separate checker accepts every timed case

## Description

The separate checker accepts every timed case.

Scope: FR-921-AC-4.

## Test Procedure

Produce the four documents of FR-921-AC-4 with EN-5 as a development dependency, then run the built `fsl-probability-check` in a fresh process on each and on each with one value altered. Test: `tools/fsl-probability-check/tests/it/timed_product.rs` `timed_product_certificates_pass_the_separate_checker`.

Tag each test with `#[trace("TC-2495", ...)]` and the FR-921 criterion it asserts.

## Expected Results

- Exit 0 for each document; nonzero for each altered one.
