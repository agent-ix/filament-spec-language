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

Verify plan search, policy pruning, native replay, the core certificate checker and the planner's test seams over FSpec's published counter fixture. Scope: FR-755-AC-1 to FR-755-AC-4.

## Test Procedure

1. Over the counter fixture with goal value 8 and an empty policy, run with the objective `length`, then with the per-step cost sum (`double` 5, `inc` 1).
2. Run goal value 21. Through `PlanFault::DropSuccessor`, drop successor 1 of entry 0 from the goal-8 `potential` certificate, and successor 0 of entry 0 from the goal-21 closure certificate.
3. Run with the policy `always holds(c.value != 4)`.
4. Through `run_with_plan_fault`, report five `double` steps; send `synthesis` members carrying `replay_plan` and `engines`.

## Expected Results

- Step 1: `optimal` `inc, double, double` with `objective.value` `{"integer": "3"}`, then `optimal` `inc, inc, inc, double` with `{"integer": "8"}`, each `potential` accepted, `certified`.
- Step 2: `infeasible` with an accepted closure certificate; the tampered `potential` rejected at `closure`, entry 0, result `feasible` `inc, double, double`, `uncertified`; the tampered closure rejected at `closure`, entry 0, result `incomplete` with `certificate-rejected`, plan `null`, exit 22.
- Step 3: no plan state has value 4 and replay holds the policy at every state.
- Step 4: a method failure, exit 30, naming step 4 `double`, no `synthesis` outcome; each member refuses `unknown-member` at its `/synthesis/...` pointer, exit 20.
