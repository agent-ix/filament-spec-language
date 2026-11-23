---
id: TC-1590
title: "A refinement item with internals, wider history rows and nested references"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-630
    type: verifies
---
# TC-1590: A refinement item with internals, wider history rows and nested references

## Description

Verify FR-630's request item and decisions. Scope: FR-630-AC-1 to FR-630-AC-4, FR-135-AC-6, FR-138-AC-9.

## Test Procedure

1. Analyze a register refined by a one-place queue with a correct and with a wrong mapping.
2. Analyze with and without `refinement_internals`.
3. Check and analyze refinements with a decimal and an enum history row.
4. Check and analyze, over the Counter fixture with a `chain: Sequence<Reference<Counter>>[0, 2]` field on both models, a refinement whose object row is `chain = self.chain` (the shape FR-135-AC-6 states over `RingIsQueue`; the Counter fixture is the one this test case's other steps use).

## Expected Results

- Step 1: `proved` `certified`; `refuted` with a replayed counterexample carrying per-step records.
- Step 2: the internals reach compilation; absent means the empty list.
- Step 3: both check and settle `proved`.
- Step 4: checks and settles `proved`, `certified`.
- Assertions compare the outcome document's typed members and the exit code, never message text.
