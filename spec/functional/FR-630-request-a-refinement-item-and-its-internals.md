---
id: FR-630
title: "Request a refinement item, its internals and its history and nested-reference rows through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-135
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-138
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-140
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-142
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-143
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-144
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-145
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-146
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-147
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-149
    type: depends_on
---
# FR-630: Request a refinement item, its internals and its history and nested-reference rows through analyze

## Description

A `native-analyze/1` item naming a refinement declaration SHALL request
FR-144's refinement item: the mapping is evaluated at every reachable
concrete state (FR-140), its safety half is decided on the explicit-state
product (FR-142) and its liveness half under the abstract fairness
(FR-143), with auxiliary and history variables (FR-138), a simulation
certificate (FR-149) and counterexample replay (FR-145). A wrong mapping
compiles, and only this item finds it.

### Decisions

- **Refinement internals are a request member.** The document carries
  `refinement_internals` at its top level for the program's own refinements
  and on each `libraries` entry for that library's, exactly as FR-027 reads
  them for compile; the reader hands them to compilation unchanged (FR-147).
  The same internals reach every replay the item runs.
- **History rows of every finite value type are checked and computed.** An
  enum, decimal, or option-of-enum or option-of-decimal history row is
  checked and computed as an integer row is; its value enters the history key
  by its canonical literal. A history row is auxiliary state the user writes
  to make a mapping expressible; restricting its type would make some
  correct refinements unstatable.
- **Nested-reference rows are lifted.** A field row whose type nests a
  `Reference<T_C>` inside an option, record, union or collection value lifts
  each nested reference to the `Reference<T_A>` of the object row that maps
  `T_A` from `T_C`, element by element at every depth, as a top-level
  reference is lifted (FR-135). A nested reference whose concrete type no
  object row maps refuses at S3 as a top-level one does.
- **With no `abstract_subject`, the abstract initial states are the
  admissible completions of each mapped concrete initial state.** An FSL
  model has no initial clause and no constructor: a snapshot is the only
  source of initial states, and the caller who omits `abstract_subject`
  supplies no abstract snapshot. The abstract initial states of one concrete
  initial state are therefore every abstract state that agrees with the
  mapping's image of that concrete state on every field the mapping fixes,
  with each abstract field the mapping does not fix (a hidden field) ranging
  over its declared type, kept only when the state satisfies the abstract
  model's invariants and its unit bounds. One concrete initial state may
  thus map to a set of abstract initial states; that set is the item's
  initial candidate set (FSpec FR-378), and refinement must hold from it.
  Taking a single default value for a hidden field instead would refute
  correct refinements whose hidden state is free at the start, and taking
  no constraint at all would admit abstract states the abstract model
  forbids. The same rule gives the initial states of a protocol abstract
  side: its abstract fields the mapping does not fix range over their types
  under the protocol subject's invariants and unit bounds.
- **The enumeration is metered, under one candidate cap.** Every completion
  the enumeration builds is charged toward `temporal.limits.max_candidates`
  (FR-625), and that cap is one total across all concrete initial states of
  the item, not a cap per concrete state, so the work of the item's start is
  bounded by one number the caller chose. A stop at the cap, or at any other
  budget of the item, settles FSpec FR-331 `incomplete`, FR-241
  `resource-incomplete`, cause `limit-reached{limit, value, setting}` naming
  the limit reached (FSpec FR-378), never `proved` over the completions built
  so far.
- **An empty completion set is a counterexample, not a refusal.** When no
  abstract state agrees with a concrete initial state's image on its fixed
  fields and also satisfies the abstract invariants and unit bounds, the
  mapping sends that initial state outside the abstract model's initial
  states. That is a wrong mapping, which the refinement item exists to
  find, so the item settles `refuted` with FSpec FR-331's
  `InitialNotAbstract{initial}`, `initial` the concrete initial state's
  index, at position 0, and its counterexample is the one-state concrete
  path, replayed by FR-145. It is never `declined` or `invalid-input`.

## Inputs

### Request item

- `claim`: the refinement declaration's name; `kind` `refinement`;
  `temporal.item_kind` `refinement`.
