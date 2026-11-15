---
id: FR-628
title: "Request possible, always-possible and unique-path state-graph items through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-165
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-166
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-168
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-169
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-170
    type: depends_on
---
# FR-628: Request possible, always-possible and unique-path state-graph items through analyze

## Description

A `native-analyze/1` item naming a checked state-graph claim (`possible`,
`always possible` or `unique path`, FR-165) SHALL be decided over the
explored state graph (FR-167, FR-168) and report its verdict, witness or
counterexample and certificate in the FR-621 members.

## Inputs

### Request item

- `claim`: the declared name of a state-graph claim; `kind`
  `temporal-satisfaction`, `item_kind` `formula`.
- `temporal.subject`, `temporal.limits` (FR-625's members) and
  `temporal.witness_samples` with `temporal.seed`, the number of sampled
  witnesses for a `possible` claim and their seed (FR-166).
- Negotiation routes the item only to a backend whose manifest lists the
  claim's form in `state_graph_forms` (FSpec FR-390).

## Outputs

- `record`: `proved` or `refuted` with FR-169's settlement method; a proof
  carries the state-graph certificate.
- `counterexample` for a refuted `always possible` claim: a path to a state
  from which no goal state is reachable; for a refuted `unique path` claim:
  two distinct paths. A `possible` proof carries its witness path and the
  sampled witnesses in the record's evidence, each replayed (FR-170).

## Behavior

- **Budgets.** An exploration stopped by a limit with open nodes settles
  `incomplete` with the limit, unless the explored graph already decides the
  claim (FR-169), which settles it.
- **Cancellation** is FR-625's.
- **Refusals.** A form no candidate advertises settles `unsupported` with
  `unsupported-requested-capability`; `witness_samples` on a claim that is
  not `possible` refuses the request `invalid-request` naming the member.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-628-AC-1 | Over the Counter fixture, `possible (value = 3)` settles `proved` with a witness path of three steps that replays; `possible (value = 4)` settles `refuted`. | Test (TC-1588) |
| FR-628-AC-2 | `always possible (value = 0)` settles `refuted` with a path to `value = 1`, from which no step returns to 0. | Test (TC-1588) |
| FR-628-AC-3 | `temporal.witness_samples` 3 with seed 7 gives three sampled witnesses, equal across two runs. | Test (TC-1588) |

## Dependencies

- FR-165 to FR-170 own state-graph claims and their settlement.
