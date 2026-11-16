---
id: FR-861
title: "Take the partial-value, repair, completeness and activation authority rules from filament-observation"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-251
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-502
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-524
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-526
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-007
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-008
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-010
    type: depends_on
---
# FR-861: Take the partial-value, repair, completeness and activation authority rules from filament-observation

## Description

`filament-observation` owns the observation-authority rules a temporal
monitor relies on: partial values and the order of uncertain event-time
intervals (its FR-007), activation-scope, progress and completeness
authority (its FR-008), and the affected-region plan of a revision repair
(its FR-010). FSL SHALL hold exactly one implementation of each of those
four rule sets, and that implementation SHALL be `filament-observation`'s.
`fsl-eval` SHALL call the owner's `partial`, `completeness`, `activation`
and `repair` authority modules where it applies those rules, as
`fsl-analyze` already calls the owner's `clock` and `progress` modules
(FR-251), and SHALL hold no second copy of them.

The temporal meaning stays FSL's. FR-501 (finite-window evaluation and its
counter cover), FR-502 (the progress ledger and its contradictions), FR-524
(instances, captures and activation dispositions) and FR-526 (the result
chain) state what the evaluator reports, and the owner's rules are the means
it uses to decide the observation-side facts those requirements read.

### Decision: one owner, in filament-observation

Two implementations of the same authority rules drift, and an observation
fact accepted by one and refused by the other gives two answers for one
trace. The rules belong to the observation authority, which already
publishes them as validated contracts
(`filament.observation.partial-fact/v1`,
`filament.observation.completeness-assertion/v1`,
`filament.observation.activation-scope-authority/v1`,
`filament.observation.repair-plan/v1`), and FSL already depends on its
`clock` and `progress` modules. Keeping the owner there and deleting FSL's
copy leaves one implementation that every consumer shares. The edge it
needs, `fsl-eval` to `filament-observation`, adds no cycle:
`filament-observation` depends only on `fsl-foundation`, `filament-exact`,
`quire-canonical` and serialization crates, each below layer 5 (ADR-011).
FSL's crates are pre-release, so the superseded copy is deleted with no
shim and no re-export.

## Inputs

The observation facts the evaluator reads: a trace's stamps and cover
(FR-501), its progress and completeness assertions (FR-502), its
deliveries, origins and capture facts (FR-524), and a superseding or
invalidating request's prior results (FR-526).

## Outputs

The evaluator's reports, unchanged: FR-501's, FR-502's, FR-524's and
FR-526's.

## Behavior

- `fsl-eval` SHALL name `filament-observation` in its `[dependencies]`
  (ADR-011's X-8 row lists it), at `branch = "main"`, through the workspace
  patch that keeps one `fsl-foundation` in the graph.
- Where the evaluator orders two uncertain event-time intervals or derives
  a partial value, it SHALL use the owner's `partial` rules.
- Where it accepts, revises or refuses a completeness assertion, it SHALL
  use the owner's `completeness` rules.
- Where it decides an activation scope's disposition and execution and
  evidence states, it SHALL use the owner's `activation` rules.
- Where it decides which retained results a revision invalidates, it SHALL
  use the owner's `repair` rules.
- No FSL crate SHALL hold a second implementation of any of the four rule
  sets: no type or function that restates the owner's partial-value
  algebra, completeness revision rule, activation-authority states or
  affected-region planning.
- While FSL still holds its own implementation, the agreement tests SHALL
  run it and the owner's on the same cases. A case on which they disagree is a defect, reported by
  a failing test that names the case and both answers. Neither answer is
  preferred silently: the switch lands only once the disagreement is
  resolved, by correcting the FSL requirement's text or the owner's.
- Every existing temporal test in `fsl-eval` and `fsl-analyze` SHALL pass
  unchanged after the switch.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-861-AC-1 | On FR-501-AC-13's counter-cover cases (`Counter { uncertainty: 1 }`, request at tick 0, `Complete { through: 3002 }`, watermarks 3001 and 3002), the interval order and partial value the evaluator uses equal those the owner's `partial` module derives for the same stamps. | Test (TC-2442) |
| FR-861-AC-2 | On FR-502-AC-2's cases (`Complete { through: 100 }` retained, then `Complete { through: 60 }`, `Incomplete` and an uncovered position stamped 80), the owner's `completeness` module accepts and refuses exactly what the evaluator's ledger accepts and refuses. | Test (TC-2442) |
| FR-861-AC-3 | On FR-524-AC-27 to AC-29's scopes and FR-526-AC-2's superseding request, the owner's `activation` module gives the dispositions the evaluator reports, and its `repair` module invalidates exactly the retained result the evaluator's chain supersedes. | Test (TC-2442) |
| FR-861-AC-4 | After the switch, `fsl-eval`'s manifest names `filament-observation`, its source calls the owner's `partial`, `completeness`, `activation` and `repair` modules, and no FSL crate defines a type or function carrying one of the four rule sets of its own; a disagreement case of AC-1 to AC-3 fails its test naming the case and both answers instead of passing on either one. | Test (TC-2442) |

## Dependencies

- ADR-011 (the crate graph and its X-8 row), FR-251 (the existing use of the
  owner's `clock` and `progress`).
- FR-501, FR-502, FR-524, FR-526: the evaluator behavior the owner's rules
  serve.
- filament-observation FR-007, FR-008, FR-010: the owned rules.
