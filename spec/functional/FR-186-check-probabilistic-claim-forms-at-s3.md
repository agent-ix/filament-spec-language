---
id: FR-186
title: "Check probabilistic claim forms at S3"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-326
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-254
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-740
    type: references
  - target: ix://agent-ix/filament-spec-language/US-022
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: depends_on
---
# FR-186: Check probabilistic claim forms at S3

## Description

S3 SHALL admit a `probabilistic` claim over a model subject, check its
scheduler, its form, its event or measure, its thresholds, its units and its
confidence parameters (ADR-024 §2, PM-8), and record a
`CheckedProbabilisticClaim` whose normal form is the content the claim
asserts: its subject, profile, scheduler, fairness set, form, `over` type and
the confidence parameters it states. The normal form is not an obligation identity (ADR-013
R-04): the obligation identity (O-09) is CG's structural identity (the preimage value, compared by equality) over the claim's S4 node
and its occurrence key. A checked claim is a claim of the FSpec FR-290 kind
`probabilistic-satisfaction`, which the claim type implies, so the record
carries no kind member. FSpec owns the grammar and the meaning of each form;
this requirement specifies what the compiler checks and records.

## Use case

A modeller writes `probabilistic P95 using ev on Service under Steady on
origin with alpha 0.01, beta 0.01, indifference 0.01 { quantile 0.95 of
(accumulate duration from holds(s.phase = Busy) until holds(s.phase = Done)
within 10) <= 5 ms }`. S3 records a quantile claim under the workload
`Steady` at those confidences, or refuses it with a diagnostic naming what
is wrong: a threshold outside `(0, 1)`, a threshold in a unit of another
dimension, an unbounded event, or confidence parameters that leave no
indifference region.

## Inputs

- A parsed `probabilistic` claim, the checked model, its workloads and
  rewards (FR-185), and the profile it names (FR-123).

## Outputs

```rust
pub struct CheckedProbabilisticClaim {
    pub name: String,                      // the claim's name
    pub span: Span,                        // the whole claim's locus
    pub subject: ModelSubjectRef,          // FR-125
    pub profile: DefinitionRef,            // the temporal profile it is checked under (FR-123)
    pub scheduler: ProbScheduler,
    pub fairness: Vec<FairnessConstraint>, // the set of `under every scheduler`; empty under a workload
    pub form: ProbForm,
    pub confidence: Option<Confidence>,
    pub over: Option<OverBinding>,
    pub normal_form: ClaimNormalForm,      // not an identity (ADR-013 R-04)
}
pub struct ModelSubjectRef { pub alias: String, pub package: DomainPackageRef }
pub struct OverBinding { pub name: String, pub value_type: ValueType }
pub enum ProbScheduler { Workload(String), Every } // the workload's package-wide name
pub enum Bound { AtLeast(Rational), AtMost(Rational) }
pub enum ProbForm {
    Probability { bound: Bound, event: Event },
    Quantile { q: Rational, measure: BoundedMeasure, threshold: Bound },
    MeanOfFraction { bound: Bound, fraction: FractionMeasure },
    LongRunFraction { bound: Bound, predicate: CheckedPredicate, weight: Option<Identifier> },
}
pub enum Event {
    Formula { formula: CheckedTemporalFormula, horizon: u64 },
    Compare { measure: BoundedMeasure, bound: Bound },
}
pub enum BoundedMeasure {
    Accumulate { reward: Identifier, from: CheckedPredicate, until: CheckedPredicate, within: u64 },
    Steps { from: CheckedPredicate, until: CheckedPredicate, within: u64 },
    Fraction(FractionMeasure),
}
pub struct FractionMeasure { pub predicate: CheckedPredicate, pub window: u64, pub weight: Option<Identifier> }
pub struct Confidence { pub alpha: Rational, pub beta: Rational, pub indifference: Rational }
```

## Behavior

Each refusal below that carries a bracketed string is an
`invalid_probabilistic_model` diagnostic with that cause; it retains the
claim's locus and names the offending part. Every such cause is in FSpec FR-272's closed set for that code (FSpec
FR-407).

A decimal literal denotes an exact rational where the grammar takes an
`exact-value`: a probability, mean-of-fraction or long-run threshold, a
quantile level, a confidence parameter and the magnitude of a
`quantity-literal`. A bare dimensionless threshold in an `expr` position,
such as an event comparison's `>= 0.999`, is the decimal primary of
[FR-764](FR-764-read-a-bare-decimal-literal-as-an-exact-decimal-value.md), the
same exact rational as `rational(999, 1000)` or `decimal(999, 3)`; a quantity threshold such as `5 ms` or
`0.005 s` is a `quantity-literal` and keeps its decimal magnitude. S3 reduces
every `exact-value` under the written-value bound and limit outcome that
FR-185 states (setting pending, QSL-883).

