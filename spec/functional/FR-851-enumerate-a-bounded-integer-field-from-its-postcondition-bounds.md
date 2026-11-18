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
graph full enumeration builds. That holds only while nothing before the
narrowing conjunct can be undefined on a skipped candidate: a clause's `and`
is evaluated left to right with short-circuit (FR-006), so an earlier
conjunct that is undefined on a candidate makes the clause undefined there,
and full enumeration records a finding for it. Narrowing therefore reads only
the defined prefix of each clause (below). The alternative, evaluating the
earlier conjuncts on every skipped candidate, would restore the per-state
cost this requirement removes, while the defined prefix keeps narrowing for
the usual clause shape, field assignments and bounds over plain field reads. A semantic reduction that removes states
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

### Defined prefix

- A conjunct is *defined on every candidate* when it is one of:
  - an equation or a bound whose `e` evaluated to `Completed(v)` for the
    application, since `e` reads only the pre-state, the arguments and
    literals and so has that value on every candidate;
  - a boolean field literal `self.b` or `not self.b`;
  - a comparison `x op y`, `op` one of `=`, `!=`, `<`, `<=`, `>=`, `>`,
    whose operands are each a literal, an operation argument, or a read
    `self.g` or `pre(self.g)` of a field whose type is `Boolean`, an
    enumeration or an integer type, never an `Option`, and `not` of such a
    comparison.
  Every other conjunct, one that divides, calls a function, reads an
  optional value, looks up a key or applies any other operator, can be
  undefined or refused on some candidate.
- The *defined prefix* of a clause is its longest run of leading
  conjuncts, in source order, each defined on every candidate. An equation
  or a bound SHALL solve or narrow its field only when it stands in its
  clause's defined prefix. An equation or a bound after the first conjunct
  that is not defined on every candidate is kept and evaluated on each
  generated candidate, like any other conjunct, and narrows nothing.
- The prefix is read per clause. A clause that narrows makes the effective
  postcondition false on each candidate it skips, and the conjunction over
  clauses is false whenever any clause is false (FR-120 "Contract clauses"),
  so an undefined conjunct in another clause cannot change a skipped
  candidate's result. A conjunct after the narrowing one in its own clause is
  never reached on a skipped candidate, because the clause short-circuits at
  the false narrowing conjunct.

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

- For every eligible application, narrowing SHALL yield exactly the result
  unnarrowed enumeration yields: the generated successor set SHALL equal
  the enumeration's, in the enumeration's candidate order, with the same
  transition identities, `StepEffect`s and results, the same
  `ContractUndetermined` findings for every `Undefined` or `Refused` clause
  on every candidate, the skipped ones included, and the same
  verdicts and refusals; FR-101's exploration order is unchanged. This
  exactness covers every result that is not a limit stop. The defined
  prefix is what makes this hold: every candidate narrowing skips has its
  postcondition `Completed(false)` under enumeration, never `Undefined` or
  `Refused`, so the enumeration records no finding on it.
- A candidate SHALL count against `max_candidates` exactly when the generator
  generates it, by FR-120's one counting rule: the candidates outside the
  interval are neither generated nor counted. Every argument vector, and every
  (candidate, result) pair the generator evaluates on a generated candidate,
  counts as FR-120 counts it. A run the enumeration completes
  under `max_candidates` completes with narrowing and returns the same
  `Exploration`.
- The charges FR-120 and this requirement define, `max_candidates` counts
  and evaluation meter charges alike, are normative and are counted on the
  narrowed enumeration: the candidates narrowing skips are neither counted
  nor evaluated, so they charge nothing. A limit outcome
  (`MaxCandidates`, or `EvaluationMeter` with its counter) is defined against
  that charge model, not against the work unnarrowed enumeration would do:
  a run stops exactly when the narrowed charge exceeds its limit, which is
  why FR-851-AC-2's rover item completes at 630 and stops at 629.

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
| FR-851-AC-2 | With `temporal.limits.max_candidates` 630 an analyze item over the rover fixture explores to completion and reports the same `exploration` as with no limit; with 629 it settles `incomplete` naming `max_candidates` in the initial state's expansion; with 300 it settles `incomplete` naming `max_candidates` at the `loseLidar` application. 630 is the charge of the largest expansion, the initial state's, by FR-120's counting rule: 12 argument vectors (one each for `loseLidar`, `brake`, `halt` and `enterFailsafe`, and 8 for `sample`'s three Boolean parameters), 301 `loseLidar` candidates (`mode` and `lidarOk` solved, `speed` 0 to 300) and one (candidate, result) pair for each, since every rover operation returns a `Boolean` and the first pair (`false`) of a kept candidate is accepted, and for `sample` one solved `voteWrong` candidate and one pair per argument vector, 8 + 8; `brake`, `halt` and `enterFailsafe` charge only their vectors, their preconditions being false: 12 + 301 + 301 + 16 = 630. No other expansion charges more: a `Degraded` state charges at most 12 + 101 + 101 + 16 = 230. | Test (TC-2411) |
| FR-851-AC-3 | With `loseLidar`'s bounds replaced by one that is not of the `self.f op e` form (`300 - self.speed >= 0`), or placed under `or` (`self.speed <= 300 or self.stopped`), the application is enumerated over the whole range: with `max_candidates` 630, under which the narrowed item completes, the item settles `incomplete` naming `max_candidates`. | Test (TC-2411) |
| FR-851-AC-4 | The rover fixture analyzed through `native-analyze/1` for the temporal clause `always (holds(not r.lidarOk) implies eventually holds(r.stopped))` under `fair weak brake` and `fair weak halt`, with no `temporal.limits`, settles `proved` with basis `exhaustive`, reaches no limit at EN-1's published defaults, and completes within 60 seconds in the test profile. | Test (TC-2411) |
| FR-851-AC-5 | Over a model whose object type `Cell` (population `cells`, universe `{k1}`) declares `a: Int[0, 3]` and `b: Int[0, 99]`, the query operation `nonZero(): Boolean` with an empty frame, body `true` and precondition `self.a != 0`, and the operation `set()` with frame `modifies a, b`, no result, and the clause `post A { self.nonZero() and self.a = 2 and self.b <= 10 }`, from the state with `a` 1 and `b` 50: the first conjunct is a dispatched call and so is not defined on every candidate, the defined prefix is empty, and neither `a` nor `b` is solved or narrowed. The `set` application generates all 400 candidates and charges 401 (one argument vector and 400 candidates); on each of the 100 candidates with `a` 0 the call's precondition is false, so the call is `Undefined` with reason `precondition-false` and the application records one `ContractUndetermined` for `A`; it has the 11 successors with `a` 2 and `b` 0 to 10, equal to forced full enumeration in successors, order and findings. The expansion, which also applies `nonZero` (one argument vector, its one unchanged candidate and its first result pair accepted), charges 404: it completes with `max_candidates` 404 and stops naming `MaxCandidates` with 403. The same model with the clause reordered to `post A { self.a = 2 and self.b <= 10 and self.nonZero() }` solves `a`, narrows `b` to 0 to 10, and the `set` application generates 11 candidates, charges 12 and records no finding, as its forced full enumeration records none, because every candidate with `a` 0 stops at `self.a = 2`. | Test (TC-2411) |

## Dependencies

- FR-120 owns successor generation, its solved assignment and its counting
  rule; this requirement adds the bound conjuncts to that rule.
- FR-126 and FR-142 call the generator; their explorations are unchanged.
- FR-623 owns the reductions a caller selects; narrowing is not one of them.
