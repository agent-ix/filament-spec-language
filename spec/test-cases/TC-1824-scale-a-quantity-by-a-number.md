---
id: TC-1824
title: "Scale a quantity by a number"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-680
    type: verifies
---
# TC-1824: Scale a quantity by a number

## Description

Verify FR-680-AC-5: quantity scaling keeps the unit and its refused forms.

## Test Procedure

Check `d * 2`, `2 * d`, `d / 2`, `d / k`, `d * f` and `2 / d` over `d: Quantity<ms>`. Stubs in `fsl-semantics/tests/it/type_decisions.rs` and the conformance cases `scale-accepted.native` and `scale-ill-typed.native`.

## Expected Results

The scalings check as `Quantity<ms>`; `d / k` is `unproved-nonzero`; `d * f` and `2 / d` are `ill_typed`/`type-mismatch`.