- `temporal.subject`: the concrete subject; `temporal.abstract_subject`: the
  abstract subject's initial states and universes. When it is absent, the
  abstract initial states are the admissible completions of the mapping's
  image of each concrete initial state ("Decisions" above).
- `temporal.limits` as FR-625, applied to the product.

## Outputs

- `record`: `proved` with the simulation certificate, `certified` once
  FR-149's core checker accepts it; `refuted` with the refinement failure
  part (`unmapped-step`, `mapping-undefined`, `abstract-step-disabled`,
  `liveness`) of FSpec FR-331's `RefinementFailure`, or, at position 0,
  `InitialNotAbstract{initial}`; `incomplete` with cause
  `limit-reached{limit, value, setting}` when the initial enumeration or the
  product reaches a budget.
- `counterexample`: the concrete path with the abstract image of each state,
  replayed by FR-145; the per-step simulation records of FR-146 travel in it.

## Behavior

- **Budgets and cancellation** are FR-625's over the product.
- **Default abstract initial states.** When `abstract_subject` is absent,
  the item SHALL take, for each concrete initial state, every abstract state
  that equals the mapping's image on the fields the mapping fixes, whose
  other fields hold values of their declared types, and that satisfies the
  abstract model's invariants and unit bounds (for a protocol abstract side,
  the protocol subject's), and SHALL decide refinement from each of them.
- **Initial enumeration budget.** The item SHALL charge each completion it
  builds toward one `max_candidates` total for the item; when the total or
  another budget is reached it SHALL settle `incomplete`,
  `resource-incomplete`, `limit-reached{limit, value, setting}`.
- **No admissible completion.** When a concrete initial state has no
  admissible completion, the item SHALL settle `refuted` with
  `InitialNotAbstract{initial}` at position 0 and a replayed one-state
  counterexample, and SHALL NOT settle `declined`.
- **Refusals.** A refinement whose abstract side is a protocol runs FR-147's
  protocol arm; an abstract subject that does not admit settles `declined`
  `invalid-input`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-630-AC-1 | A register refined by a one-place queue with a correct mapping settles `proved` with certification `certified`; with a mapping that sends the empty register to a non-empty queue it settles `refuted` with `mapping-undefined` or `abstract-step-disabled` and a replayed counterexample carrying per-step records. | Test (TC-1590) |
| FR-630-AC-2 | A request carrying `refinement_internals` naming the program's refinement compiles with them and settles as FR-147 states; without the member it supplies the empty list. | Test (TC-1590) |
| FR-630-AC-3 | A history row typed `Decimal[0, 100; 0, 2; nearest-even]`, and one typed as an enum, check, and the refinement item over each settles `proved` for a correct mapping. | Test (TC-1590) |
| FR-630-AC-4 | A field row of type `Sequence<Reference<Ring>>[0, 2]` mapped to `Sequence<Reference<Queue>>[0, 2]` checks and its refinement item settles `proved`. | Test (TC-1590) |
| FR-630-AC-5 | Over the Counter fixture (`value: Int[0, 3]`) refined by the abstract `Spec::Counter`, with no `abstract_subject`: the refinement `Hidden`, whose object row fixes no field (`object Spec::Counter from M::Counter { }`) and maps `inc` to `stutter`, over the concrete snapshot with counter `a` at 0, settles `proved`, its initial candidate set holding exactly the four abstract states `value` 0, 1, 2 and 3; the same item with `temporal.limits.max_candidates` 3 settles `incomplete`, `resource-incomplete`, cause `limit-reached` naming `max_candidates`; over the two concrete snapshots `a` at 0 and `a` at 1 it settles `incomplete` naming `max_candidates` with `max_candidates` 4 and `proved` with 8, the cap counting completions across both initial states; the refinement `Shifted` (`value = self.value + 3`, `inc` mapped to `stutter`) over the snapshot `a` at 1, whose image `value` 4 lies outside the unit bound `Int[0, 3]`, settles `refuted` with `InitialNotAbstract{initial: 0}` at position 0 and a one-state counterexample that replays, never `declined` or `mapping-undefined`. | Test (TC-2964) |

## Dependencies

- FR-135 to FR-149 own refinement checking; FR-027 owns the
  `refinement_internals` reading.
