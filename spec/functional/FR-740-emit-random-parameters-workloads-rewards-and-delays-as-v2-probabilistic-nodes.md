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
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-230
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
  member, parameters the operation's parameters in declared order, random
  parameters included, result type the reward's value type, and clause kind
  `reward` (FR-094 `DeclaredClauseKind::Reward`, FSpec FR-322 and FR-640),
  never `precondition` or `body`, so the reward adds no guard to its
  operation and gives an operation without a body no body. For a reward on
  an operation of a source-declared type (FR-569), the function carries the
  unit's `SourceOwner` and the `declaration` `[<type>, <operation>,
  "reward", <reward name>]` (FSpec FR-322 "Source-declared object types"
  item 5) instead of a `ModelOwner`, and is keyed by that path at ordinal 0;
  the reward is emitted, never refused for its type's origin. The reward
  node's identity carries the content operand FSpec FR-640 "Reward identity"
  names (the key of the expression node the function's `body` references, or
  a literal body's text), and S3 lowers the rewards after every other clause
  function in ascending order of (operation key, reward name), so the
  reward functions' ordinals follow that lowering order and never the source
  order of any declaration.
- S4 SHALL emit a `probabilistic`/`delay_distribution` node for each checked
  delay declaration, with the family and its parameters converted exactly to
  the timed model's time unit.
- S4 SHALL emit each checked random parameter, workload and delay whatever
  the origin of the object type that declares its operation, or that a
  workload is on: an object type of a selected domain package or an object
  type the unit declares in its own source (FR-569). For a source-declared
  type, an operation reference's `declaring`, and a workload's `model`, SHALL
  reference that type's source `model`/`object_type` node, the node its
  source declaration key `ix://I/<Type>` names (`I` the unit's `SourceOwner`
  identity, FSpec FR-610 "Declaration keys"), and the operation is named by
  its name as a model operation is. A delay on an operation of the source
  model has its times in the unit of the source model's `dense` time member
  (FR-230 "The source model's time member"), whichever source object type
  declares the operation, because that time member times the whole source
  model and not only the type it names. S3 admits these declarations on the
  source model as on a selected model (FR-185, FR-253), so a package that
  dropped or refused them would leave a checked unit that no request can run.
- Each node SHALL carry one `declaration` occurrence whose source region is
  the declaration's label span, so a diagnostic about the node is located at
  the declaration.
- A checked claim `under W` SHALL resolve, when S4 emits it (FR-498), to
  `W`'s workload node, held in the claim node's `scheduler` binding and as an
  operand of its path (FSpec FR-640 "Probabilistic claim node"). The reward
  nodes its form reads and its subject model's random-parameter and delay
  nodes are held the same way. None of them SHALL be held in an
  emission-side table or a sidecar row.