### Kind and subject

- S3 SHALL require a model subject (FR-125).
- If a claim's subject, its `using` profile alias, its `under W` workload, a
  reward it names or an operation in an `under every scheduler` fairness set
  names nothing that exists, S3 SHALL refuse the claim
  `missing_declaration`/`missing-name`, located at that name. A subject name
  that declares a type but is no `model` alias is a trace subject, not a
  missing name.
- If a `probabilistic` claim has a trace subject, S3 SHALL refuse it
  [`trace-subject`] with a diagnostic that names
  window aggregates (FR-194) as the form for observed statistics.
- If an `over` parameter's type is not an object type that some population
  of the subject's model admits, S3 SHALL refuse the claim
  [`over-not-object-type`], located at the
  parameter's name. FR-125 "Instancing" binds `over` to the objects of the
  subject's universes, so a scalar type, or an object type no population of
  the model admits, has no instance and the claim would hold or fail
  vacuously. The parameter's type is part of the claim's normal form (see
  "Normal form").
- A claim SHALL be checked under a temporal profile (FR-326). If the profile
  its `using` alias selects is not a temporal profile, S3 SHALL refuse the
  claim [`not-temporal-profile`], naming the profile.
- S3 SHALL require exactly one scheduler: `under W` naming a workload of the
  subject's model (FR-185), or `under every scheduler`. A workload name is a
  package-wide identifier, so `W` resolves to the one workload of that name,
  and the checked claim carries that name; S4 resolves it to the workload's
  `probabilistic`/`workload` node (FR-740), which the claim's obligation
  identity binds.
- If `under W` names a workload declared on another model than the claim's
  subject, S3 SHALL refuse the claim [`foreign-workload`], naming the
  workload and its model.
- A claim under a workload carries no fairness set (ADR-024 PM-7); a
  fairness set belongs to `under every scheduler`, and the checked claim
  carries it in `fairness`, each constraint once. The grammar's
  `under ident` takes no fairness items, so source text cannot state one
  under a workload: S1 refuses `under W fair weak Op` `invalid_syntax`, and
  `SchedulerForm::Workload` (FR-392) cannot hold one. FSL has no producer of
  a workload scheduler with a fairness set, so S3 has no
  `fairness-under-workload` check; FSpec FR-407 defines that cause, and
  STD-188 asks FSpec to settle whether the grammar or the cause changes.

### Events and measures

- An `Event::Formula` SHALL be a formula with activation `on origin` and only
  interval operators (ADR-018 TP-2), under a bounded profile, where S3 SHALL
  record its step horizon `h` (ADR-014 TR-4), or under the timed profile as a
  bounded-window formula whose every interval is a closed time interval
  (FR-254, FSpec FR-407), where S3 SHALL record its time horizon.
- S3 SHALL judge an event formula's operators by the operator admission of
  its claim's profile (FR-326) before any probabilistic check, exactly as it
  judges the same formula in a `temporal` clause under that profile: under a
  bounded or the timed profile an operator written with no interval, or with
  a lower-bounded interval `[a,*]`, is refused
  `unsupported_construct`/`expression-form` at the operator's or interval's
  span, and S3 SHALL NOT report `unbounded-event` for it. The reason is that
  one spelling under one profile gets one refusal wherever it stands, which
  is the rule FSpec FR-407 and FR-048 state.
- If, under the infinite-trace profile, an event is not one of FR-195's
  unbounded reachability forms, S3 SHALL refuse the claim
  [`unbounded-event`] with a diagnostic stating that a probabilistic event
  needs a finite window. An event whose root is one `eventually`, `until` or
  `always` written with no interval is checked as a `Reach` form, not as an
  `Event::Formula`, and FR-195 judges its operands (an operand that is not a
  `holds(P)` is refused `not-state-predicate`, not `unbounded-event`). Every
  other event under that profile, such as `eventually[0, 5] holds(P)`,
  `eventually[2, *] holds(P)` or `holds(P) and eventually holds(Q)`, is
  refused `unbounded-event`.
- If the sum of interval bounds that makes up the horizon exceeds the range
  of `u64`, S3 SHALL refuse the claim as a resource-exhausted checking limit
  (work budget), reporting the sum and located at the operator whose bound
  makes the sum overflow.
