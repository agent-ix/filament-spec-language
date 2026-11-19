---
id: FR-755
title: "Plan and optimize over a state model and certify the plan"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-754
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-650
    type: depends_on
---
# FR-755: Plan and optimize over a state model and certify the plan

## Description

FSL SHALL implement FSpec FR-650's planner and optimizer: for a `synthesis`
declaration whose grammar is a state model object type, search operation
plans over FR-120's `ModelSystem` up to the declared plan length, order them
by the objective, replay the result natively, and emit and check the
`potential` or `closure` certificate in the core. This planner is distinct
from `fsl-replay`'s proof-composition planner, which composes proofs and
plans no operations.

## Inputs

The checked package, the `synthesis` declaration, the bound initial
snapshot, the policy clause identities, and the run's limits.

## Outputs

FSpec FR-650's `optimal`, `feasible`, `infeasible` or `incomplete` result
with its plan, certificate and provenance.

## Behavior

- The planner SHALL expand plans breadth-first by length over `ModelSystem`,
  applying only enabled invocations, pruning a prefix at the first state
  where a model state clause, invariant or policy clause fails, and
  evaluating the goal and objective natively.
- For an objective that is a sum of non-negative per-step costs it SHALL
  search in nondecreasing cost and produce a `potential` certificate; for
  any other objective it SHALL explore the full bounded closure and produce
  a `closure` certificate.
- The core certificate checker SHALL be a separate function from the
  search, re-deriving every successor by `ModelSystem` and re-evaluating
  goal, policy and objective; it alone decides `certified`.
- The planner SHALL replay every reported plan natively from the initial
  snapshot before it writes the result.
- The planner SHALL expose FSpec FR-650's two test seams in `fsl-analyze`'s
  library as one value, `PlanFault`, passed to the plan search beside its
  limits: `PlanFault::ReportPlan(steps)` replaces the plan the search
  reports with `steps` before native replay, and
  `PlanFault::DropSuccessor { entry, successor }` removes the `successor`th
  successor record (zero-based, in step order) from certificate entry
  `entry` before the core checker reads it. The command adapter exposes them
  as `fsl_command::command::run_with_plan_fault(path, negotiate, fault)`,
  which runs a `native-run/1` request exactly as `run` does with `fault`
  applied. No request reader, CLI option or wire member constructs a
  `PlanFault`, so the `synthesis` member stays closed (FR-1010).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-755-AC-1 | Over FSpec FR-650 "The counter fixture" (model `M`, `Counter.value: Int[0, 20]`, `inc` then `double`, `c.value = 1`, `domain Upto5`, goal `Reach.final.value = 8`) with an empty policy list: the default objective `length` gives `optimal` `inc, double, double` with `objective.value` `{"type": "integer", "value": "3"}`, and `minimize sum<Cost>(s in Reach.steps : if s.operation = M::Op::double then 5 else 1)` (the fixture declares `type Cost = Integer;`) gives `optimal` `inc, inc, inc, double` with `objective.value` `{"type": "integer", "value": "8"}`, each with an accepted `potential` certificate and certification `certified`. The same 4-step plan under the policy `SomeTime` with the objective `length` is FSpec FR-803-AC-2's, objective 4, a different request. | Test (TC-2105) |
| FR-755-AC-2 | Goal value 21 gives `infeasible` with an accepted closure certificate. Through `PlanFault::DropSuccessor { entry: 0, successor: 1 }`, the goal-8 `potential` certificate is rejected with `rule` `closure` and `entry` 0 and the result is `feasible` `inc, double, double`, `uncertified`; the goal-21 closure certificate with `DropSuccessor { entry: 0, successor: 0 }` is rejected with `rule` `closure` and `entry` 0 and the result is `incomplete` with cause `certificate-rejected` and plan `null`, exit `22`. | Test (TC-2105) |
| FR-755-AC-3 | The policy `always holds(c.value != 4)` gives a plan that never visits value 4, and the replayed plan satisfies the policy at every state. | Test (TC-2105) |
| FR-755-AC-4 | Through `run_with_plan_fault` with `PlanFault::ReportPlan` of five `double` steps over the counter, the run is a method failure, exit `30`, naming step index 4 and `double`, and writes no `synthesis` outcome; a `synthesis` member carrying `replay_plan` or `engines` refuses `invalid_runtime_input`/`unknown-member` at `/request/synthesis/replay_plan` or `/request/synthesis/engines`, exit `20`. | Test (TC-2105) |

## Dependencies

- FR-120 (`ModelSystem`), FR-754 (synthesis form); FSpec FR-650.
