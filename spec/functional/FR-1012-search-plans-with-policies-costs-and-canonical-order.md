---
id: FR-1012
title: "Search plans with exact costs, policy monitors and the canonical plan order"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-800 filament-specification/FR-802 filament-specification/FR-803 filament-specification/FR-804 filament-specification/FR-808 filament-specification/FR-809 filament-spec-language/FR-755
    type: depends_on
---
# FR-1012: Search plans with exact costs, policy monitors and the canonical plan order

## Description

FSL's plan search in `fsl-analyze` SHALL implement FSpec FR-800 to FR-804, FR-808 and FR-809: search nodes keyed by the canonical state key, monitor states and accumulators; exact objectives classified as per-step sums or general; safety monitors that prune and non-safety policies evaluated on each completed plan's behaviour; the canonical plan order; delays over the region abstraction; and exact probabilistic outcome distributions.

## Behavior

- The search SHALL be a separate module from `fsl-replay`'s proof-composition planner and from the verification planner.
- The reported plan and every count SHALL be those of FSpec FR-801's canonical sequential exploration ("Exploration order": layers by step count, nodes within a layer by accumulated per-step sum then representative prefix in FSpec FR-804's step order, successors in step order, and its fixed charge order), whatever the worker count.
- Over a timed grammar the search SHALL implement FSpec FR-808: tick steps of delay 0, dense-time steps of an exact delay then an invocation over FSpec FR-418's region abstraction, concrete delays at the least point or the open-bound midpoint, and an unattained infimum reported `feasible` with its exact `bound`; every `delay` and `elapsed` is written in FR-650's run value form.
- Over a probabilistic grammar the search SHALL implement FSpec FR-809: open-loop actions with drawn arguments, adversarial choice, nodes holding the exact state distribution, and `probability` and the expected objective computed as exact rationals.
- Every reported plan SHALL replay natively before it is written (FR-755).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1012-AC-1 | The spine plan under the policy `eventually holds(c.versionNumber = 3)` is evaluated, not refused, and every reported plan visits version 3. | Test (TC-2862) |
| FR-1012-AC-2 | Two runs of one plan request give byte-identical outcome documents. | Test (TC-2862) |
| FR-1012-AC-3 | A decimal per-step cost objective is written exactly in the run value form and certified with a `potential` certificate. | Test (TC-2862) |
| FR-1012-AC-4 | Over FSpec's published `timed-tick` fixture the plan's `elapsed` is the Quantity `30 min` with every `delay` 0 (FSpec FR-808-AC-1); over `timed-go` with guard `x >= 2` the result is `optimal` with one step of delay the Rational `2/1` and an accepted `closure` certificate (FR-808-AC-2), and with guard `x > 2` `feasible` with `bound` `{side: lower, value: 2/1, attained: false}` and delay `5/2`, byte-identical across two runs (FR-808-AC-3, AC-5). | Test (TC-2862) |
| FR-1012-AC-5 | Over FSpec's published `probabilistic-try` fixture the goal `Reach.final.done` gives `optimal` `retry` (FSpec FR-809-AC-1), and `maximize Reach.probability` over lengths 1 to 2 using only `try` gives `optimal` `try, try` with `objective.value` the Rational `3/4` (FR-809-AC-2). | Test (TC-2862) |
| FR-1012-AC-6 | Over FSpec's published `counter` fixture, `limits` `{"max_states": 2}` gives `incomplete` with `explored.states` 2 and `explored.work` 6 and plan `null`, and `{"max_work": 1}` gives `explored.states` 1 and `explored.work` 1 (FSpec FR-801-AC-8), each byte-identical across two runs. | Test (TC-2862) |

## Dependencies

- FR-800 filament-specification/FR-802 filament-specification/FR-803 filament-specification/FR-804 filament-specification/FR-808 filament-specification/FR-809 filament-spec-language/FR-755 (filament-specification).
