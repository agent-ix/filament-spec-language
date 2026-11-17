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

When a `native-analyze/1` item names a checked state-graph claim
(`possible`, `always possible` or `unique path`, FR-165), FSL SHALL decide the
claim over the explored state graph (FR-167, FR-168). The FR-621 members of the
item carry its verdict, witness or counterexample and certificate.

## Inputs

### Request item

- `claim`: the declared name of a state-graph claim; `kind`
  `temporal-satisfaction`, `temporal.item_kind` `formula`.
- `temporal.subject`, `temporal.limits` (FR-625's members) and
  `temporal.witness_samples` with `temporal.seed`, the number of seeded walks
  phase 0 draws for a `possible` claim per instance and initial state, and
  their seed (FR-166). `witness_samples` bounds the walks, not the witnesses:
  FR-166 keeps the witness of the lowest-index walk that found one, so an
  instance has at most one sampled witness per initial state.
- `claim` names a checked claim. A target that the checker refuses, such as
  `c.value = 4` over a field typed `Int[0, 3]` (FR-104), refuses the program
  at check, so it never becomes an item; an unreachable target is written
  with values inside the field's type.
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
  `incomplete` with `LimitReached{limit}`, unless the explored graph already
  decides the claim (FR-169), which settles it, or the item is a `possible`
  claim holding a witness for every initial state. That item settles
  `inconclusive` with `well-definedness-unchecked` (FR-166, FR-169): a
  witness exists, but the stopped run could not rule out a reachable
  undefined evaluation of the target, which would refute it (FSpec FR-391),
  so the record says a witness was found and the claim is unsettled rather
  than that nothing was learned. An `always possible` or `unique path` item
  has no witness that settles it early, so a stop leaves it `incomplete`.
- **Evidence encoding.** The item's evidence is FSpec FR-393's
  `GraphEvidence`. Its `instance.key` is the bound object's universe key in
  FSpec FR-181 typed canonical form, never an object-reference JSON value,
  so two runs over the same universe write the same key and a replay
  matches the instance by key equality. The `undefined` arm's `where`
  carries `position`, `state_key` and `locus`, the source locus of the
  expression that has no value, with `predicate` or `transition` as FSpec
  FR-393 states, so the evidence replays on its own.
- **Cancellation** is FR-625's.
- **Refusals.** A form no candidate advertises settles `unsupported` with
  `unsupported-requested-capability`; `witness_samples` on a claim that is
  not `possible` refuses the request `invalid-request` naming the member.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-628-AC-1 | Over the Counter fixture, `possible (value = 3)` settles `proved` with a witness path of three steps that replays; over the Counter started at `value = 1`, `possible (value = 0)` settles `refuted`, since `inc` only increases the value. | Test (TC-1588) |
| FR-628-AC-2 | `always possible (value = 0)` settles `refuted` with a path to `value = 1`, from which no step returns to 0. | Test (TC-1588) |
| FR-628-AC-3 | `temporal.witness_samples` 3 with seed 7 on `possible (value = 3)` over the Counter fixture (one instance, one initial state) gives exactly one sampled witness, whose source names seed 7 and a trace index below 3; the record names `witness_samples` 3 and seed 7; two runs give equal evidence. | Test (TC-1588) |
| FR-628-AC-4 | The evidence of AC-1's proof has `instance` `{"type": "instance", "parameter": "c", "key": K}`, `K` counter `a`'s universe key in FSpec FR-181 typed canonical form, never an object reference; a `possible` claim whose target divides by `3 - c.value` settles `refuted` with an `undefined` arm whose `where` carries `position`, `state_key`, `predicate` `"P"` and the `locus` of the division. | Test (TC-2291) |
| FR-628-AC-5 | With `temporal.limits.max_states` 2, `possible (value = 3)` with the default `witness_samples` settles `inconclusive` `well-definedness-unchecked`, never `proved` and never `incomplete`; `always possible (value = 0)` under the same limit settles `incomplete` `LimitReached{max_states}`. | Test (TC-2291) |

## Dependencies

- FR-165 to FR-170 own state-graph claims and their settlement.
