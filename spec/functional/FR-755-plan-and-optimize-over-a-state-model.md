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

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-755-AC-1 | FSpec FR-650-AC-1 and AC-2 hold over the `inc`/`double` counter fixture: `optimal` `inc, double, double` at length 3 and `inc, inc, inc, double` at cost 8, each with an accepted `potential` certificate. | Test (TC-2105) |
| FR-755-AC-2 | Goal value 21 gives `infeasible` with an accepted closure certificate; a certificate with one successor removed is rejected by the checker naming the closure rule and entry, and the result is `feasible`. | Test (TC-2105) |
| FR-755-AC-3 | The policy `always holds(c.value != 4)` gives a plan that never visits value 4, and the replayed plan satisfies the policy at every state. | Test (TC-2105) |

## Dependencies

- FR-120 (`ModelSystem`), FR-754 (synthesis form); FSpec FR-650.