- The repository SHALL carry, as worked examples of this emission, the rover
  and architecture probabilistic units of agent-ix/filament-examples commit
  `b6c0fdc`, vendored byte for byte under `examples/rover/` and
  `examples/architecture/` in the upstream `rules/`, `requests/` and `model/`
  subdirectories, so that each request's relative `file` paths resolve: the
  units `rules/staged-vote-probability.fsl` and
  `rules/staged-vote-probability-missed.fsl`, their `native-compile/1` requests
  `requests/staged-vote-probability.json` and
  `requests/staged-vote-probability-missed.json`, and the domain model
  `model/rover.semantic-ir.json`; and `rules/staged-availability.fsl`,
  `rules/staged-availability-missed.fsl`, `requests/staged-availability.json`,
  `requests/staged-availability-missed.json` and
  `model/architecture.semantic-ir.json`. Each vendored file keeps its MIT licence
  in a `.license` sidecar holding `SPDX-FileCopyrightText: 2026 Agent IX`,
  `SPDX-License-Identifier: MIT` and a `Source:` line naming the repository,
  path and commit, as the MIT fixtures under `fsl-analyze/tests/fixtures/rover/`
  do (whose `rover.semantic-ir.json` is already byte-identical to the one at
  `b6c0fdc`). A test compiles the four units through S4 from those files,
  each with the program `identity` its request names
  (`example:rover/staged-vote-probability`,
  `example:rover/staged-vote-probability-missed`,
  `example:architecture/staged-availability` and
  `example:architecture/staged-availability-missed`) as its unit package,
  and reads each package with the strict reader. Each unit is a formula-event claim
  under a workload over random parameters, which is how a modeller first
  writes a probability claim, so the examples keep that path compiling to a
  package that a provider can run.
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
| FR-740-AC-4 | A claim `under Steady` emitted with `Service` has a `scheduler` binding referencing `Steady`'s `workload` node and a path holding that node as an operand, and its `distributions` binding and path hold the two `Server::attempt` `random_parameter` nodes. The strict reader admits the package holding the claim, the random parameters, the reward and the workload. With a second workload `Bursty on Service`, a claim `under Bursty` references `Bursty`'s node, and a claim `under every scheduler` references no workload node. | Test (TC-2062) |
| FR-740-AC-5 | A unit that selects no domain package and declares `object O { d: Clock; operation step() modifies self.d; }`, `object Other { n: Integer; operation poke() modifies self.n; }`, `time O dense unit ms;`, `delay O::step ~ uniform;` and `delay Other::poke ~ discrete { 1 ms: 9, 3 ms: 1 };` checks and emits two `delay_distribution` nodes, one whose `declaring` references the source `model`/`object_type` node of `O` and one whose `declaring` references that of `Other`, the second with the delays `1` and `3` typed by the unit node of `ms`; the strict reader admits the package and the identity-row join admits both operation references. Without the `time` member, `delay O::step ~ uniform;` refuses `invalid_probabilistic_model`/`delay-on-untimed-model` at its span. | Test (TC-3012) |
| FR-740-AC-6 | A unit that selects no domain package and declares `object O { n: Integer; operation step(k: Int[1, 3]) modifies self.n; }`, `random O::step(k) ~ { 1: 1, 3: 3 };` and `workload W on O { weight O::step = 1; }` checks and emits one `random_parameter` node and one `workload` node whose `declaring` and `model` reference the source `model`/`object_type` node of `O`; the strict reader and the identity-row join admit the package, and FSL's reader rebuilds a `RandomParameter` with probabilities `1/4` and `3/4` and a `Workload` equal to S3's. | Test (TC-3012) |
| FR-740-AC-7 | `Service` emits the `duration` reward function with `clause` `"reward"`, owner the `Server::attempt` operation member and no `precondition` or `body` clause function built for it; the IR strict reader admits the package with `Server::attempt`'s `operation_bodies` row `no_body`, and FSL's reader rebuilds the reward with its clause kind `reward`. FSpec FR-640-AC-3's three declaration orders of `cost`, `load`, `stable`, the random parameter, the workload and the claim emit byte-identical packages, and changing `load`'s expression changes the `load` node id and no reward function id. FR-740-AC-6's unit with a reward `cost` on `O::step` returning `k` emits a `reward` node whose function carries the unit's `SourceOwner` and `declaration` `["O", "step", "reward", "cost"]`, and the strict reader admits it. | Test (TC-2062) |
| FR-740-AC-8 | The vendored `examples/rover/rules/staged-vote-probability.fsl`, compiled by its request `examples/rover/requests/staged-vote-probability.json` (unit package `example:rover/staged-vote-probability`, with `model/rover.semantic-ir.json` selected as `example/rover`), emits a package with three `random_parameter` nodes on `Rover::sample` (`faultA`, `faultB`, `faultC`, each with support `true` at weight 1 and `false` at weight 999), one `workload` node `Driving` with weight 1 on each of `sample`, `loseLidar`, `brake`, `halt` and `enterFailsafe`, and one `probabilistic_claim` node `VoteRarelyWrong` whose `subject` references the `example/rover` `model_root`, whose `scheduler` references `Driving`'s node, whose `reward` and `delays` are empty and whose `distributions` holds the three `random_parameter` nodes in ascending node-id order. `examples/architecture/rules/staged-availability.fsl`, compiled by its request (unit package `example:architecture/staged-availability`, `model/architecture.semantic-ir.json` selected as `example/architecture`), emits one `random_parameter` node on `Arch::Service::call` (`ok`, `true` at 999 and `false` at 1), one `workload` node `Traffic` and one claim node `CheckoutAvailable` with `scheduler` `Traffic`'s node, empty `reward` and `delays`, and `distributions` that one node. The strict reader admits both packages. The two `-missed` units, compiled by their requests (unit packages `example:rover/staged-vote-probability-missed` and `example:architecture/staged-availability-missed`), emit too, and the strict reader admits them. Their claim nodes bind the same kinds of node as the passing units': `subject` the same `model_root` (`example/rover`, respectively `example/architecture`, a node of the selected package and so equal across the two units), `scheduler` the unit's own `Driving`, respectively `Traffic`, `workload` node, empty `reward` and `delays`, and `distributions` the unit's own three, respectively one, `random_parameter` nodes. Each unit is its own package, so its `workload` and `random_parameter` nodes carry its own unit package in their `node_id`: compared by FSpec FR-322 structural keys (operand slot numbers aside), the `-missed` claim's path has the same shape and the same `subject` operand as the passing claim's, its `workload` and `random_parameter` operands differ from the passing claim's only in their `package`, and, for rover, its name segment is `VoteAlmostNeverWrong` in place of `VoteRarelyWrong`. Each claim is at ordinal 0. The two `CheckoutAvailable` thresholds differ, and the threshold is not yet a body member or path operand (FSpec FR-640 "Open"), so this criterion asserts nothing about whether the two are one obligation. | Test (TC-2062) |

## Dependencies

- [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md) and
  [FR-253](FR-253-check-delay-distributions-and-define-the-race.md) supply the
  checked declarations; FSpec FR-640 and FR-322 own the node bodies, identity
  and reader checks; [FR-187](FR-187-give-model-transitions-step-probabilities-and-rewards.md)
  reads the rebuilt records.
