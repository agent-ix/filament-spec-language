---
id: FR-173
title: "Classify hyper and relation clauses into forms and record their requirements"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-021
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-057
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-172
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-043
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
---
# FR-173: Classify hyper and relation clauses into forms and record their requirements

## Description

S3 SHALL classify every checked hyper clause over `behaviours` and every
relation over model executions into exactly one form, HP-1 to HP-6 (ADR-023
§3), from its quantifier prefix, body, fairness sets and alignment, and
record the form beside its requirement record. The admitted forms are HP-1,
HP-2, HP-3, HP-5 and HP-6; HP-4 records a form that negotiation settles V-8.
The request writer SHALL add one subject per alias and the deadlock-freedom
item of each subject (ADR-023 HM-6).

## Use case

A verification operator writes a `∀∀` noninterference clause, a `∀∃`
opacity clause, a determinism relation, and a `∃∀` clause. The first three
are admitted and routed to their constructions; the last is reported
`unsupported` with the reason, and no engine runs for it.

## Inputs

- A `CheckedHyperClause` or `CheckedModelRelation` (FR-172), with its
  body's safety classification (ADR-014 A-4 as amended by ADR-018 IV-4, as
  FR-123 classifies `Safety`).

## Outputs

```rust
pub enum HyperForm {
    StepRelation,              // HP-1
    Universal,                 // HP-2
    ForallExistsSafety,        // HP-3
    Other { reason: HyperUnsupported },   // HP-4
    SingleExistential,         // HP-5
    ProjectionAligned,         // HP-6
}
```

- `Requirements{kind: temporal-satisfaction, extent: Unbounded{domains}}`
  with the `HyperForm` beside it; for HP-1 with a code binding, the
  additional record FR-180 states.

## Behavior

- S3 SHALL classify in this order:
  1. `StepRelation` (HP-1): a `relation` over model executions.
  2. `SingleExistential` (HP-5): a prefix of exactly one `exists`, with no
     `align skip`.
  3. `Other` (HP-4): an `exists` before a `forall`; or an `exists` with a
     `forall` or a second `exists` beside it whose body is outside the safety
     fragment or whose fairness set is non-empty; or `align skip` with any
     `exists` or with a body outside the safety fragment.
  4. `ProjectionAligned` (HP-6): every quantifier `forall`, `align skip`
     present, body in the safety fragment.
  5. `Universal` (HP-2): every quantifier `forall`, `n >= 1`, lockstep.
  6. `ForallExistsSafety` (HP-3): prefix `forall^n exists^m`, `n >= 0`,
     `m >= 1`, `n + m >= 2`, body in the safety fragment, every existential
     variable with the empty fairness set.
- `Other` SHALL carry which rule placed it there.
- If an HP-5 clause carries a `match` block, then S3 SHALL refuse
  `unsupported_construct`/`expression-form` at the block (ADR-023 SE-1).
- S3 SHALL record each clause as `Requirements{kind: temporal-satisfaction,
  extent: Unbounded{domains}}` (FR-057, ADR-014 A-3) with its `HyperForm`
  beside it, so it routes on the existing kind and a candidate's arm reads
  the form.
- The EN-1 provider manifest SHALL advertise HP-1, HP-2, HP-3, HP-5 and HP-6
  through FR-075's registration, and no provider SHALL advertise HP-4, so
  negotiation settles HP-4 V-8, `unsupported-requested-capability`, with no
  engine run.
- A `behaviours` clause under `filament.temporal.event-position.false-extension/v1`
  SHALL be classified by the same rules, in the same order, and SHALL
  record its form beside its profile, so `CheckedHyperClause::form()`
  returns that form for it. Wherever a
  classification rule requires the safety fragment, a body under this
  profile meets it, because a closed finite trace decides its truth
  (ADR-018 TP-2, CX-1). Its form is read over closed finite traces
  ("Event-position form" below), it reaches a verdict and it replays
  (FR-640-AC-12).
