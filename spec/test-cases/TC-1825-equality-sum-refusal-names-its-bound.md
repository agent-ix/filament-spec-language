---
id: TC-1825
title: "An equality sum refusal names its bound"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-680
    type: verifies
---
# TC-1825: An equality sum refusal names its bound

## Description

Verify FR-680-AC-6: the FSpec FR-149 rule is kept, its refusal names interval, bound and operand, and the explicit join checks.

## Test Procedure

Check `c = a + b` and `convert<Integer>(c) = a + b` over three `Int[0, 10]` parameters. Stubs in `fsl-semantics/tests/it/type_decisions.rs` and the conformance cases `eq-sum-accepted.native` and `eq-sum-ill-typed.native`.

## Expected Results

`c = a + b` is `unproved-range` at `a + b` with a message naming `[0, 20]`, `Int[0, 10]` and `c`; the explicit join checks.
