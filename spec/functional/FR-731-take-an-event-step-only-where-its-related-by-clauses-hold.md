---
id: FR-731
title: "Take an event step only where its related-by clauses hold"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-390
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-217
    type: references
  - target: ix://agent-ix/filament-specification/FR-051
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-466
    type: references
---
# FR-731: Take an event step only where its related-by clauses hold

## Description

A `related by r (a, b)` clause on an event node (`send`, `receive`,
`attempt`, `effect` or `event`) states business correlation: the step it
annotates belongs to the instance of the model relationship `r` that links
the value of `a` to the value of `b` (FSpec FR-051). S3 checks the clause
(FR-218, Relationships). The protocol system SHALL execute it as a premise of
the step: the step exists only in a state and binder valuation in which every
`related by` clause of the node holds, and an observed event binds to the node
only when its observed values satisfy every clause.

Correlation is explicit so that the engine never infers a business link from
equal identifiers, equal payloads or arrival order (FSpec FR-051). Making the
clause a step premise gives it one meaning in exploration, simulation and
replay: a behaviour in which a payment effect is attached to an order it is
not related to does not exist.

## Use case

A designer annotates `effect Paid of Charge as (paid: Payment) related by
settles (paid, order) { true };` where `settles` is the domain relationship
from a payment to an order. Exploration produces `Paid` steps only for
payments that settle the protocol's order. Replay of a trace whose payment
settles a different order refuses at that step instead of reporting a
conforming run.

## Inputs

- A checked protocol with its checked `related by` clauses (FR-218), each
  bound to its model relationship's declaration key and end types.
- The model state and binder valuation at a candidate step (FR-206), or an
  observed event at a replay position (FR-217).

## Outputs

- The enabled steps of the node, restricted by its clauses; or a replay
  refusal at an observed event whose values the clauses reject.

## Behavior

- The protocol system SHALL evaluate, when it computes the steps of an event
  node (FR-206), each `related by` clause's two ends with FR-390's evaluator
  in the same state and binder valuation in which it evaluates the node's
  `constraint`, and SHALL treat the clause as holding exactly when the pair of
  end values is an instance of the relationship in that state's model
  observation. The relationship is identified by its declaration key, never
  by its written name.
- A candidate step SHALL be a step of the node only when the node's
  `constraint` and every one of its `related by` clauses hold. Clauses are
  conjoined; their order does not change which steps exist. A node none of
  whose candidates satisfies its clauses has no step, and its thread waits,
  as for an unsatisfied constraint (FR-206).
- When an end is undefined (FR-390), the protocol system SHALL treat the
  clause as not holding and SHALL retain the evaluation's undefinedness
  finding with the state, as for a constraint.
- In replay (FR-217) and in any observation binding of a protocol event, an
  observed event SHALL bind to an event node only when every `related by`
  clause of that node holds over the observed values. An observed step that
  binds to no enabled step refuses with `invalid_runtime_input`/
  `invalid-value` at that step, naming the node and the first clause, in
  source order, that does not hold.
- Two equal scalar identifiers SHALL NOT satisfy a clause: only an instance
  of the relationship does.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-731-AC-1 | Over a model with orders `o1`, `o2` and payments `p1` settling `o1` and `p2` settling `o2`, a protocol over `o1` whose `event Paid by R as (paid: M::Payment) related by settles (paid, input) { true }` explores `Paid` steps binding `p1` only; the same node with no `related by` clause explores steps binding both. | Test (TC-2023) |
| FR-731-AC-2 | The AC-1 node with a second clause that no payment satisfies has no step, and its thread waits; swapping the order of the two clauses gives the same states. | Test (TC-2023) |
| FR-731-AC-3 | Replaying a trace whose `Paid` step binds `p2` refuses `invalid_runtime_input`/`invalid-value` at that step, naming `Paid` and the clause `settles`; the trace binding `p1` replays. | Test (TC-2023) |

## Dependencies

- FR-218 checks the clauses; FR-206 defines the steps; FR-390 evaluates the
  ends; FR-217 replays.
- FSpec FR-051 requires explicit correlation.
