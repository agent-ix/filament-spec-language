---
id: FR-883
title: "Evaluate allInstances and lookup under a state clause or a protocol slot"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-406
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-403
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-381
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-391
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-464
    type: depends_on
---
# FR-883: Evaluate allInstances and lookup under a state clause or a protocol slot

## Description

The clause entry (FR-406, `Clause` regime) SHALL evaluate `allInstances<T>(p)`
and `lookup(p, key)` over a `Population`-typed operand by the rules the
`Value` regime uses (FR-403 items 6 to 8), on the side the read map gives the
node:

- at `own` (`post`), the population as the state holds it;
- under `Pre(...)`, the population at `pre` (contract item C5).

A lookup of an absent key gives FR-403's absent-key result, the same result
it gives outside a clause.

The protocol evaluator SHALL evaluate the two operations in the same way in
a protocol slot that reads model state, over the model state of the protocol
state it is evaluated in. These slots are a protocol, registration or activation capture, an event
or `send` constraint, a `finish` constraint and a `terminal when` condition
(FSpec FR-464 stage table). A slot that bars model state never reaches
evaluation with either operation. These slots are the activation and trigger
guards, the retry relation, the recovery predicate, a `choice` or `repeat`
guard, a `visible` entry, a `check` condition and a FIFO key. S3 refuses the
read with `ill_typed`/`operator-ineligible` at the read (FSpec FR-464
"Slots that bar model state").

### Decision: one population rule in every regime

`allInstances` and `lookup` already have one meaning: they read the
population that the evaluation's state holds. A clause body, or a protocol
capture that enumerates or looks up model objects, reads that same
population. Refusing the operations in those places would remove a feature
that works everywhere else, while the operations need no rule of their own.
Which slots may read model state at all is the standard's stage table, and
this requirement leaves it unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-883-AC-1 | A protocol capture `n: Integer = size(allInstances<M::Order>(M::orders))` over a state holding two orders binds 2, and a `send` constraint `size(allInstances<M::Order>(M::orders)) >= 1` is satisfied there. The same protocol over a state with no order binds 0 and the `send` is not enabled. | Test (TC-2507) |
| FR-883-AC-2 | A capture `o: Option<M::Order> = lookup<M::Order>(M::orders, k)`, naming the domain package's `orders` population (FR-391), binds the order for a present key. For an absent key it gives the same absent-key result as the same `lookup` in a state clause body (FR-403): `none` under the `empty` policy and the undefined result under `undefined`. | Test (TC-2507) |
| FR-883-AC-3 | A state clause whose body is `size(allInstances<T>(p))` over a `Population` parameter returns the member count at `own`, and under `Pre(...)` the count at `pre`. A `pre` read with no pre side returns `WrongAnchor`. No `unsupported-feature` result is produced. | Test (TC-2507) |
| FR-883-AC-4 | A `choice` guard and a `check` condition that call `allInstances<M::Order>(M::orders)` each refuse at S3 with `ill_typed`/`operator-ineligible` at the call, naming the slot. | Test (TC-2507) |

## Dependencies

- [FR-406](FR-406-evaluate-clause-families-through-a-filament-eval-clause-entry.md):
  the clause entry and its node table.
- [FR-403](FR-403-implement-the-no-std-evaluator-in-filament-eval.md): the
  population operations.
- [FR-381](FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md):
  the protocol slot stages.
- FSpec FR-464: the stage table and the slots that bar model state.
