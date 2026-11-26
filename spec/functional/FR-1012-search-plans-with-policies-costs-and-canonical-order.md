---
id: FR-1012
title: "Search plans with exact costs, policy monitors and the canonical plan order"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-800
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-802
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-803
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-804
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-808
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-809
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-755
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: depends_on
---
# FR-1012: Search plans with exact costs, policy monitors and the canonical plan order

## Description

FSL's plan search in `fsl-analyze` SHALL implement FSpec FR-800 to FR-804, FR-808 and FR-809: search nodes keyed by the canonical state key, monitor states and accumulators; exact objectives classified as per-step sums or general; safety monitors that prune and non-safety policies evaluated on each completed plan's behaviour; the canonical plan order; delays over the region abstraction; and exact probabilistic outcome distributions.

## Behavior

- The search SHALL be a separate module from `fsl-replay`'s proof-composition planner and from the verification planner.
- The reported plan and every count SHALL be those of FSpec FR-801's canonical sequential exploration ("Exploration order": layers by step count, nodes within a layer by accumulated per-step sum then representative prefix in FSpec FR-804's step order, successors in step order, and its fixed charge order), whatever the worker count.
- Over a timed grammar the search SHALL implement FSpec FR-808: tick steps of delay 0, dense-time steps of an exact delay then an invocation over FSpec FR-418's region abstraction, concrete delays at the least point or the open-bound midpoint, and an unattained infimum reported `feasible` with its exact `bound`; every `delay` and `elapsed` is written as FSpec FR-181's unit-bearing quantity in the model's time unit.
- Over a probabilistic grammar the search SHALL implement FSpec FR-809: open-loop actions with drawn arguments, adversarial choice, nodes holding the exact state distribution, and `probability` and the expected objective computed as exact rationals.
- The plan search SHALL keep, for each policy clause, one monitor for each admitted key of the clause's `over` type (FR-125 "Instancing").
- A plan keeps a policy only when every one of that policy's monitors accepts it (FSpec FR-803).
- Every reported plan SHALL replay natively before it is written (FR-755).
- A requested policy over a type SHALL monitor the distinct object keys that the initial snapshot lists for that type, in snapshot list order. A policy with no such admitted instance SHALL settle `inconclusive` with cause `no-instance` before exploration; it SHALL NOT be treated as vacuously satisfied or produce `optimal`, `feasible` or `infeasible`. A non-reference `over` binding that has no instances is not this condition.
- For each policy clause the search SHALL charge `max_work` once per clause, preserving the clause's snapshot instance order and stopping at the first decisive instance.
- Before policy-name and `over` binding refusals, an initial population marked `complete:false` SHALL refuse `incomplete_population` / `incomplete-scope` at `/request/synthesis/initial`. After every policy name and `over` binding resolves, an omitted population required by a policy SHALL refuse `incomplete_population` / `missing-required-artifact` at that pointer. Both refusal rows exit `22`.
- A state at which a model invariant is violated SHALL be inadmissible and never part of a plan. A state at which an invariant cannot be decided SHALL NOT be expanded; if the search excluded any state for that reason, it SHALL settle `incomplete` with cause `invariant-undetermined` naming the invariant, carrying the best plan found that does not pass through such a state, and SHALL NOT settle `optimal` or `infeasible` or write a certificate. A certificate check that re-derives such a state SHALL end with the same cause, not a certificate rejection, and a plan that passes through such a state is not admissible evidence at replay.

The first positive-length source-policy increment covers capture-free,
origin-activated infinite-trace `always holds(A)`, `eventually[0,k] holds(A)`
and `always[0,k] holds(A)` through their canonical shared monitor states and
terminal-stutter closure. Other checked positive-length shapes retain an
explicit unsupported result until implemented. Captures, general liveness
and FR-1012-AC-1 remain required subsequent increments; this staged boundary
does not change their acceptance obligations.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1012-AC-1 | The spine plan under the policy `eventually holds(c.versionNumber = 3)` is evaluated, not refused, and every reported plan visits version 3. | Test (TC-2862) |
| FR-1012-AC-2 | Two runs of one plan request give byte-identical outcome documents. | Test (TC-2862) |
| FR-1012-AC-3 | A decimal per-step cost objective is written exactly in FSpec FR-181's typed `decimal` form (`{"type": "decimal", "coefficient": "30", "scale": "2"}` for `0.30`) and certified with a `potential` certificate. | Test (TC-2862) |
| FR-1012-AC-4 | Over FSpec's published `timed-tick` fixture the plan's `elapsed` is `{"type": "rational", "numerator": "30", "denominator": "1", "unit": "min"}` with every `delay` of numerator 0 (FSpec FR-808-AC-1); over `timed-go` with guard `x >= 2` the result is `optimal` with one step of delay `{"type": "rational", "numerator": "2", "denominator": "1", "unit": "s"}` and an accepted `closure` certificate (FR-808-AC-2), and with guard `x > 2` `feasible` with `bound` `{side: lower, value: 2/1, attained: false}` and delay `{"type": "rational", "numerator": "5", "denominator": "2", "unit": "s"}`, byte-identical across two runs (FR-808-AC-3, AC-5). | Test (TC-2862) |
| FR-1012-AC-5 | Over FSpec's published `probabilistic-try` fixture the goal `Reach.final.done` gives `optimal` `retry` (FSpec FR-809-AC-1), and `maximize Reach.probability` over lengths 1 to 2 using only `try` gives `optimal` `try, try` with `objective.value` the Rational `3/4` (FR-809-AC-2). | Test (TC-2862) |
| FR-1012-AC-6 | Over FSpec's published `counter` fixture, `limits` `{"max_states": 2}` gives `incomplete` with `explored.states` 2 and `explored.work` 6 and plan `null`, and `{"max_work": 1}` gives `explored.states` 1 and `explored.work` 1 (FSpec FR-801-AC-8), each byte-identical across two runs. | Test (TC-2862) |
| FR-1012-AC-7 | Over a model whose invariant is undetermined at every state from some value up, a goal reachable only through such a state gives `incomplete` with cause `invariant-undetermined` naming the invariant, plan `null` and no certificate, never `infeasible`; a goal reached before the first such state gives `incomplete` carrying that plan, never `optimal`; and a plan substituted through such a state is not reported. | Test (TC-2862) |
| FR-1012-AC-8 | For a policy over two snapshot instances listed `b` then `a`, instance evaluation follows that order; when both satisfy the clause the empty plan is `optimal` with `explored.states` 1 and `explored.work` 3, and when only the second rejects the result is `infeasible` with states 1 and work 1. With `max_work:1` and both instances safe, the result is `incomplete` with states 1 and work 1. | Test (TC-2862) |
| FR-1012-AC-9 | An empty admitted population for a requested policy yields the no-instance outcome before exploration, while an empty policy list does not trigger that outcome. | Test (TC-2862) |
| FR-1012-AC-10 | A population marked `complete:false` refuses `incomplete_population` / `incomplete-scope` at `/request/synthesis/initial` before an unknown policy name is reported, exit `22`. | Test (TC-2860) |
| FR-1012-AC-11 | An omitted population required by a policy refuses `incomplete_population` / `missing-required-artifact` at `/request/synthesis/initial` after all policy names and `over` bindings resolve, exit `22`. | Test (TC-2860) |

## Dependencies

- FSpec FR-800, FR-802, FR-803, FR-804, FR-808 and FR-809 (plan search).
- FR-755 (native plan replay) and FR-125 "Instancing" (admitted keys).
