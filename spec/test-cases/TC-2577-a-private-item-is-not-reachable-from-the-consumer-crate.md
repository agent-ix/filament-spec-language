---
id: TC-2577
title: "A private item is not reachable from the consumer crate"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-911
    type: verifies
---
# TC-2577: A private item is not reachable from the consumer crate

## Description

A private item is not reachable from the consumer crate.

Scope: FR-911-AC-4.

## Test Procedure

Run the `facade-contract-consumer` doctests.

Tag the tests `#[trace("TC-2577", "FR-911-AC-4")]`.

## Expected Results

- The `compile_fail` doctest naming `fsl_package::protocol_view::relation_over` fails with `E0603`.
- The same doctest naming `fsl_package::protocol_view::causal_relation` compiles.
