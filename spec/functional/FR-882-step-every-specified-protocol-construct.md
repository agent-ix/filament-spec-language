---
id: FR-882
title: "Step every specified protocol construct"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-207
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-209
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-155
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-220
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-880
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-426
    type: implements
  - target: ix://agent-ix/filament-specification/FR-729
    type: implements
  - target: ix://agent-ix/filament-specification/FR-730
    type: implements
---
# FR-882: Step every specified protocol construct

## Description

The protocol system (`ProtocolSystem`, FR-206, ADR-027) SHALL step every
construct of a checked protocol that the standard specifies, and SHALL NOT
settle a model-check item (deadlock freedom, terminal and temporal
obligations over the protocol subject) `unsupported` for any of the following
constructs. Each one is stepped by the rule given here. Projection and
realizability items keep their own profile rule (FR-575, ADR-036 PA-21), which
this requirement does not change.

- **`finish` and compensation templates.** Every checked control kind maps
  to a step kind. A protocol's `finish` is stepped by FR-206 (`finish`, which
  is enabled once every thread has left `running`, ADR-027 ST-9). Its
  compensation templates are stepped by FR-207 (registration, activation,
  `cattempt`, `cend`). The checked clause has no "unstepped" control
  class.
- **Recovery predicates that read their recovery binder.** These are stepped
  by FR-207-AC-5 and FSpec FR-730: one `cend` successor per recovery record.
- **Guards and captures undecided at an initial state.** A protocol capture,
  `choice` guard, `repeat` guard or `check` whose value is undefined at an
  instance's first state ends the check with a violation with cause
  `UndefinedEvaluation{where, cause}` (FSpec FR-426). In that violation,
  `where` is the construct's position and the counterexample is the prefix
  that reaches that state: the empty trace for an `origin` instance, and the
  `activate` step for an `each` instance. FR-127 settles the violation
  `refuted`, as any undefined protocol expression is settled. The value is
  never read as `false`.
- **An `over` input that is not an object reference.** The instance's `over`
  binder ranges over the finite domain of its type. For `activation on
  origin` there is one initial instance per value, in the type's canonical
  value order. Under `activation on each`, each `activate` step binds the
  `over` value of the request it was started for (FR-209). A type with no
  finite domain is refused by the step-binder pre-check (ADR-027 TS-5), as
  any unenumerable binder type is.
- **Attempt binders.** S3 admits an attempt binder only when its type is the
  operation's result type (FR-218, Attempts), and refuses any other binder
  type with `ill_typed`/`type-mismatch` at the binder. The refusal retains
  `expected`, the result type, and `actual`, the binder's type, or no result
  for an operation that returns none. Every admitted attempt binder is
  therefore bound by ST-1, and the stepper never meets an unbound one.
- **Attempt footprints under a weak `parallel`.** An attempt's access is
  classified (FR-220) by its FR-155 footprint. When FR-155 derives no
  footprint for the application, for example for a body it cannot read
  statically, the footprint is the operation's declared frame (`modifies`)
  for writes, together with every location the selected contract clauses
  read for reads. This is a sound over-approximation, and the access is
  classified over it.
- **Protocol references.** A `protocol X` reference evaluated in a `check`
  condition, a guard, a capture initializer, an event or `send` constraint,
  a `finish` constraint, a `terminal when` condition, or a function body
  that such a slot calls has the value of the clause `X` (FSpec FR-463).
  That value is the verdict of `X`'s obligations (its `terminal` member and
  its required temporal clauses) checked over `X`'s protocol subject from the
  subject's initial states, with `X`'s `over` input ranging over its domain
  as an `origin` instance (the rules above): `true` when every obligation
  holds and `false` when one is refuted. The reference reads no current
  model state, binder or capture (FSpec FR-728), so it is admitted in slots
  that bar model state and has one value for the whole item. Any other
  settlement of that check (a limit, an inconclusive or unsupported result)
  makes the read undefined, and the undefined rule above applies. A check
  that re-enters a protocol whose value is already being computed is
  undefined in the same way, so a reference cycle (FR-880-AC-5) never loops.
  The check runs once per item, under the enclosing item's limits and meter.
- **Newly written forms (FR-880).** A post-test `repeat` is stepped by FSpec
  FR-729's moves. A `terminal when` that reads `over` under `on each`
  evaluates over each instance's own `over` value. A `repeat` that steps
  through a nested `repeat` is stepped by FR-206's structural moves.