- `accumulate R from holds(A) until holds(B) within h` SHALL name a reward
  `R` declared by at least one operation of the model; a step of an
  operation that declares no `R` contributes 0. `steps …` and `fraction holds(P) over [0, h]`
  SHALL be admitted with `h` a natural number, and `weighted by R` SHALL
  name a reward.
- Every declaration of a reward name by the operations of one model SHALL have
  one value type. If operations of the subject's model declare `R` with
  different value types, S3 SHALL refuse a claim that names `R`
  `ambiguous_declaration`/`ambiguous-name`, naming `R`. FR-185 forbids only a
  second `R` on one operation.
- A comparison `M <= c` or `M >= c` SHALL have a threshold `c` that is a
  literal of `M`'s type: a natural number for `steps`, the reward's dimension
  for `accumulate`, a rational for `fraction`. If `c` is not a literal, or its
  type is not `M`'s (a rational for `steps`, a duration where a natural number
  is due), S3 SHALL refuse the claim `ill_typed`/`type-mismatch`.
- If a `fraction` comparison's threshold is not strictly between 0 and 1, such
  as `0`, `1`, `3/2` or a negative value, S3 SHALL refuse the claim
  [`fraction-bound-out-of-range`], naming the
  bound. A bound on a fraction inside an event is no probability threshold, so
  the diagnostic SHALL NOT name the qualitative form: `qualitative-threshold`
  (FSpec FR-407) is the refusal of a probability, quantile, mean-of-fraction or
  long-run threshold of 0 or 1.

### Thresholds and units

- Every probability threshold `θ`, quantile level `q`, mean-of-fraction
  threshold and long-run threshold SHALL be an exact rational with
  `0 < θ < 1`.
- If a probability, quantile, mean-of-fraction or long-run threshold is 0 or
  1, S3 SHALL refuse the claim [`qualitative-threshold`], naming the
  qualitative form (an ADR-018 TP-2 claim, or a possible property).
- A quantile SHALL be over an `accumulate` or `steps` measure, and a
  `mean of` SHALL be over a fraction measure.
- If a quantile is over a fraction measure, S3 SHALL refuse the claim
  [`quantile-over-fraction`].
- A threshold with a dimension SHALL have the reward's dimension. A
  threshold in another unit of that dimension SHALL convert exactly by
  FSpec FR-142.
- If a threshold has another dimension than the reward's, S3 SHALL refuse
  the claim (`ill_typed`/`unit-mismatch`), naming the dimension.

### Confidence parameters

- A claim MAY state `alpha`, `beta` and `indifference`, each exactly once,
  all three or none.
- If a claim states one or two of them, S3 SHALL refuse it
  [`partial-confidence`], naming the missing parameter.
- If a claim states a parameter more than once, S3 SHALL refuse it
  [`partial-confidence`], naming the repeated parameter.
- When stated, S3 SHALL require `0 < α < 1`, `0 < β < 1`, `0 < ι`,
  `θ − ι > 0` and `θ + ι < 1` for the claim's threshold `θ`.
- If a stated parameter breaks one of those bounds, S3 SHALL refuse the
  claim [`invalid-confidence`], naming the
  parameter.

### Activation

- A claim's activation SHALL be `on origin` or `on each (x: T)`, optionally
  with `when (g)`. Both are checked; neither is refused as not yet
  implemented.
- For `on each (x: T)`, `T` SHALL be an object type that some population of
  the subject's model admits; otherwise S3 SHALL refuse the claim
  [`over-not-object-type`] at the parameter's name, for the reason the
  `over` rule above gives: a type with no instance would make the claim hold
  or fail vacuously.
- The guard `g` SHALL be a Boolean state expression over `x`, the `over`
  parameter and the subject, checked as FR-125's activation guard; a guard
  of another type SHALL be refused `ill_typed`/`type-mismatch` at the guard.
- S3 SHALL record the activation in the checked claim and in its normal form
  as `Each{parameter type, guard}`, so two claims differing only in their
  activation are distinct obligations.
- The meaning is FSpec FR-407 "Activation": FSL instantiates the claim per
  key of the universe of `T`'s population, in product with any `over`
  parameter (FR-125 "Instancing", including `NoInstance` for an empty
  population, settled `inconclusive`); each instance's event or measure is
  read from the run's first activating position, conditioned on the run
  activating; and the item holds when every instance holds, naming the first
  failing instance in key order.

