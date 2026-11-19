---
id: FR-498
title: "Carry a probabilistic claim's checked-package identity"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-012
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-203
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-235
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-421
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-640
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-740
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-195
    type: references
---
# FR-498: Carry a probabilistic claim's checked-package identity

## Description

FSpec FR-421 reports its `timed-form-shape{locus}` and `punctual-interval{locus}`
causes against a probabilistic claim's exact `{declaration NodeRef, occurrence}`
resolved through `checked_term_data.probabilistic_claims` (FSpec FR-322-AC-297).
[FR-186](./FR-186-check-probabilistic-claim-forms-at-s3.md)'s
`CheckedProbabilisticClaim` holds only a `name`, a `span` and a normal form,
neither of which is an identity (ADR-013 R-04), so the exact plan cannot name
the claim of its exact timed verdicts (FR-204) or the locus of its
`PunctualInterval` (FR-235) cause; QSL-1416 records that gap. This requirement owns carrying the claim's
checked-package identity from the producer to the carrier and to those causes.

## Inputs

- A checked probabilistic claim (FR-186).
- The checked-package producer's decision to emit that claim in the local
  checked semantic graph.

## Outputs

- For each emitted checked probabilistic claim, one local `claim` node with
  `semantic_form` `probabilistic_claim` whose body and path carry what the
  claim reads, one declared `claim` occurrence,
  exactly one corresponding `checked_term_data.probabilistic_claims` row of
  FSpec FR-322, and a `CheckedProbabilisticClaim` that exposes that same
  producer-owned identity.

## Behavior

- For each `CheckedProbabilisticClaim` included in an emitted package, the
  producer SHALL emit exactly one local `claim` node whose `semantic_form` is
  `probabilistic_claim`.
- For each such node, the producer SHALL declare exactly one occurrence with
  role `claim`.
- The node's `node_id` SHALL be the FSpec FR-322 structural `NodeRef`
  `{package, path, ordinal}` the producer assigns when it lowers the checked
  claim: `package` is the unit's package, `path` is FSpec FR-640's
  "Probabilistic claim node" path, `["claim", "probabilistic_claim", N, …]`
  for the claim's declared name `N` followed by the nodes the claim reads as
  operand segments (its subject's `model_root`, its workload or its fairness
  set, the reward nodes its form reads, and its subject model's
  random-parameter and delay nodes), and `ordinal` ranks the claims of that package and path by their written
  content, ascending, from 0. A claim's written content is the sequence of
  the exact spellings of its significant tokens, from its leading
  `probabilistic` keyword through its closing `}`, its declared name
  included; whitespace and comments are trivia and are excluded, and no
  span or source position enters it. Two contents compare element by
  element, each spelling as UTF-8 bytes, a proper prefix sorting first, as
  FSpec FR-322 orders node-id paths; claims with equal contents take
  consecutive ordinals.
- The node SHALL carry no `declaration` member: its path is the derived-node
  shape `[node_tag, semantic_form, ...operands]`, not a declaration path, and
  the claim is located by its one `claim` occurrence.
- The claim's identity SHALL cover what the claim reads: changing a weight of
  its workload, a support value or weight of a random parameter of its
  subject's model, a delay of that model or the expression of a reward its
  form reads changes the operand node, so the claim's `node_id` changes too.
  Claims that read none of the changed nodes keep their `node_id`. Two
  claims that differ only in what they read have different paths, so each
  takes ordinal 0.
- Equal names or equal spans SHALL NOT merge two claim declarations: each
  checked claim has its own node, which the `ordinal` distinguishes. Two
  claims with equal names and equal written content are indistinguishable,
  so permuting them yields identical output and any assignment of their
  ordinals between them conforms. Unlike the content-addressed claim nodes
  of FR-337, FR-410 and FR-454, identical probabilistic claims get two nodes,
  because FSpec FR-322-AC-297 (`PC-R4`) admits one row per declaration.
- Moving a claim declaration within its source, reformatting or commenting
  it, or reordering the unit's declarations SHALL leave every claim's
  `{declaration, occurrence}` unchanged.
- The node's body SHALL be FSpec FR-640's `probabilistic_claim` body, the five
  bindings `subject`, `scheduler`, `reward`, `distributions` and `delays` in
  that order, built from the checked claim (FR-186, FR-195) and the emitted
  `probabilistic` nodes (FR-740). `subject` references the `model_root` of the
  claim's subject. `scheduler` references `W`'s `workload` node for `under W`;
  for `under every scheduler` it holds the claim's fairness set.
