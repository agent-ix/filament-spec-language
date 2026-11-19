---
id: TC-2023
title: "Related-by clauses restrict event steps and replay"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-731
    type: verifies
---
# TC-2023: Related-by clauses restrict event steps and replay

## Description

Verify that a `related by` clause is a premise of its event node's step in exploration and in replay. Scope: FR-731-AC-1 to FR-731-AC-3.

## Test Procedure

| # | Change | Expected |
| --- | --- | --- |
| 1 | orders `o1`, `o2`, payments `p1` settling `o1`, `p2` settling `o2`; `event Paid ... related by settles (paid, input)` over `o1` | `Paid` steps bind `p1` only; without the clause both |
| 2 | a second clause no payment satisfies; the two clauses swapped | no `Paid` step, the thread waits; equal states in both orders |
| 3 | replay of a trace whose `Paid` binds `p2`; one binding `p1` | `invalid_runtime_input`/`invalid-value` naming `Paid` and `settles`; the second replays |

## Expected Results

- States are compared by state key; refusals by code, cause and retained node and clause.
