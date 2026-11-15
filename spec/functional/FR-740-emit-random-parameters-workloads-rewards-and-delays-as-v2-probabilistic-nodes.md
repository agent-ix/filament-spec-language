---
id: FR-740
title: "Emit random parameters, workloads, rewards and delays as v2 probabilistic nodes"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-022
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-253
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-498
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-187
    type: references
  - target: ix://agent-ix/filament-specification/FR-640
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
---
# FR-740: Emit random parameters, workloads, rewards and delays as v2 probabilistic nodes

## Description

S4 SHALL write each checked random parameter, workload and reward (FR-185)
and each checked delay declaration (FR-253) into the
`filament.checked-semantic-graph/v2` graph as the FSpec FR-640
`probabilistic` node of its form, and FSL's checked-package reader SHALL read
those nodes back into the checked records FR-187 and the engines use. A unit
that draws a value, weights its operations, rewards a step or samples a delay
therefore compiles to a checked package like any other unit, and every
probability engine, replay and certificate checker can run from that package.

## Use case

A modeller compiles a rover unit with `random Rover::sample(faultA) ~ { true:
1, false: 99 };`, a workload `Driving on Rover` and a reward `cost`. S4 emits
the package with its two `random_parameter` nodes, its `workload` node and
its `reward` node; `fsl analyze` then runs the vote claim from that package.

## Inputs

- A checked package (S3) with FR-185's `RandomParameter`, `Workload` and
  `Reward` records and FR-253's checked delay declarations.

## Outputs

- The emitted package with one `probabilistic` node per declaration, and the
  reader's reproduction of the checked records from it.

## Behavior

- S4 SHALL emit a `probabilistic`/`random_parameter` node for each
  `RandomParameter`, with the operation reference of its operation, the
  parameter's name and its support in declared order, each value with its
  weight as written reduced to lowest terms (FSpec FR-640). The emitted node
  carries weights, not probabilities.
- S4 SHALL emit a `probabilistic`/`workload` node for each `Workload`, with a
  reference to its model's node, its name and one weight member per
  operation, weights of 0 included, in FSpec FR-640's operation order.
- S4 SHALL emit a `probabilistic`/`reward` node for each `Reward`, with the
  operation reference, the name, a reference to the reward's value type node
  and a reference to a clause `function` node it lowers the reward's checked
  expression into, as FR-094 lowers a clause function: owner the operation
  member, parameters the operation's parameters in declared order, result
  type the reward's value type.
- S4 SHALL emit a `probabilistic`/`delay_distribution` node for each checked
  delay declaration, with the family and its parameters converted exactly to
  the timed model's time unit.
- Each node SHALL carry one `declaration` occurrence whose source region is
  the declaration's label span, so a diagnostic about the node is located at
  the declaration.
- A checked claim `under W` SHALL resolve, when S4 emits it (FR-498), to
  `W`'s workload node.
- The reader SHALL admit the emitted package under FSpec FR-640's reader
  checks and rebuild from it `RandomParameter`, `Workload`, `Reward` and the
  delay records equal to S3's, each probability recomputed exactly from the
  weights, so FR-187's `ModelSystem` over the read package gives the same
  actions and step probabilities as over S3's checked package.
- Emission SHALL be deterministic: the same checked package emits the same
  bytes, and reordering a workload's weight lines or the unit's declarations
  changes no node's bytes.
- A failure to emit one of these nodes SHALL refuse the package with the
  declaration's source locus in the diagnostic's locus member (FR-185
  "Package emission").

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-740-AC-1 | `Service` (FR-185-AC-1) compiles through S4 to a package holding two `random_parameter` nodes on `Server::attempt` (`d` with supports `1`, `3`, `12` and weights `90`, `7`, `3`; `outcome` with `Ok` and `Fail` at `98` and `2`), one `reward` node `duration` whose function returns `d`, and one `workload` node `Steady` with weight 1 per operation; each node has one `declaration` occurrence at its declaration's label span; the strict reader admits the package and rebuilds records equal to S3's, with probabilities `9/10`, `7/100`, `3/100`, `49/50`, `1/50`. | Test (TC-2062) |
| FR-740-AC-2 | Over the read package, `ModelSystem::weighted_steps` at FR-187-AC-1's state under `Steady` returns the six steps and probabilities FR-187-AC-1 states. | Test (TC-2062) |
| FR-740-AC-3 | A `time dense` unit with `delay Call::reply ~ discrete { 1 ms: 90, 3 ms: 10 };` emits a `delay_distribution` node of family `discrete` and the reader admits it; a workload with `Server::reset: 0` emits weight 0; writing `Steady`'s weight lines in another order emits byte-identical output. | Test (TC-2062) |
| FR-740-AC-4 | A claim `under Steady` emitted with `Service` resolves its scheduler to the `Steady` workload node, and the package holding the claim, the random parameters, the reward and the workload is admitted by the strict reader. | Test (TC-2062) |

## Dependencies

- [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md) and
  [FR-253](FR-253-check-delay-distributions-and-define-the-race.md) supply the
  checked declarations; FSpec FR-640 and FR-322 own the node bodies, identity
  and reader checks; [FR-187](FR-187-give-model-transitions-step-probabilities-and-rewards.md)
  reads the rebuilt records.