- An event-position clause whose prefix is exactly one `exists`, with no
  `align skip`, is `SingleExistential` (ADR-023 SE-1). EN-1 decides it by
  searching, from each initial state, the product of the subject with the
  body's bounded monitor for a closed finite execution (ending at a
  terminal state, or covering the horizon) on which the monitor accepts.
  It settles `proved` with one closed witness trace per initial state, and
  `refuted` with a trap naming an initial state from which the exhausted
  product holds none (SE-4, SE-5); its `match` block is still refused by
  SE-1.
- A `behaviours` clause under any profile other than infinite-trace and
  event-position false-extension SHALL record its profile and no form, so
  negotiation settles it V-8, FR-360 label `unsupported` with basis
  `unavailable`, `unsupported-requested-capability`, with no engine run
  (ADR-023 HS-7, as amended).
- The request writer SHALL supply one model subject (FR-125) per distinct
  alias the item names, and SHALL add the deadlock-freedom item of each
  distinct subject as FR-124 states; a hyper or relation item SHALL report
  no deadlock itself.
- When negotiation evaluates an `Other` form before provider selection, S3 SHALL
  apply FR-182's hyper-form settlement as V-8
  `unsupported-requested-capability` and produce no engine outcome for that
  item. When `fsl-route` receives the resulting `unsupported` disposition,
  fsl-route SHALL route the item to no candidate. This item-local result SHALL NOT
  prevent an independently supported sibling item from routing to its
  selected candidate.

### Event-position form

The event-position profile is the model subject's bounded profile (ADR-018
TP-2, SM-4): it reads a behaviour as a closed finite execution, never as an
infinite one. A hyper clause under it keeps that reading for every trace
variable, so its form is decided over tuples of closed finite traces.

- Each trace variable ranges over the closed finite executions of its
  subject: a path from an initial state that ends at a terminal state, or
  that covers the body's ADR-014 TR-4 horizon from every activation the
  body reads (ADR-018 CX-1). A terminal state is not stutter-extended
  (ADR-023 HM-5 does not apply), and no trace is read as a lasso: the engine
  never produces, and replay never reads, a progressing lasso for this form.
  A subject with no terminal state contributes the paths of exactly
  `h + 1` positions, 0 to `h`; under `align skip` a trace covers `h + 1`
  visible positions, and steps after its last visible one are not read.
- Lockstep alignment (ADR-023 HM-2) pairs positions by their index. A joint
  position past a component's last position lies outside that component's
  closed scope, and every atom that reads that component there evaluates
  false (FR-043 false extension). A joint step past a component's last
  position carries no step of that component: a `match` conjunct that
  reads that component's step label is not evaluated there, so `μ`
  constrains only the joint steps every component it reads takes.
- A trace variable of an event-position clause carries no fairness set: a
  closed finite execution admits every behaviour (ADR-018 SM-5, FA-1), and
  S4 refuses a clause with a non-empty set
  `unsupported_construct`/`not-yet-implemented` at its first constraint
  (FR-488 item 3).
- Each trace is read under its own clock configuration: the
  `event_position` configuration of the `temporal_bindings` row the
  compiler writes for that trace variable at the clause's `claim`
  occurrence (FR-640), whose `sequence_authority` is its subject's
  declaration identity. Two traces over two models are read under two
  configurations; two traces over one model under two equal ones.
- The verdict is decided over every tuple of closed finite executions: the
  clause is `proved` when the body holds on every such tuple the quantifier
  prefix and `match` admit (ADR-023 HM-3), and `refuted` by a counterexample
  that is one closed finite trace per trace variable, universal and
  existential, with no loop. `fsl_replay::replay` reads each trace of that
  counterexample as a closed trace under its own row's configuration and
  reproduces the verdict. Bounds, limits and undefined evaluation settle as
  ADR-023 HV-3, HV-5 and HV-8 state for the infinite-trace forms.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-173-AC-1 | ADR-023 §1's `NonInterference` classifies `Universal`; §8.2's `Opaque` `ForallExistsSafety`; FR-171-AC-3's `Det` `StepRelation`; §13's clause with `align skip` `ProjectionAligned`; `exists trace b of V { eventually holds(v.l @ b = 1) }` `SingleExistential`. Each records (`temporal-satisfaction`, `Unbounded`) with its form. | Test (TC-598) |
