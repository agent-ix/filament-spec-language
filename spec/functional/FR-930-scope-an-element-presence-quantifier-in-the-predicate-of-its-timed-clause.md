---
id: FR-930
title: "Scope an element-presence quantifier in the predicate of its timed clause"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-415
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-760
    type: implements
---
# FR-930: Scope an element-presence quantifier in the predicate of its timed clause

## Description

A timed clause predicate may read the presences of a clock collection's
elements through a quantifier, beside clock constraints that read no slot of
that collection: the guard `self.x >= 1 ms and forall(e in self.q: present(e))`,
the time invariant `when { forall(e in self.q: present(e)) } { self.x <= 9 ms }`
and the urgency `urgent when { forall(e in self.q: present(e)) }`. The emitter
SHALL write such a quantifier inside the clause's predicate expression, as the
collection quantifier application that binds its own slot parameter, and SHALL
NOT lift its binder into the guard part's or invariant's `binders`. A binder
belongs in `binders` only when a clock constraint of that part or a bound of
that invariant has it as a slot term (FSpec FR-322-AC-295); a binder that only
presence reads use is local to the quantifier, which keeps the source meaning:
the quantifier is true on an empty collection (`forall`) or false on one
(`exists`), and the constraints beside it apply whatever the collection holds.

## Inputs

A checked timed-model clause (guard, time invariant or urgency) whose predicate
holds a `forall` or `exists` over a clock collection place whose body reads
only element presences and data, and the clause's clock constraints.

## Outputs

The clause's emitted node: a `clock_guard` part, a `time_invariant` or an
`urgency`, whose predicate holds the quantifier application with the
`presence_read` of the element place at the quantifier's binder.

## Behavior

- **Binder scope.** For each quantifier over a clock collection in a timed
  clause predicate, the emitter SHALL decide its scope by its body: when the
  body holds an atomic clock constraint over the places of its binder, the
  quantifier is a quantified guard part or invariant as FR-230 "Quantified
  reads" states and its binder is a member of that part's or invariant's
  `binders`; otherwise the quantifier SHALL be written as the
  `filament.op.collection.forall` or `filament.op.collection.exists`
  application in the predicate expression, its binder a `parameter` of the
  selector's domain (FSpec FR-322 "Presence") scoped by that application, and
  every `present(e)` in the body the `presence_read` of the element place at
  the clause's owner term and that parameter.
- **Constraints beside the quantifier.** A clock constraint outside the
  quantifier SHALL be written as an unquantified constraint of the same part
  or invariant, with no slot term; the part or invariant SHALL carry no
  `binders` binding when none of its constraints or bounds has a slot term.
- **One part.** A guard conjunction of one such quantifier and unquantified
  constraints SHALL be one guard part, the quantifier being part of its
  predicate, and SHALL NOT be split into one part for each collection size.
- **Disjunction.** `exists(e in self.q: present(e))` SHALL be written the same
  way, as the `exists` application in the predicate, and SHALL NOT become one
  part for each slot, because its body constrains no clock.
- **Shared binder.** A body that holds both a presence read and a clock
  constraint of its binder (`forall(e in self.q: present(e) implies e >= 1 ms)`)
  is a quantified part: the `presence_read` at the binder is in that part's
  predicate and the binder is in its `binders`, as FR-230-AC-55 states.
- **Urgency.** An `urgency` node has no `binders`, so every quantifier of its
  predicate SHALL be written in the predicate expression.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-930-AC-1 | Over `x: Clock` and `q: Sequence<Option<Clock>>[0, 2]`, `pre HasElement using v on Config::ConfigVersion::attemptUpdate { self.x >= 1 ms and forall(e in self.q: present(e)) }` emits one `clock_guard` with one part and no `binders` binding: the part's predicate is the `filament.op.collection.forall` application whose binder is a `parameter` typed by `q`'s selector domain and whose body is the `presence_read` of `q`'s element place at `self` and that parameter, and its one constraint is `x ge 1/1` with no slot term. I2 admits the package and reads the guard back. | Test (TC-2600) |
| FR-930-AC-2 | `time invariant HasElement when { forall(e in self.q: present(e)) } { self.x <= 9 ms }` emits one `time_invariant` with no `binders` binding, its predicate the `forall` application of AC-1 and its one bound `x le 9/1`; `urgent HasElement when { forall(e in self.q: present(e)) };` emits one `urgency` whose predicate is that application. I2 admits both. | Test (TC-2600) |
| FR-930-AC-3 | `pre Any using v on Config::ConfigVersion::attemptUpdate { self.x >= 1 ms and exists(e in self.q: present(e)) }` emits one guard part whose predicate is the `filament.op.collection.exists` application, never one part per slot; `forall(e in self.q: present(e) implies e >= 1 ms)` emits parts whose `binders` hold the binder and whose predicates hold the `presence_read` at it (FR-230-AC-55). | Test (TC-2600) |
| FR-930-AC-4 | Over the emitted package of AC-1, a copy whose guard part also carries `binders` naming the quantifier's parameter is refused by I2 `invalid_package`/`timed-node-reference` at that binder target, because no constraint of the part has it as a slot term (FSpec FR-322-AC-295). | Test (TC-2600) |

## Dependencies

- FR-230 ("Quantified reads", "Clock-typed positions", FR-230-AC-53, AC-55, AC-56, AC-66).
- FSpec FR-322 ("Presence", "Quantified guards and invariants", FR-322-AC-155, FR-322-AC-295) and FSpec FR-760; FSpec FR-415-AC-20.
