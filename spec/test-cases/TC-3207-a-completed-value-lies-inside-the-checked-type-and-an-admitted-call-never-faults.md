---
id: TC-3207
title: "A completed value lies inside the checked type and an admitted call never faults"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-096
    type: verifies
---
# TC-3207: A completed value lies inside the checked type and an admitted call never faults

## Description

Verify that every call the checker admits and the evaluator completes returns a value inside the type the checker assigned to the call node, and that no admitted call returns an internal fault. Scope: FR-096-AC-23.

Catches a kernel that completes with a value outside its checked integer, rational, decimal or collection type, and an admitted call that reaches `runtime_invariant` instead of completing or refusing with a typed cause.

## Test Procedure

1. Generate checked functions over integer, rational, decimal and collection types, with generated admitted arguments.
2. Call each through `CheckedPackage::call`.
3. For each completed outcome, test the value against the checked type of the call node.
4. Read each non-completed outcome.

## Expected Results

1. Every completed value lies inside the checked type of its call node.
2. No outcome is an `InternalFault` with code `runtime_invariant`; the only outcomes are completed, undefined or a typed refusal, and the run-time-only operations are exactly the listed ones.
3. Test: `fsl-eval/tests/it/typed_value_differential.rs` `p1_completed_values_lie_inside_the_checked_type` and `p2_no_completed_call_returns_runtime_invariant`.