### Time positions

- An `elapsed` measure, an `expected elapsed` claim and a time quantity used
  as a horizon or interval bound (`within 5 ms`, `over [0, 5 ms]`,
  `[0 ms, 4 ms]`) SHALL be admitted over a timed subject (a subject whose
  model declares a time binding, FR-233 `ModelTime`), with the horizon or
  bound converted exactly into the model's time unit and recorded as that
  exact rational.
- Over a subject whose model declares no time binding, each of them SHALL be
  refused `ill_typed`/`type-mismatch`, located at the `elapsed` measure, at
  `expected elapsed`'s quantity or at the time quantity, with a message
  stating that the subject's model declares no time binding. There is no
  elapsed time to measure, which is a type fact of the subject, not a missing
  feature.
- A claim under the timed profile (`filament.temporal.timed/v1`) over a timed
  subject SHALL be checked by that profile's operator admission (FR-326,
  FSpec FR-407-AC-11). Over a subject whose model declares no time binding,
  its clock is the model's `model-steps` binding, which the timed profile
  does not read: S3 SHALL refuse the claim
  `invalid_timed_model`/`model-time-on-untimed-model`, naming the model,
  at the `using` alias (FSpec FR-252). No claim is refused with an FSL-only
  cause for the timed profile.

### Quantity positions

Every position of a probabilistic declaration or claim that the grammar lets
hold a quantity literal SHALL be read by a check. A quantity of another
dimension than the position's SHALL be refused `ill_typed`/`unit-mismatch`,
naming the dimension, at the literal; a quantity literal in a position that
takes no quantity SHALL be refused `ill_typed`/`type-mismatch` at the
literal. No position refuses a quantity literal as not yet implemented.

| Position | Dimension it takes | Owner |
| --- | --- | --- |
| `accumulate R` measure threshold | `R`'s dimension | this requirement |
| quantile threshold (`quantile q of (M) <= c`) | the measure's dimension | this requirement |
| `expected accumulate R` threshold, quantity reward | `R`'s dimension | FR-195 |
| `expected elapsed` threshold | time | FR-195 |
| measure horizon (`within h`) and fraction window (`over [0, h]`) | time, over a timed subject; otherwise a natural number | this requirement |
| timed interval bound of an event | time | this requirement, FR-254 |
| `uniform[a, b]` and `discrete { d: w }` delay values | time | FR-185 |
| reward value (`reward R on Op = e`) | any quantity; it gives `R` its type | FR-185 |
| fraction threshold, `steps` threshold | none: a rational or a natural number | this requirement |
| probability threshold, quantile level, confidence parameter, random or workload weight | none; the grammar takes an `exact-value`, so a quantity there is `invalid_syntax` at S1 | FR-185, this requirement |

### Normal form

- S3 SHALL record each checked claim's normal form: the content the claim
  asserts, which is its subject, profile, scheduler, fairness set, `over`
  binding type, activation, form and, when stated, confidence parameters. The normal form
  SHALL bind no source span, no method and no budget.
- S3 SHALL give two checked claims equal normal forms exactly when they
  agree on the content below, and different normal forms when they differ in
  any of it.
- Content is: the subject, which is the `model` alias as written and the
  domain package it selects; the profile; the scheduler (the workload's name,
  its model and its weights, or `every`); the fairness set; the `over` type;
  the form, which is every bound, threshold, level, window, reward and
  predicate; and each stated confidence parameter.
- Not content, so that a change to it leaves the normal form unchanged: the
  order in which a workload's weights, a fairness set's constraints and a
  reward's operations are written; the `over` binder's name; the operand order
  of a commuted equality in a predicate (the predicate is recorded in its
  normal form, FR-174); and the unit a threshold is written in, since a
  threshold is recorded in the reward's unit, so `0.005 s` and `5 ms` give one
  normal form.
- The normal form is not an identity (ADR-013 R-04). The obligation identity
  stays CG's structural identity over the claim's S4 node and its occurrence key (O-09),
  so two declarations with equal content have equal normal forms and are two
  obligations.
