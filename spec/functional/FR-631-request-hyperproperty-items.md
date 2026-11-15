---
id: FR-631
title: "Request hyperproperty and step-relation items through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-163
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-172
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-174
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-176
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-177
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-178
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-179
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-182
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-183
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-184
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-488
    type: depends_on
---
# FR-631: Request hyperproperty and step-relation items through analyze

## Description

A `native-analyze/1` item naming a hyper clause or a step relation SHALL be
decided in the form FR-173 classifies it into: 2-copy and k-copy
self-composition (FR-176), copy-swap reduction (FR-174), forall-exists by
witness sets (FR-177), projection-aligned (FR-178), single existential, or a
step relation over reachable transitions (FR-179). It reports its verdict,
its k-trace counterexample and its certificate in the FR-621 members.

### Decision: a hyper clause with no object parameter is emitted and analyzed

A hyper `temporal` clause with no object parameter quantifies over the
behaviours of the whole subject. It SHALL be emitted as a v2 hyperproperty
node (FR-488) like any other hyper clause and be analyzable through this
item; checking it without emitting it would leave a checked claim no request
can name.

## Inputs

### Request item

- `claim`: the hyper clause or step relation's name; `kind`
  `temporal-satisfaction`, `item_kind` `formula`.
- `temporal.subject` as FR-625; every copy reads the same subject.
- `temporal.limits`: FR-625's limits plus `max_witness_set` and
  `max_relation_tuples` (FR-184).
- `reductions.symmetry` selects the copy-swap reduction where FR-174 admits
  it (FR-623's rules apply).

## Outputs

- `record`: `proved` with basis `exhaustive`, or `reduced` with copy-swap
  and the product-closure certificate (FR-163), `certified` once the core
  checker accepts it; `refuted` with FR-182's settlement.
- `counterexample`: FSpec FR-331's `HyperCounterexample`, one trace per copy
  aligned step by step, replayed copy by copy (FR-183); for forall-exists, the
  universal traces and the exhausted witness set.

## Behavior

- **Budgets.** A witness set or relation that would exceed its limit
  settles `incomplete` `LimitReached{max_witness_set}` or
  `{max_relation_tuples}`; FR-625's limits apply to the product.
- **Cancellation** is FR-625's.
- **Refusals.** A form no candidate advertises settles `unsupported`
  `unsupported-requested-capability`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-631-AC-1 | The vault subject's non-interference clause (the low output does not depend on the high input) settles `proved` for the resetting vault and `refuted` for the leaking one with a two-trace counterexample that replays copy by copy. | Test (TC-1591) |
| FR-631-AC-2 | The same proof requested with copy-swap `symmetry` settles `proved` with basis `reduced` and certification `certified`. | Test (TC-1591) |
| FR-631-AC-3 | A forall-exists clause with `max_witness_set` 1 settles `incomplete` `LimitReached{max_witness_set}`. | Test (TC-1591) |
| FR-631-AC-4 | A hyper `temporal` clause with no object parameter is emitted as a hyperproperty node, and an item naming it settles `proved` or `refuted`, never `unsupported`. | Test (TC-1591) |

## Dependencies

- FR-163 and FR-171 to FR-184 own the hyper forms, settlement, replay and
  budgets; FR-488 owns emission.
