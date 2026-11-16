---
id: FR-851
title: "Enumerate a bounded integer field's successor values from the bounds its postcondition states"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-625
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-627
    type: references
---
# FR-851: Enumerate a bounded integer field's successor values from the bounds its postcondition states

## Description

When an operation's postcondition bounds a modified integer field by
comparisons against values known before the step, the successor generator
SHALL enumerate only the field values inside those bounds, instead of every
value of the field's type. The explored states, transitions, verdicts,
counterexamples and findings SHALL be exactly those of full enumeration.

### Decision: narrow the candidates, keep the state graph

A model field of a wide integer type, such as a speed of `Int[0, 2000]`,
whose postcondition says `self.speed <= 300 and self.speed <= pre(self.speed)`,
admits a few hundred successor values, but full enumeration evaluates the
postcondition on all 2001 candidates for every application. Over a model with
several such operations and a liveness claim under fairness, that per-state
cost makes an explicit-state search run for minutes on a state space of a
few thousand states. FR-120's solved assignment already removes this cost for
an equation `self.f = e`; this requirement extends the same rule to the
bounds `<`, `<=`, `>=` and `>`, inside a conjunction. The candidates skipped
are exactly those whose postcondition is false, so the state graph is the
graph full enumeration builds. A semantic reduction that removes states
would change what a proof covers, and it stays a caller choice (FR-623).

## Inputs

An operation application during successor generation (FR-120 "Post-states"),
over a model with a `modifies` frame.

## Outputs

The application's successor candidates.

## Behavior

### Eligible conjuncts

- A `post` clause SHALL be read as the conjunction of its top-level `and`
  operands. A conjunct is a *bound* on a field `f` when it is
  `self.f op e` or `e op self.f`, with `op` one of `<`, `<=`, `>=`, `>`,
  where `f` is a field the frame `modifies` whose type is an integer type
  with a finite range (FR-120 "Bounds"), and `e` reads only `pre(...)` terms,
  the operation's arguments and literals: not `self.g` outside `pre(...)`,
  not `result`, and not another field's post value.
- A conjunct is an *equation* on `f` under FR-120's solved-assignment rule
  when it is `self.f = e` with `e` of that same form. A boolean field
  conjunct `self.b` or `not self.b` is the equation `self.b = true` or
  `self.b = false`.
- Every other conjunct is kept and evaluated as before; it never narrows a
  field.

### Narrowing

- For a field with one or more bounds and no equation, the generator SHALL
  evaluate each bound's `e` once per application with a fresh meter through
  `evaluate_clause` and enumerate only the values of the field's range that
  satisfy every bound, in the range's ascending order. The interval is the
  intersection of the field's range with each bound's half-line.
- For a field with an equation conjunct, FR-120's solved assignment applies,
  also when the equation is one conjunct of a conjunction (FR-120
  "Eligible"), and each bound on the same field is a conjunct the solved
  value must satisfy. In `loseLidar` and `brake`, `mode` and `lidarOk` are
  solved and `speed` is narrowed, so the application generates one
  candidate per admitted speed.
- When any bound's `e` evaluates to anything other than `Completed(v)` with
  an integer `v`, the field SHALL be enumerated over its whole range for that
  application, so the findings and stops are the enumeration's.
- Each candidate the generator produces SHALL still have the whole
  postcondition evaluated on it, so a conjunct this requirement does not read
  still decides whether the candidate is kept.
- An application whose frame has a `deletes` or `creates` entry is enumerated
  as before (FR-120 "Eligible").

### Invariant and counting

- For every eligible application, the generated successor set SHALL equal
  the enumeration's, in the enumeration's candidate order, with the same
  transition identities, `StepEffect`s, findings and `ExpansionStop`, and
  FR-101's exploration order is unchanged.
- A candidate SHALL count against `max_candidates` exactly when the generator
  generates it, by FR-120's one counting rule: the candidates outside the
  interval are neither generated nor counted. A run the enumeration completes
  under `max_candidates` completes with narrowing and returns the same
  `Exploration`.

## Acceptance Criteria

The rover fixture: the rover mode-machine domain package (`Rover` with `mode`
in `Driving`, `Degraded`, `SafeStop` and `Failsafe`, `speed: Int[0, 2000]`,
`lidarOk`, `stopped` and `voteWrong`), its rules (`loseLidar` from `Driving`
with `post { self.mode = Degraded and not self.lidarOk and self.speed <= 300
and self.speed <= pre(self.speed) }`, `brake` from `Degraded` with
`post { self.mode = SafeStop and self.speed <= 100 and self.speed <=
pre(self.speed) }`, `halt` from `SafeStop` with `post { self.speed = 0 and
self.stopped }`, `enterFailsafe` once stopped, and `sample` in every mode but
`Failsafe`), and one initialization snapshot holding a driving rover `r1` at
`speed` 800 with a working lidar.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-851-AC-1 | Exploring the rover fixture with narrowing and with the generator forced to full enumeration gives equal `Exploration`s: the same state keys in the same order, the same transitions and the same findings; the `loseLidar` application from the initial state generates 301 candidates and `brake` from a `Degraded` state at `speed` 300 generates 101, where full enumeration generates 2001 each. | Test (TC-2411) |
| FR-851-AC-2 | With `temporal.limits.max_candidates` 400 an analyze item over the rover fixture explores to completion and reports the same `exploration` as with no limit; with `max_candidates` 300 it settles `incomplete` naming `max_candidates` at the `loseLidar` application. | Test (TC-2411) |
| FR-851-AC-3 | With `loseLidar`'s bounds replaced by one that is not of the `self.f op e` form (`300 - self.speed >= 0`), or placed under `or` (`self.speed <= 300 or self.stopped`), the application is enumerated over the whole range: with `max_candidates` 400 the item settles `incomplete` naming `max_candidates`. | Test (TC-2411) |
| FR-851-AC-4 | The rover fixture analyzed through `native-analyze/1` for the temporal clause `always (holds(not r.lidarOk) implies eventually holds(r.stopped))` under `fair weak brake` and `fair weak halt`, with no `temporal.limits`, settles `proved` with basis `exhaustive`, reaches no limit at EN-1's published defaults, and completes within 60 seconds in the test profile. | Test (TC-2411) |

## Dependencies

- FR-120 owns successor generation, its solved assignment and its counting
  rule; this requirement adds the bound conjuncts to that rule.
- FR-126 and FR-142 call the generator; their explorations are unchanged.
- FR-623 owns the reductions a caller selects; narrowing is not one of them.