- S4 SHALL emit a checked claim as its `claim`/`probabilistic_claim` node with
  one declared `claim` occurrence and one `probabilistic_claims` row (FR-498);
  the random parameters, rewards, workloads and delays of the same package are
  emitted beside it (FR-185 "Package emission", FR-740).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-186-AC-1 | ADR-024 §7's four claims check: `NoFault` as `Probability{AtLeast(999/1000), Formula{horizon: 1000}}`; `P95` as `Quantile{q: 19/20, Accumulate{duration, within: 10}, AtMost(5 ms)}`; `LongRun` as `LongRunFraction{AtLeast(999/1000)}`; `Monthly`, written with the threshold `rational(999, 1000)`, as `Probability` over `Compare{Fraction{window: 10000}, AtLeast(999/1000)}`; each with `Workload(Steady)` and its stated `Confidence`. | Test (TC-621) |
| FR-186-AC-2 | `P95` with threshold `0.005 s` checks and records `5 ms`; with threshold `5 m` (length) it is refused `ill_typed`/`unit-mismatch` naming the dimension. `probability >= 1 [E]` and `probability >= 0 [E]` are refused `qualitative-threshold` naming the qualitative form. `quantile 0.95 of (fraction holds(v.up) over [0, 10]) <= rational(1, 2)` is refused `quantile-over-fraction`. | Test (TC-621) |
| FR-186-AC-3 | `probability >= 0.99 [always holds(n.healthy)]` under a bounded profile is refused `unsupported_construct`/`expression-form` at `always` (FR-326), not `unbounded-event`; `probability >= 0.99 [eventually[0,100] always[0,10] holds(n.healthy)]` checks with horizon 110. `NoFault` (on `Health`) with `under Throttle`, where `Throttle` is declared `on Service`, is refused `foreign-workload` naming `Throttle` and `Service`; `NoFault` with `under Steady`, `Steady` declared `on Health`, checks. A claim over a trace subject is refused `trace-subject`, its diagnostic naming window aggregates; a claim `using` a profile that is not temporal is refused `not-temporal-profile`; an event under the infinite-trace profile that is not one of FR-195's three `Reach` shapes, such as `eventually[0, 5] holds(n.healthy)`, is refused `unbounded-event`, while the three shapes check as FR-195-AC-1 states. `under every scheduler fair weak Node::tick` checks and carries the fairness set. `under Steady fair weak Node::tick` is refused at S1 `invalid_syntax`, because `under ident` takes no fairness items. | Test (TC-621, TC-2063) |
| FR-186-AC-4 | `NoFault` with `indifference 0.002` is refused `invalid-confidence` (`θ + ι = 1.001`); with only `alpha` stated it is refused `partial-confidence`; with `alpha 0.01, alpha 0.05, beta 0.01, indifference 0.0005` it is refused `partial-confidence` naming `alpha`; with `alpha decimal(1, 4294967295)` it is refused with the `rational-bits-exceeded` limit refusal (`LimitExceeded`, naming `s3.rational_bits`), as FR-185 states; with none stated it checks with `confidence: None`. `NoFault` at `alpha 0.01` and at `alpha 0.05` have different normal forms; `NoFault` under `Steady` and under a second workload `Burst` have different normal forms. The same workload with its weight lines reordered, a fairness set with its constraints reordered, a reward declared on its operations in another order, the same claim with its `over` binder renamed, and a predicate with a commuted equality each keep one normal form. A claim that differs only in its subject (another `model` alias), and one that differs only in its `over` type, each have another normal form. Two declarations of equal content have equal normal forms. | Test (TC-621) |
| FR-186-AC-5 | `steps ... >= 3` checks with threshold 3; `steps ... >= rational(1, 2)` and `steps ... >= 5 ms` are each refused `ill_typed`/`type-mismatch`; a threshold that is no literal is refused `ill_typed`/`type-mismatch`; `fraction holds(P) over [0, 10] <= rational(1, 4)` checks, and `>= rational(1, 1)`, `>= rational(0, 1)` `>= rational(3, 2)` and a negative bound are each refused `fraction-bound-out-of-range` with a diagnostic that does not name the qualitative form. An event whose interval bounds sum beyond `u64` (two `[0, 2^63]` intervals) is refused as a resource-exhausted checking limit located at the operator whose bound overflows. A claim `over (n: Integer)` is refused `invalid_probabilistic_model`/`over-not-object-type` located at `n`, as is one over an object type that no population of the subject's model admits; the same claim over an object type a population admits checks. A package holding a checked claim and no other probabilistic declaration is emitted by S4 with the claim's node (FR-498); with a reward declared after it, S4 emits the claim node and the reward's `probabilistic`/`reward` node (FR-740). | Test (TC-621, TC-2062) |
| FR-186-AC-6 | A claim over `on Nowhere`, one `using zz`, one `under Nope`, one that accumulates a reward no operation declares, and one `under every scheduler fair weak Node::nope` are each refused `missing_declaration`/`missing-name` located at that name. A claim that accumulates a reward `R` that one operation of the model declares as `Millis` and another as an integer is refused `ambiguous_declaration`/`ambiguous-name` naming `R`; with `R` of one value type on both it checks. | Test (TC-621) |
| FR-186-AC-7 | Under a bounded profile, `probability >= 0.99 [eventually[2, *] holds(n.healthy)]` is refused `unsupported_construct`/`expression-form` at the interval and `[always holds(n.healthy)]` at `always`, each with the refusal the same formula gets in a `temporal` clause under that profile, and neither is refused `unbounded-event`; under the infinite-trace profile `[eventually[2, *] holds(n.healthy)]` is refused `unbounded-event`; under the timed profile over a subject with a time binding, `[eventually[0 ms, 4 ms] holds(n.healthy)]` checks with time horizon 4 ms and `[eventually holds(n.healthy)]` is refused `unsupported_construct`/`expression-form`. | Test (TC-2063) |
| FR-186-AC-8 | Over the service model, `on each (m: Service::Server) { probability >= 0.9 [ holds(P) ] }` checks and records activation `Each` with parameter type `Service::Server` and no guard; with `when (m.phase = Service::Phase::Idle)` it records that guard; the two checked claims have unequal normal forms. Neither is refused `not-yet-implemented`. | Test (TC-2166) |
| FR-186-AC-9 | `on each (k: Integer)` is refused `over-not-object-type` at `k`, and `on each (m: Service::Server) when (1)` is refused `ill_typed`/`type-mismatch` at `1`. | Test (TC-2166) |
| FR-186-AC-10 | Through `fsl analyze` over the `Link` model, `probabilistic E using inf on M under Even on each (k: M::Msg) when (not k.delivered) { probability >= rational(9, 10) [ eventually holds(k.delivered) ] }` settles `proved`, never `unsupported`, with one instance per `Msg` key of the start snapshot: the one key `m` activates at position 0, so its proof basis equals the `on origin` claim's (`391/400`). | Test (TC-2167) |
| FR-186-AC-11 | Over the untimed service model, `expected elapsed until holds(P) <= 3 ms`, the measure `elapsed from holds(A) until holds(B) within 5` and the window `steps from holds(A) until holds(B) within 5 ms` are each refused `ill_typed`/`type-mismatch` at `expected elapsed`'s quantity, at the measure and at `5 ms`; none is refused `not-yet-implemented`. | Test (TC-2168) |
| FR-186-AC-12 | Over the timed `Retx` subject (`time dense unit ms`) under the timed profile, `expected elapsed until holds(c.late) <= 3 ms`, the measure `elapsed from holds(not c.late) until holds(c.late) within 5 ms >= 1 ms` and the window `steps from holds(not c.late) until holds(c.late) within 5 ms >= 1` each check, the `expected elapsed` threshold recorded as 3 and each horizon as 5, in milliseconds. | Test (TC-2169) |
| FR-186-AC-13 | A claim under the timed profile over the untimed service model is refused `invalid_timed_model`/`model-time-on-untimed-model` naming `Service`, at the `using` alias, and not `timed-event-profile`; under the timed profile over `Retx`, `probability >= 0.99 [ eventually[0 ms, 4 ms] holds(c.late) ]` checks. | Test (TC-2170) |
| FR-186-AC-14 | For each quantity row of "Quantity positions" a quantity of the position's dimension checks and a quantity of length (`5 m`) is refused `ill_typed`/`unit-mismatch` naming `Length` at the literal; `fraction holds(P) over [0, 10] <= 5 ms` and `steps from holds(A) until holds(B) within 5 >= 5 ms` are refused `ill_typed`/`type-mismatch` at `5 ms`; no position refuses a quantity literal `not-yet-implemented`. | Test (TC-2171) |

## Dependencies

- ADR-024 §2 PF-1 to PF-10, PM-7, PM-8, QS-2, QS-5, QS-6; ADR-018 TP-2,
  SM-2; ADR-013 O-09; ADR-014 TR-4.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (profiles, fairness sets, interval operators),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (model subject), [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md).

## References

- FSpec half, which owns the claim grammar, the forms and the FR-290 kind:
  FSpec FR-407 and FR-290 (Linear STD-137).
- [FR-392](FR-392-build-the-probabilistic-claim-form-at-s2.md): the S2 form this
  requirement checks.
- Owning ticket: Linear QSL-371.