| FR-173-AC-2 | `Other`, each naming its rule: `exists trace b of V forall trace a of V`; `Opaque` with body `always eventually holds(v.l @ a = v.l @ b)`; `Opaque` with `fair { weak V::Vault::step }` on `b`; §13's clause with an `exists` variable. Each settles V-8 `unsupported-requested-capability` in negotiation with no engine run. | Test (TC-598) |
| FR-173-AC-3 | `SingleExistential` with a `match` block refuses `unsupported_construct`/`expression-form` at the block. `NonInterference` under the event-position profile records profile event-position and form `Universal`, and `form()` returns `Some(Universal)`; the same clause under the fixed-sample profile records its profile and no form, `form()` returns `None`, and negotiation settles it V-8 `unsupported-requested-capability` with no engine run. | Test (TC-598) |
| FR-173-AC-4 | A request with `SavesPower` supplies subjects for `S` and `N` and carries two deadlock-freedom items; a request with `NonInterference` and `Opaque` over `V` carries one. | Test (TC-598) |
| FR-173-AC-5 | For a request containing an HP-4 `Other` item and a supported `Universal` sibling, negotiation settles the HP-4 item as `unsupported` V-8 without an engine outcome and `fsl-route` gives it no target; the sibling keeps its `supported` disposition and selected candidate. | Test (TC-598) |
| FR-173-AC-6 | Under the event-position profile, analyzed through `native-analyze/1` with EN-1, never `not-yet-implemented`. Fixtures: `fsl-analyze`'s two Secure vault models `V` and `N` (`v: V::Vault`, `w: N::Vault`), each with the one initial state `h = 1`, `l = 0`, whose `step(i)` sets `l = i` for `i` in `{0, 1}` and which have no terminal state, and FR-178-AC-1's busy vault. (HP-2) `forall trace a of V forall trace b of N { always [0, 3] holds(v.l @ a = w.l @ b) }` settles `refuted`: its counterexample holds one trace for `a` and one for `b`, neither with a loop, each of four positions, 0 to 3, and `fsl_replay::replay` reads each as a closed trace under its own `temporal_bindings` row and reproduces the refutation; the same clause with body `always [0, 3] holds(v.h @ a = w.h @ b)` settles `proved`. (HP-3) `forall trace a of V exists trace b of N { always [0, 3] holds(v.l @ a = w.l @ b) }` settles `proved`, and with `!=` in place of `=` settles `refuted` at position 0 with a universal trace that has no loop. (HP-6) FR-178-AC-1's `align skip { V::Vault::mix }` clause over the busy vault from its idle initial states, with body `always [0, 2] holds(v.l @ a = v.l @ b)`, settles `proved`, and FR-178-AC-2's leaky vault started mid-operation with the same body settles `refuted` with a projected counterexample of two closed traces, no loop, that replays. (HP-5) `exists trace b of V { eventually [0, 2] holds(v.l @ b = 1) }` settles `proved` with one closed witness of three positions; `exists trace b of V { eventually [0, 2] holds(v.h @ b = 0) }` settles `refuted` with a trap at the initial state. FR-640-AC-12's `Saves` settles `proved` or `refuted` by the HP-2 construction above, never `FormNotDecided`. | Test (TC-2243) |

## Dependencies

- ADR-023 §3 HP-1 to HP-6 and the admitted fragment; §2 HM-6; HS-7; SE-1;
  ADR-014 A-3 and A-4 as amended by ADR-018 IV-4.
- [FR-172](FR-172-check-hyper-and-relation-clauses-over-model-subjects.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (safety classification), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md),
  [FR-057](FR-057-admit-shared-capability-kinds.md).
- FSpec owns the forms, the admitted fragment, V-8 for HP-4 and the
  claim-form rows (ADR-023 QS-3, QS-4).

## References

- ADR-023. FSpec half: FSpec FR-396 (Linear STD-136; ADR-023 QS-3, QS-4).
