---
id: TC-1823
title: "Some adopts the expected option element type"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-680
    type: verifies
---
# TC-1823: Some adopts the expected option element type

## Description

Verify FR-680-AC-4: `Some(e)` is a general constructor and its literal payload adopts the expected bounded element type.

## Test Procedure

Check `Some(3)` and `Some(30)` at an `Option<Int[0, 9]>` result and `=` operand, and `Some(1)` in a state clause. Stubs in `fsl-semantics/tests/it/type_decisions.rs` and the conformance cases `some-accepted.native` and `some-ill-typed.native`.

## Expected Results

`Some(3)` checks; `Some(30)` is `ill_typed`/`type-mismatch` at `30`; the state-clause `Some(1)` is the constructor.
