---
id: TC-2105
title: "The planner returns a certified optimal plan"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-755
    type: verifies
---
# TC-2105: The planner returns a certified optimal plan

## Description

Verify plan search, policy pruning, native replay and the core certificate checker. Scope: FR-755-AC-1 to FR-755-AC-3.

## Test Procedure

1. Run FSpec FR-650-AC-1 and AC-2 over the `inc`/`double` counter fixture.
2. Run goal value 21; remove one successor from a certificate and re-check.
3. Run with the policy `always holds(c.value != 4)`.

## Expected Results

- Step 1: `optimal` `inc, double, double` at 3 and seven `inc` at 7, certificates accepted.
- Step 2: `infeasible` with an accepted closure certificate; the tampered certificate rejected naming rule and entry, result `feasible`.
- Step 3: no plan state has value 4 and replay holds the policy at every state.