### Decision: an unsupported verdict is never the answer for a specified construct

Each construct listed here has a meaning in the standard. Before this rule,
the stepper reported `unsupported` for them before exploring any state, so
a protocol author learned nothing about the protocol. Under this rule each
construct is stepped or settled by a rule that already applies to the same
kind of construct elsewhere:

- an undefined value is a refutation, as it is for a predicate join
  (FR-206);
- an `over` value is a finite choice of initial state, as a trigger record
  is;
- a footprint that cannot be derived is over-approximated by the frame, as
  FR-155 already does for frame enforcement.

Apart from a channel whose capacity is given only by name, which is stepped
by the symbolic-capacity rule of FR-218-AC-13, two stepper causes remain.
Each one names a limit that a request or a model sets, not a missing design:

- `UnenumerableRecord`: a binder type with no finite domain;
- `WeakMemory`: a memory model that the selected system does not implement.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-882-AC-1 | Over a protocol whose last step is `finish` and which holds a compensation template activated under `on each`, the reachable states hold `finish` steps and one compensation thread per activation (ADR-027 §7.1's `Pay` under `on each` with triggers 0 and 1), and no checked control has an unstepped class. | Test (TC-2505) |
| FR-882-AC-2 | A `choice` guard that divides by a protocol capture equal to 0 at the first state ends the deadlock-freedom check with `UndefinedEvaluation` at the `choice`, settled `refuted` with an empty-prefix counterexample under `origin` and an `activate`-only prefix under `each`, never `unsupported`. | Test (TC-2505) |
| FR-882-AC-3 | A protocol `over (n: Int[0, 3]) on origin` has four initial instances, one per value. A `check` that reads `n > 1` is passed by two of them and blocks two, so deadlock freedom is refuted with the instance `n = 0` as the first counterexample. A protocol over `Boolean` has two initial instances. | Test (TC-2506) |
| FR-882-AC-4 | An attempt binder of type `Boolean` over an operation that returns `Integer`, and one over an operation that returns nothing, each refuse at S3 with `ill_typed`/`type-mismatch` at the binder, retaining `expected` and `actual`. The same binder over an operation that returns `Boolean` checks and is bound by each `attempt` step. | Test (TC-2506) |
| FR-882-AC-5 | Under a `tso` `parallel`, an attempt whose operation body defeats footprint derivation is classified over its frame and contract reads. The protocol explores, and the access class equals the one obtained from a hand-written footprint of the same locations. | Test (TC-2506) |
| FR-882-AC-6 | A post-test `repeat` (FR-880-AC-8), a `terminal when` that reads `over` under `on each` (FR-880-AC-6) and a protocol over `Option<M::Order>` each explore to a settled deadlock-freedom result. The post-test `repeat`'s first body step is enabled at its entry with no guard read. | Test (TC-2506) |
| FR-882-AC-7 | Over a corpus holding one minimal protocol for each construct FR-218 admits, every protocol explores to a result. Apart from symbolic capacity, the only stepper refusals in the corpus are `UnenumerableRecord`, for a unit whose binder type has a text field, and `WeakMemory`, for a unit whose resolved model the system does not implement. | Test (TC-2505) |
| FR-882-AC-8 | Where `Other`'s terminal obligation holds from its subject's initial states, `check Gate { protocol Other }` in `Flow` passes at every state and `Flow`'s deadlock freedom is proved; where it is refuted there, the `check` blocks at every state and deadlock freedom is refuted at `Gate`. Changing the model state `Flow` has reached before `Gate` does not change the value. A self-reference `while { protocol Flow }` settles `refuted` with `UndefinedEvaluation` at the guard, never `unsupported` and never a non-terminating run. A `send` constraint and a capture that hold `protocol Other` read the same value as the `check`. | Test (TC-2505) |

## Dependencies

- [FR-206](FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md)
  and [FR-207](FR-207-run-compensation-templates-as-protocol-threads.md): the
  step kinds and compensation.
- [FR-209](FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md):
  instances and triggers.
- [FR-155](FR-155-derive-and-enforce-read-and-write-footprints.md) and
  [FR-220](FR-220-check-access-orderings-fences-and-access-classification.md):
  footprints and access classes.
- [FR-880](FR-880-write-every-checked-protocol-construct-at-s4.md): the
  forms S4 now writes.
- FSpec FR-426 (the undefined rule), FR-729 (post-test moves) and FR-730
  (recovery records).