- `reward` SHALL hold the `reward` nodes, on operations of the subject's
  model, of the one reward the claim's form reads, and SHALL be empty for
  every other form. The forms that read a reward `R` are
  `BoundedMeasure::Accumulate` (in `Event::Compare` or `Quantile`), a
  `FractionMeasure` or `LongRunFraction` whose `weight` is `Weighting::Reward(R)`, and
  `ExpectedReward`. `Steps`, `Elapsed`, an unweighted fraction,
  `Event::Formula`, `Reach` and `MeanTime` read none. A fraction or long-run
  fraction whose `weight` is `Weighting::Delay` (FR-186, FSpec FR-407's delay
  weighting) reads the subject's delays, which `delays` already holds, and no
  reward.
- `distributions` and `delays` SHALL hold every `random_parameter`, respectively
  `delay_distribution`, node of the subject's model, whatever the scheduler.
- The producer SHALL emit the claim node after the `model_root` and every
  `probabilistic` node it references. A read workload or reward with no
  emitted node is a `runtime_invariant` refusal naming the claim and the
  workload or reward. The producer holds what a claim reads only in the claim
  node's body and path, never in an emission-side table or a sidecar row.
- The producer fabricates no formula or bound: the form, threshold,
  activation, `over` binding and confidence parameters stay in the written
  content that ranks the ordinal.
- For each such node, the producer SHALL emit exactly one
  `checked_term_data.probabilistic_claims` row `{declaration, occurrence}`.
  Its `declaration` SHALL be the node's structural `NodeRef`.
- The row's `occurrence` SHALL be the node's declared `claim` occurrence.
- The row, the node and the associated `CheckedProbabilisticClaim` SHALL carry
  the same identity. Rows are in ascending structural `NodeRef` order, then
  occurrence order. The row repeats no formula, bound or other claim body, and
  no row names another claim form.
- The producer SHALL expose that same identity through the associated
  `CheckedProbabilisticClaim` to every consumer that holds it. No consumer
  selects or joins a claim's identity by its name, span, display text or
  declaration order; it reads the exposed `{declaration, occurrence}`.
- An emitted package SHALL NOT carry one of these probabilistic claim nodes
  without exactly one corresponding row and checked claim, nor carry a row or
  checked claim without exactly one corresponding node and occurrence. No
  protocol-location row is substituted for a claim row.
- An exact timed verdict (FR-204) SHALL be reported against, and the
  `PunctualInterval` cause (FR-235) SHALL carry, the claim's FR-331 locus `{node, occurrence}` (`node` the claim's declaration `NodeRef`). Where FSpec
  defines a `probabilistic_claim` body, the cause SHALL also carry the member
  path of the unsupported form or interval operator within that body, as
  FSpec FR-421 states its loci. A claim with no identity (one checked outside an assembled package) SHALL keep the
  refusal that carries the claim's `span` as its locus (FR-186) and SHALL NOT
  report a package locus built from the span.
- The producer SHALL assign the claim node identities when it lowers the
  checked claims, before it writes any `probabilistic_claims` row: each row names
  the identity already associated with its emitted claim node, and writing
  the rows mints or remints no graph node.
- FSL emission SHALL NOT claim reader admission; FSpec FR-322 and TC-233 own the
  reader refusals.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-498-AC-1 | A unit with two checked probabilistic claims that share a declared name and differ in written content (the FSL counterpart of FSpec FR-322-AC-297 `PC-01` and `PC-02`; one source cannot give two declarations equal spans), and a third claim with another name, emits exactly three local `claim` nodes with `semantic_form` `probabilistic_claim`, each with one declared `claim` occurrence, and exactly three matching `probabilistic_claims` rows with distinct declaration `NodeRef`s in ascending structural order. Every permutation of the three declarations keeps each claim's `{declaration, occurrence}` and emits byte-identical rows. | Test (TC-1288) |
