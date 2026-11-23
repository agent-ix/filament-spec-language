---
id: SR-967175026302197768900474181941935084802
title: "gap-analysis of PR 3589: zero-delay cycle traceability"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-analyze/src/exact_probabilistic/finite_horizon.rs (PR 3589 head 4f5e22b337246d1ebd271225639a6d1c34d0aebc); FR-197-AC-1..4, FR-204-AC-4, FR-204-AC-16"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: references
---

## Summary

Ticket: QSL-2881. This is a planless gap audit, scoped to the PR diff and the criteria it
touches. `quire matrix --scope . --format json` reports the following criteria as tagged:

| Criterion | Tagged in |
| --- | --- |
| FR-197-AC-1 | exact_probabilistic.rs:635 |
| FR-197-AC-2 | exact_probabilistic.rs:488 |
| FR-197-AC-3 | exact_probabilistic.rs:242 |
| FR-197-AC-4 | finite_horizon.rs:1667 and 1690 |
| FR-204-AC-4 | settle.rs, digital_clocks.rs, exact_probabilistic.rs, outcome.rs |
| FR-204-AC-16 | digital_clocks.rs:1379 |

The two new tests carry the bare tag `TC-632` and bind to no criterion.

## Verdict

CONDITIONAL. No scoped criterion is untagged and no stubs were found. One trace finding and one
spec gap remain: the ticket's acceptance asks that a requirement state the outcome when the shape
is reachable, and the shape in code-review FND-001 is reachable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new tests carry `TC-632`, whose scope is FR-197-AC-1..4 (finite-horizon values), but they verify zero-delay-cycle decisions (FR-204-AC-16 / TC-2175). They bind to no criterion | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:1797-1818 |
| FND-002 | medium | No FR-204 criterion states the outcome for a leaky zero-delay cycle outside every end component when the product also has one. That case is reachable (code-review FND-001) and today it settles as an internal fault | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:251 |

## Coverage

- Matrix: FR-197-AC-1..4, FR-204-AC-4 and FR-204-AC-16 are all tagged. The new tests add no binding.
- Reverse gap: the diff adds no production behavior, only a comment and tests.
- Stubs and coverage inflation: none. Both tests assert exact values. Their oracles are weak (code-review FND-004).
- Semantic review: skipped. The reviewer brief scoped this review; the code-review lane covers intent for the diff.
- Plan completion: not assessed