| FR-498-AC-2 | Each emitted claim node, its `probabilistic_claims` row and its associated `CheckedProbabilisticClaim` expose the same `{declaration, occurrence}`. Renaming a claim whose name no other claim shares changes its identity and no other claim's (renaming or editing a member of a same-named group may re-rank the rest of its group); moving, reformatting or commenting a claim leaves its written content, and so its identity, unchanged; changing a token of it changes its written content. Two claims written identically are two declarations with distinct identities. | Test (TC-1288) |
| FR-498-AC-3 | An elapsed-time claim and a punctual-interval claim over a timed subject each settle an exact verdict (FR-204), never `unsupported`, reported against the selected claim's FR-331 locus `{node, occurrence}`, whose `node` is the claim's declaration `NodeRef` (FSpec FR-421-AC-7); a `PunctualInterval` cause on the zone route (FR-235) carries the same locus; for two claims with equal names the result names the selected claim only. | Test (TC-1288) |
| FR-498-AC-4 | An emitted package has a one-to-one correspondence among each included `CheckedProbabilisticClaim`, one local `claim`/`probabilistic_claim` node with one declared `claim` occurrence, and one matching sidecar row; it has no unmatched node, row or checked claim, every row names a node already in the emitted graph with that node's own occurrence, and the strict reader admits the package. | Test (TC-1288) |
| FR-498-AC-5 | `Service` (FR-740-AC-1) with a second workload `Bursty on Service` and claims `A` (`under Steady`), `B` (`under Bursty`) and `C` (`under every scheduler`), each `probability >= 0.5 [always[0,2] holds(true)]`, emits three claim nodes whose `subject` references the `example/service` `model_root`, whose `scheduler` binding and path operand name `Steady`'s node, `Bursty`'s node, and an empty fairness set (path `"every"` with no member), whose `reward` is empty and whose `distributions` are the two `Server::attempt` nodes. Renaming `A` to `B` gives two claims named `B` with different paths, each at ordinal 0. Changing one weight of `Steady` changes `A`'s `node_id` and row and leaves `B`'s and `C`'s byte-identical; changing one support weight of `d` changes all three. Writing `Steady`'s weight lines, and the three claims, in another order changes no byte. The strict reader admits each package. | Test (TC-1288) |
| FR-498-AC-6 | Over `Service` with `reward duration on Server::request = 1 ms;` added, a claim of each form emits the `reward` binding of "Behavior": the two `duration` nodes, in ascending node-id order, for `probability >= 0.5 [accumulate duration from holds(s.phase = Busy) until holds(s.phase = Done) within 10 <= 5 ms]`, ADR-024's `P95`, `probability >= 0.5 [fraction holds(s.phase = Busy) over [0, 10] weighted by duration >= rational(1, 2)]`, `mean of (fraction holds(s.phase = Busy) over [0, 10] weighted by duration) >= 0.5`, `long_run_fraction holds(s.phase = Busy) weighted by duration >= 0.5` and `expected accumulate duration until holds(s.phase = Done) <= 3 ms`; an empty `reward` for `probability >= 0.5 [steps from holds(s.phase = Busy) until holds(s.phase = Done) within 10 >= 3]`, the same fraction and long-run fraction without `weighted by duration`, `[always[0,2] holds(true)]` and, under the infinite-trace profile, `[eventually holds(s.phase = Done)]`. Rewriting `duration` on `Server::request` to `2 ms` changes exactly the first group's claim `node_id`s. Over FR-740-AC-3's `time dense` unit with `delay Call::reply ~ discrete { 1 ms: 90, 3 ms: 10 };`, `expected elapsed until holds(P) <= 3 ms`, `quantile 0.5 of (elapsed from holds(P) until holds(Q) within 5 ms) <= 3 ms` and `long_run_fraction holds(P) weighted by delay >= 0.5`, for state predicates `P` and `Q` of that unit, each emit an empty `reward` and `delays` holding the one `delay_distribution` node, and changing the delay's weight 10 to 20 changes all three `node_id`s. The strict reader admits each package. | Test (TC-1288) |

## Dependencies

- **Upstream:** FR-186 defines `CheckedProbabilisticClaim`; FR-204 owns the
  exact timed verdicts and FR-235 owns `PunctualInterval`, the results this wires; FSpec FR-322 (FR-322-AC-297), TC-233 and FR-421 own the
  carrier, its reader checks and the locus.
- **Resolved here:** this requirement defines emission of the local
  `claim`/`probabilistic_claim` node, its structural `NodeRef` (path and
  content-ranked ordinal) and its one-to-one association with the checked
  claim and FR-322 sidecar row.
- **Resolved by FSpec FR-640:** the `probabilistic_claim` body and path that
  carry what the claim reads.
- **Open:** the causes' member path within the claim body waits for FSpec to
  define the form members of the `probabilistic_claim` body; until then the
  causes carry the `{node, occurrence}` locus alone.
