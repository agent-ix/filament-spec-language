---
id: FR-450
title: "Replay a hybrid trace against the checked hybrid model"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-447
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-250
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-290
    type: references
  - target: ix://agent-ix/filament-specification/FR-193
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-146
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-364
    type: depends_on
---
# FR-450: Replay a hybrid trace against the checked hybrid model

## Description

When FSL reads a hybrid trace, whether a plugin provider's or its own
solver's, it SHALL replay that trace against the claim's checked hybrid model
(FR-447) by FSpec FR-193 "Trace replay" before it settles the item. Static
replay checks each trace in exact rational interval arithmetic within the
trace's tolerance, using the caller's `HybridReplayMeter`, and records its
outcome on the `inconclusive` record. Exact replay decides every eligible
point trace through modes with polynomial flows exactly when its work fits the
caller's meter. A trace it reproduces settles the item
`refuted` with the trace as its counterexample. If the meter is exhausted,
replay settles the item `incomplete` with the exhausted counter and its
accounting setting, without a verdict or counterexample. This is ADR-026 HY-2's
one refutation path for a hybrid item.

## Use case

A plugin δ-complete solver reports `δ-sat` for FR-447's `SlowEnough` with a
trace whose boxes overlap the guard of `Moving`'s transition within its
`delta` of 1/100. FSL replays it statically, finds it consistent, and keeps
it on the `inconclusive` record. A second provider reports the point trace
that starts at `v = 20`, `a = 2`, dwells 10 s and jumps to `Stopped` at
`v = 0`. Against `always [0, 12] holds(h.v > 0)` FSL replays it exactly:
`Moving`'s flow is polynomial, `v(t) = 20 - 2t` reaches 0 at `t = 10`, and the
claim is false there, so the item settles `refuted` with that trace as its
counterexample.

## Inputs

- A `HybridTrace` (FR-250), from a `solver-inconclusive` cause or from a
  `HybridCounterexample` in a provider's `refuted` entry (FSpec FR-193
  "Solver result on the wire"), read through the plugin reader (FR-290) or
  returned by an in-process provider.
- The item's `CheckedHybridClaim` and its `CheckedHybridModel` (FR-447).
- The caller's FR-255 `accounting.work_units` bound, held by the
  `HybridReplayMeter` used by both replay functions.

## Outputs

```rust
pub enum StaticReplay {
    Consistent,
    Inconsistent { at: JsonPointer, check: StaticCheck }, // the first failing check
}
pub enum StaticCheck { Initial, Invariant, Time, Jump, Loop, End } // FSpec FR-193 static checks 1 to 6
pub struct ReplayRecord {
    pub static_outcome: StaticReplay, // written as `replay.static`
    pub exact: Option<ExactRecord>,   // written as `replay.exact` when present
}
pub enum ExactRecord {
    NotExact,
    NotReproduced { at: JsonPointer, check: ExactCheck },
}
pub enum ExactReplay {
    NotExact,                                   // not a point trace, or a mode without a polynomial flow
    Reproduced(HybridCounterexample),
    NotReproduced { at: JsonPointer, check: ExactCheck },
}
pub enum HybridReplayRefusal {
    // Also carries the existing trace and package admission refusals.
    ResourceExhausted { counter: WorkUnits },
}
pub struct HybridReplayMeter {
    pub work_units: u64, // caller's configured bound
    pub consumed: u64,   // successfully charged work units
}
pub enum ExactCheck { Initial, Dwell, Jump, Loop, Claim } // FSpec FR-193 exact checks 1 to 5; check 4 also holds the `end` dwell
pub struct HybridCounterexample {
    pub kind: HybridCounterexampleKind,         // Formula | UndefinedEvaluation{where, cause}
    pub trace: HybridTrace,
}
pub enum HybridTraceDisagreement {
    NotReproduced { counterexample: HybridCounterexample },
    NotExact { counterexample: HybridCounterexample },
}
```

`static_replay(graph, claim, trace, meter: &mut HybridReplayMeter) ->
Result<StaticReplay, HybridReplayRefusal>` and
`exact_replay(graph, claim, trace, meter: &mut HybridReplayMeter) ->
Result<ExactReplay, HybridReplayRefusal>` take the caller's meter. The
`InconclusiveCause::SolverInconclusive { trace: Option<HybridTrace>, replay:
Option<ReplayRecord> }` carries the static outcome and, when exact replay ran
without reproducing the trace, its exact outcome. The stored FR-193 `replay`
object has a required `static` member and an optional `exact` member; the
`exact` member is `not-exact` or `not-reproduced` with its failing pointer and
check. It is not a bare `StaticReplay`. A reproduced trace
settles as `TerminalValue::Refuted` with its `HybridCounterexample` as the
item's `Counterexample`.

## Behavior

### When replay runs

- FSL SHALL replay every `HybridTrace` it keeps or settles from, after the
  correspondence checks of FR-250 and FSpec FR-193 "Hybrid trace" admit it,
  whether a plugin provider or an in-process provider returned it.
- Replay SHALL read the `CheckedHybridModel` and `CheckedHybridClaim` of the
  checked package, by the variable and mode indices FR-250's `HybridTrace`
  carries, and SHALL read no source.
- A negative `delta` SHALL refuse the result `invalid_result`/`invalid-value`
  at `/delta`, and a `loop_to` past the trace's last state SHALL refuse it
  at `/loop` (FSpec FR-193). A trace carrying both `end` and `loop_to` SHALL
  refuse it `invalid_result`/`invalid-value` at `/end`, since a lasso has no
  last dwell to cut.

### Static replay

- FSL SHALL run static replay on every trace by FSpec FR-193 "Trace replay"
  checks 1 to 6, in exact rational interval arithmetic over the trace's
  intervals, each comparison widened by `delta` (0 when `None`), and SHALL
  return `Consistent` or `Inconsistent` at the first failing check in trace
  order, with the member's pointer. `static_replay` and `exact_replay` take
  the caller's `HybridReplayMeter` and return a `HybridReplayRefusal` if the
  meter denies a charge.
- The interval evaluator SHALL be stack-safe: it SHALL evaluate nested terms
  and recursive calls iteratively or with an explicit stack, with no recursion
  depth limit. Neither replay function nor a helper it invokes SHALL impose a
  fixed evaluation or iteration cap. One work unit is one static interval
  evaluation step, including each definite recursive call expansion, or one
  exact replay work step. Each exact replay `ClaimSegment` materialization is
  a work step, including segments created by expanding a lasso repetition to
  reach a finite interval. Before each charge, if `consumed` equals the
  configured `work_units` bound, replay SHALL return
  `HybridReplayRefusal::ResourceExhausted { counter: WorkUnits }`; otherwise,
  it SHALL increment `consumed` by one. The bound is FR-255's
  `accounting.work_units` setting.
- Interval evaluation SHALL take every `HybridTerm` form: `Variable` over
  the state's interval, `Constant` as a point, `Arithmetic` and `Negate` by
  exact interval arithmetic, a division whose divisor interval contains 0 as
  the whole real line, `Compare` as definitely true, definitely false or
  undecided, `Logic` and `Not` over those three values, `Conditional` as the
  hull of the branches its condition does not exclude, and `Call` by
  evaluating the function's checked body over the argument intervals. A
  recursive call SHALL be evaluated only while its `decreases` measure
  (FR-062) evaluates to a definite interval value: a recursive call whose
  measure interval is undecided, or whose base-case condition is undecided,
  SHALL take the whole value range of the function's result type (the
  real line for an unbounded numeric type, undecided for `Boolean`). A call
  with a definite measure and base-case condition SHALL continue under the
  supplied meter; its termination comes from FR-062's decreasing measure and
  caller fuel, not a fixed evaluation cap.
- A check SHALL fail only when its value is definitely false.
- Static replay SHALL settle no value: FSL SHALL keep the outcome as
  `SolverInconclusive{trace, replay: Some(outcome)}`, written on the stored
  record as FSpec FR-193's `replay` member (`static`, and `exact` when exact
  replay ran and did not reproduce). FSL SHALL refuse a provider entry whose
  cause carries `replay` with `invalid_result`/`unknown-member` at
  `/cause/replay`.

### Exact replay

- FSL SHALL run exact replay on a trace that FSpec FR-193 calls a point
  trace through modes with polynomial flows: `delta` 0 or `None`, every jump
  carrying `before`, every interval a point, and in every mode the trace
  dwells in each rate built from `Constant`, `Variable`, `Negate`, `+`, `-`
  and `*`, after expanding every `Call` whose function is not recursive, with
  no cycle in the relation "`y` occurs in `x`'s rate", and with no recursive
  call in any invariant, guard, reset, initial condition, activation
  condition, capture or claim atom the replay evaluates. Any other trace
  SHALL give `NotExact`.
- FSL SHALL compute each dwell's solution as polynomials in time with exact
  rational coefficients, integrating the rates in that relation's order from
  the state's point, and SHALL decide FSpec FR-193's exact checks 1 to 5 over
  the reals exactly: an atom along a dwell is a univariate rational function
  of time, and FSL SHALL decide its sign on an interval by exact real root
  isolation, with no floating-point step. For a trace with `end`, the last
  dwell, of length `d = end.time - last.time >= 0`, SHALL be decided as check
  2 decides a dwell to a `before`: the solution at `d` equals `end` and the
  mode's invariant holds on `[0, d]`, a failure giving `NotReproduced` at
  check `Loop` with the pointer `/end`; check 5 then reads the prefix ending
  at `end`.
- Exact replay SHALL not substitute an approximate evaluation or a verdict
  when its meter denies a charge.
- Exact replay SHALL decide an `on each` claim under `model-time` whose
  body, guard or captures read the activation position's state (FSpec
  FR-193 "Activations and captures"), which creates one obligation at every
  activation instant `τ` of a dwell, each with its own captured values. It
  SHALL treat `τ` as a second real variable: along a pair of dwells (the
  activation's and the one an atom is read in) every activation-state read,
  capture and atom is a bivariate rational function of `(τ, t)` with exact
  rational coefficients, and replay SHALL decide whether some activation
  instant's obligation is false by exact real algebraic decision over the
  two variables (a cylindrical decomposition with exact root isolation in
  each cell, no floating-point step). On a `loop_to` trace the activation
  instants are infinitely many, and replay decides finitely many periods:
  - When the obligation reads no past operator, its truth under
    `model-time` depends only on the behaviour from its activation instant
    on and is shift-invariant: the periodic behaviour repeats with the
    loop's period `P`, so the obligations activated at `τ` and `τ + P` in
    the periodic part agree, and replay SHALL decide the activation
    instants of the prefix and of one period, which decides them all.
  - A past operator (`once`, `historically`, `since`, `triggered`) in the
    body, guard or captures SHALL read the behaviour from position 0, the
    start of the trace, not from the activation instant: its history at an
    activation instant `τ` is the whole replayed behaviour up to `τ`. Such
    an obligation is not shift-invariant while its history still reaches
    into the prefix. Its past reach `R` bounds how far back it reads, as
    ADR-018 SM-8 bounds it for the discrete evaluator: a past operator
    whose interval has upper bound `b` adds `b`, one with an open interval
    `[a, *]` (an unbounded operator is `[0, *]`) adds `a + P`, a future
    operator adds nothing, and nested operators add up; `R` is the largest
    reach over the body, the guard and the captures. An activation more than
    `R` past the end of the prefix has a history that reads only periodic
    behaviour, so from period `1 + ceil(R / P)` after the prefix on, the
    obligation activated at `τ + P` agrees with the one activated at `τ`.
    Replay SHALL decide the activation instants of the prefix and of the
    first `1 + ceil(R / P)` periods, the instant that closes the last
    period included, which decides them all. A body whose
    one past operator is
    unbounded, or bounded with `0 < b <= P`, is decided over the prefix
    and two periods; `once [0, 1] once [0, 1]` over a period of 1 has
    `R = 2` and is decided over the prefix and three periods.

  The decisive instant is the infimum of the instants at which some
  obligation's prefix is a bad prefix or its capture or atom has no value.
  When that set has a least element the infimum is that instant; when it
  is open below, as the no-value instants of a capture that is out of
  range exactly for the activation instants in `(1/2, 1)` are (a capture
  is evaluated at its activation instant), there is no earliest instant
  and the infimum, the open stretch's start, is the decisive instant. For
  a formula failure the decisive instants are the ends of the bad
  prefixes, not the activation instants. Replay SHALL report the decisive
  instant as an exact time, FSpec FR-193's `where.time` interval: a point
  interval when it is rational, otherwise the isolating interval with
  rational ends that exact root isolation found for it, holding it and no
  other algebraic number of the decision. A capture value outside its
  type's value range at an activation instant is `UndefinedEvaluation`
  with cause `outside-capture-range`.
- Exact replay SHALL decide a fairness constraint's enabledness (FSpec
  FR-193 "Fairness", "Resets") for a transition whose reset leaves a
  variable free (a non-deterministic reset): the transition is enabled at a
  dwell instant `t` when its guard holds and some post-jump state satisfies
  the reset and the target mode's invariant. Replay SHALL decide that
  existence exactly, by real quantifier elimination over the post-jump
  variables the reset leaves free, giving the set of instants of the dwell
  at which the transition is enabled as a finite union of intervals with
  real algebraic ends, and SHALL read the constraint's weak or strong
  condition over the periodic behaviour from that set.
- A trace exact replay does not decide is `NotExact` exactly when it is not
  a point trace, dwells in a mode without a polynomial flow, or would
  evaluate a recursive call (FSpec FR-193 "Exact replay"); no other claim
  form or reset gives `NotExact`.
- Check 5 SHALL evaluate the `CheckedHybridClaim` over the replayed
  behaviour by FSpec FR-193 "Hybrid claims": with its clock, its activation
  and every obligation it creates, its captures, and its fairness set, a
  `loop_to` trace's periodic behaviour admitted only when it satisfies that
  set. A finite prefix SHALL reproduce only as a bad prefix: the claim false
  on every behaviour that extends it, every position past the prefix's end
  read as unknown.
- When checks 1 to 4 hold, exact replay SHALL decide by the decisive
  instant of FSpec FR-193 "Trace replay": the infimum of the instants at
  which the prefix up to it is a bad prefix of the claim or an expression
  the replay evaluates has no value (a division by zero, an argument outside its
  parameter's range). If an expression has no value at that instant, it
  SHALL return `Reproduced` with kind `UndefinedEvaluation{where, cause}`,
  `where` naming that instant's state, its time stamp and the expression's
  locus; otherwise `Reproduced` with kind `Formula`. When the decisive
  instant is the infimum of an open set, the kind is `UndefinedEvaluation`
  when an expression has no value at every instant of some interval
  `(d, d + e)`, `d` the decisive instant and `e > 0`, and `Formula`
  otherwise; `where` then names the state whose dwell holds `d` and the
  time `d`. A `loop_to` trace
  reproduces with kind `UndefinedEvaluation` at the infimum of the instants
  of its periodic behaviour with no value, else with kind `Formula` when check 5
  holds. Otherwise it SHALL return `NotReproduced` at the first failing
  check.

### Settlement

- A provider's claimed counterexample SHALL agree with the reproduced one
  (FSpec FR-364 step 6, FSpec FR-193 "Solver result on the wire") when both
  kinds are `Formula`, or both are `UndefinedEvaluation` with equal
  `where.position`, equal `where.locus`, equal `cause` and a claimed
  `where.time` interval that contains the reproduced instant. A trace that
  reproduces with a kind that does not agree with the claim SHALL be
  `NotReproduced` with `check` 5 and `at` the pointer of the trace state
  holding the reproduced decisive instant (`/start` for position 0,
  `/jumps/{k-1}` for position `k`), and settles as `NotReproduced` below:
  `HybridTraceDisagreement::NotReproduced` carrying the provider's complete
  counterexample.
- A `Reproduced` trace SHALL settle the item `refuted` (ADR-018 V-4) with
  its `HybridCounterexample` as the item's `Counterexample` and one
  `counterexamples` entry, with the cause `UndefinedEvaluation{where, cause}`
  for that kind, whether the provider reported the trace as
  `solver-inconclusive` or as `refuted`.
- A `NotReproduced` trace, or a `NotExact` trace with no accepted
  `validated-witness`, from a `solver-inconclusive` cause SHALL leave the
  item `inconclusive`, `SolverInconclusive{trace, replay}`.
- A `HybridReplayRefusal::ResourceExhausted { counter: WorkUnits }` from either replay
  function SHALL settle the item
  `incomplete`, `Incomplete(LimitReached{limit, value, setting})`, whether
  the trace came from a `solver-inconclusive` cause or a provider's `refuted`
  entry. `limit` SHALL be `work_units`, `value` the meter's configured
  `work_units` bound, and `setting` `accounting.work_units`; the caller can
  raise the bound through that
  setting. The item SHALL carry no counterexample and no verdict.
- A `NotExact` trace whose entry, a provider's `refuted` entry or a
  `solver-inconclusive` cause, carries an FSpec FR-652 `validated-witness`
  that FR-751 accepts SHALL settle the item `refuted` (ADR-018 V-4) with
  that trace as its `HybridCounterexample` and one `counterexamples` entry,
  its kind `undefined-evaluation{where, cause}` when the witness's decisive
  box is one over which an expression is definitely without value and
  `formula` otherwise. Its replay settlement is FSpec FR-364's
  `reproduced-with-validated-witness`: exact replay cannot decide the trace,
  and the accepted witness proves in exact interval arithmetic that every
  solution its boxes enclose, the one solution from the trace's start among
  them, violates the claim. A `NotReproduced` trace never settles `refuted`
  through a witness, because exact replay decided that trace and found no
  violation.
- The kind a provider's `refuted` entry claims SHALL agree with the kind
  its accepted witness gives, by the agreement rule above with the
  witness's decisive box in place of the reproduced decisive instant: both
  `Formula`, or both `UndefinedEvaluation` with equal `where.position`,
  equal `where.locus`, equal `cause` and a claimed `where.time` that
  contains the decisive box's instant, or the step's `time` for an a-priori
  box. A claim whose kind does not agree SHALL settle the item
  `inconclusive` with
  `ReplayParity(DisagreementCause::HybridTrace(NotExact{counterexample}))`,
  carrying the provider's complete counterexample, and no `counterexamples`
  entry, because the witness proves a different violation from the one the
  provider claimed.
- A provider's `refuted` entry whose trace gives `NotReproduced`, or gives
  `NotExact` with no `validated-witness` or one FR-751 rejects, SHALL settle
  the item `inconclusive` with
  `ReplayParity(DisagreementCause::HybridTrace(disagreement))` (FSpec FR-364).
  `disagreement` SHALL be `HybridTraceDisagreement::NotReproduced` when exact
  replay returned `NotReproduced`, and
  `HybridTraceDisagreement::NotExact` when exact replay returned `NotExact`.
  Each variant SHALL carry the provider's complete `HybridCounterexample`
  (its kind and trace), and SHALL not be converted to a
  `SeparatingWitnessRecord`, `Verdicts` or `NoValue`. The serialized
  disagreement cause SHALL use `{"cause":"hybrid-trace","disagreement":{"type":"not-reproduced","counterexample":{"type":"hybrid-counterexample","kind":...,"trace":...}}}` or the same object with `type` `not-exact`; `trace` uses FSpec FR-193's hybrid trace wire form. The enclosing `replay-parity` cause SHALL preserve this distinct variant and the complete counterexample. The
  resulting record SHALL have no `counterexamples` entry.
- FR-751 SHALL check a trace's `validated-witness` only after static and
  exact replay, and only when exact replay returns `NotExact`: a trace exact
  replay returns `Reproduced` or `NotReproduced` for settles by that outcome
  alone, and its witness is recorded unchecked (FR-621), never consulted.
- A provider's `refuted` entry settles `refuted` through a witness only
  after FR-751 accepts that witness and the claimed kind agrees with the
  witness's decisive reading; the upgrade applies to the `HybridVerdict::Refuted`
  path exactly as to a `solver-inconclusive` trace. With no witness, a
  rejected one or a kind that does not agree, it settles
  `ReplayParity(DisagreementCause::HybridTrace(NotExact{counterexample}))`.
- A witness check the work meter stops (`Rejected { rule: work-limit }`)
  SHALL settle the item `incomplete`,
  `Incomplete(LimitReached{limit, value, setting})` with `limit`
  `work_units`, `value` the meter's bound and `setting`
  `accounting.work_units`, as for a replay the meter stops, with the
  witness recorded `certificate-rejected{work-limit, at}`; the item is never
  `refuted` and never `replay-parity`, since the check decided nothing.
- No other path SHALL settle a hybrid item `refuted`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-450-AC-1 | Over FR-447's `Braking`, a `solver-inconclusive` trace whose `start` box `v` in `[25, 26]` meets no initial entry replays `Inconsistent { at: /start, check: Initial }`; a trace whose `before` box `v` in `[1/200, 1/100]` misses the guard `v = 0` by less than its `delta` 1/100 replays `Consistent`, and the same trace with no `delta` `Inconsistent` at check `Jump`; a trace whose second state's `time.upper` is below the first's `time.lower` is `Inconsistent` at check `Time`; each item settles `inconclusive`, `SolverInconclusive{trace: Some(..), replay: Some(..)}` with that outcome and no `counterexamples` entry; a `delta` of `-1/100` is refused `invalid_result`/`invalid-value` at `/delta`. | Test (TC-1221) |
| FR-450-AC-2 | Against `always [0, 12] holds(h.v > 0)`, the point trace from `(Moving, v = 20, a = 2, t = 0)` with `before` `(time 10, v = 0, a = 2, t = 10)` and transition 0 to `(Stopped, time 10, v = 0, a = 2, t = 10)` replays `Reproduced` with kind `Formula`, from a `solver-inconclusive` cause and from a provider's `refuted` entry alike, and the item settles `refuted` with that trace as its `Counterexample` and one `counterexamples` entry; against `SlowEnough` the same trace gives `NotReproduced` at check `Claim` and the item stays `inconclusive`. | Test (TC-1221) |
| FR-450-AC-3 | The AC-2 trace with `before` `v = 1` gives `NotReproduced` at check `Dwell`; as a provider's `refuted` entry it settles `inconclusive`, `ReplayParity(DisagreementCause::HybridTrace(NotReproduced{counterexample}))`, preserving the complete input counterexample, and has no `counterexamples` entry. The same input with `before.time` 9 gives `NotReproduced` at check `Dwell` and the same distinct `NotReproduced` cause with its complete counterexample. | Test (TC-1221) |
| FR-450-AC-9 | On FR-447's `Braking` with `Moving`'s flow changed to `v' = -v`, a provider's `refuted` entry carrying the otherwise valid AC-2 point trace returns `NotExact`; the item settles `inconclusive`, `ReplayParity(DisagreementCause::HybridTrace(NotExact{counterexample}))`, retaining the complete counterexample and distinguishing it from `NotReproduced`; no `counterexamples` entry is written. | Test (TC-1221) |
| FR-450-AC-4 | With `Moving`'s flow `v' = -v`, the AC-2 trace gives `NotExact` and is replayed statically only, settling `inconclusive`; with `Moving`'s flows `v' = -a` and `a' = -1 / 10` it is replayed exactly, with `a(t) = 2 - t / 10` and `v(t) = 20 - 2t + t^2 / 20`, and gives `NotReproduced` at check `Dwell` because `v(10) = 5`; with a flow `v' = drag(v)` over a recursive `drag`, it gives `NotExact`. | Test (TC-1221) |
| FR-450-AC-5 | The AC-2 trace against `always [0, 12] holds(1 / (h.v - 5) != 0)`, true up to the undefined instant, replays `Reproduced` with kind `UndefinedEvaluation`, naming the instant at time 15/2 in `Moving`, where `v = 5`, and the division's locus, and settles `refuted` with cause `UndefinedEvaluation{where, cause}`; against `always [0, 12] holds(1 / (h.v - 5) < 100)`, false from time 1499/200 before that instant, it replays `Reproduced` with kind `Formula`, the decisive instant 1499/200; against a claim whose atom calls a recursive function it gives `NotExact`; static replay of a guard calling a recursive function whose measure interval is undecided terminates with that call's result range. | Test (TC-1221) |
| FR-450-AC-6 | Over `Bounce`, `hybrid Bounce using p { mode Up invariant { x >= 0 and x <= 1 } flow { x' = 1; } transition to Down when { x = 1 } reset { true }; mode Down invariant { x >= 0 and x <= 1 } flow { x' = -1; } transition to Up when { x = 0 } reset { true }; }`, whose initial states are `Up` with `x` in `[0, 1]` at time 0, against `eventually always holds(h.x < 1)` under the infinite-trace profile, the point trace from `(Up, x = 0)` through `Down` at time 1 back to `(Up, x = 0)` at time 2 with `loop_to` 0 reproduces as a periodic behaviour, also with `fair Up;`; the same trace with `loop_to` 1, whose last state `(Up, x = 0)` is not the `Down` state it names, gives `NotReproduced` at check `Loop`; the same trace whose last state has `x = 1/2` gives `NotReproduced` at check `Jump`, the reset `true` keeping `x = 0`; and a `loop_to` 2, naming the last state, is refused `invalid_result`/`invalid-value` at `/loop`. | Test (TC-1221) |
| FR-450-AC-7 | Over `Bounce` and its `Up`/`Down` point lasso from FR-450-AC-6 with `loop_to` 0, a provider's `refuted` trace against `eventually [999999, 1000000] holds(h.x = 2)` needs at least 500,000 `ClaimSegment`s to evaluate the finite interval because the lasso repeats every 2 seconds and replay reaches the 1,000,000-second upper bound; with `work_units` 100,000, the meter cannot cover all required segment charges and exhausts deterministically before deciding the formula, so replay returns `HybridReplayRefusal::ResourceExhausted{counter: WorkUnits}` and settles `incomplete`, `LimitReached{limit: work_units, value: 100000, setting: accounting.work_units}`, with no verdict or `counterexamples` entry. | Test (TC-1221) |
| FR-450-AC-8 | In a `Bounce` variant whose `Up` transition guard is `descending(x) and x = 1`, define the nonrecursive pure Boolean wrapper `descending(s: Rational[0, 20000]): Boolean pure { descend(20000) }` and the recursive pure Boolean function `descend(n: Int[0, 20000]): Boolean pure decreases(n) { if n = 0 then true else descend(n - 1) }`. S3 admits the guard call to `descending` because its argument and parameter are rational, while the `Int` recursion stays in the wrapper body outside the hybrid-term call boundary; FSpec FR-146 admits `n` as an integer measure. Static replay evaluates 20,000 recursive edges, making 20,001 recursive `descend` calls plus the one `descending` wrapper call, on an explicit stack; with `work_units` 100, the meter exhausts while evaluating the guard and replay returns `HybridReplayRefusal::ResourceExhausted{counter: WorkUnits}`, settling `incomplete`, `LimitReached{limit: work_units, value: 100, setting: accounting.work_units}`, with no verdict or counterexample; with `work_units` `u64::MAX`, it returns `Consistent` without stack overflow. | Test (TC-1221) |
| FR-450-AC-10 | Over FR-447's `Braking` without `t`, against `always [0, 10] holds(h.v > 5)`, the point trace from `(Moving, v = 20, a = 2, time 0)` with no jump and `end` `(time 8, v = 4, a = 2)` replays statically `Consistent` and exactly `Reproduced` with kind `Formula`, the bad prefix ending at `end`; with `end` `v = 5` it gives `NotReproduced` at `/end`; with `end` `v = 25` static replay gives `Inconsistent` at check `End`; a trace carrying `end` and `loop_to` 0 refuses `invalid_result`/`invalid-value` at `/end`. | Test (TC-1221) |
| FR-450-AC-11 | Over FSpec FR-652-AC-3's `Grow`, whose flow `v' = 1 - 1 / (1 + v * v)` is not a polynomial flow so its point trace from `v = 1` gives `NotExact`, a provider's `refuted` entry carrying that trace with FR-652-AC-3's nine-step `validated-witness`, which FR-751 accepts, settles `refuted` with the trace as its `HybridCounterexample` of kind `Formula` and one `counterexamples` entry; the same entry with the witness cut after its step at `[28, 32]`, which FR-751 rejects `witness-violation`, settles `inconclusive`, `ReplayParity(DisagreementCause::HybridTrace(NotExact{counterexample}))` as FR-450-AC-9 states, with no `counterexamples` entry. | Test (TC-1221) |
| FR-450-AC-12 | Over FR-750-AC-12's `Shuttle` and its point lasso `L` (`loop_to` 0, period 1), under `model-time`, exact replay never returns `NotExact` for these `on each` claims and decides: (a) `on each (q: Shuttle) when (q.v > 0) { capture v0: Rational[0, 1; 1, 2] = q.v + 1/2; eventually [0, 1] holds(h.v + 1/2 >= v0) }`, whose capture value `τ + 1/2` is outside its type's value range `[0, 1]` exactly for the activation instants in `(1/2, 1)` of each period and is `1`, in range, at `τ = 1/2`, an open set with no earliest instant, returns `Reproduced` with kind `UndefinedEvaluation`, cause `outside-capture-range`, `where.position` 0 and `where.time` the point interval `[1/2, 1/2]`, the infimum; a provider `refuted` entry of that kind with `where.time` `[1/2, 1/2]` settles `refuted`, and one with `where.time` `[3/4, 3/4]`, which does not hold the reproduced instant, gives `NotReproduced` at check 5; (b) `on each (q: Shuttle) when (q.v > 0) { historically holds(h.v > 0) }` reproduces with kind `Formula`, because `historically` reads from position 0, where `v = 0`, so every obligation is false, while `on each (q: Shuttle) when (q.v > 0) { once holds(h.v = 0) }`, true at every activation for the same reason, gives `NotReproduced` at check 5; (c) `on each (q: Shuttle) when (q.v > 0 and q.v <= 1/2) { not once holds(h.v > 3/4) }`, true at every activation instant of the first period (its history up to `τ <= 1/2` never exceeds `1/2`) and false at every activation instant of the second and later periods (their history holds the first period's `v` in `(3/4, 1)`), reproduces with kind `Formula`: its past reach is one period, so replay decides the first two periods, and a replay that decided only the first period would return `NotReproduced`; (d) over the model `hybrid Phase using p { initial { mode Warm when { x = 0 and w = 1 }; } mode Warm invariant { x >= 0 and x <= 1 } flow { x' = 1; w' = 0; } transition to Cold when { x = 1 } reset { x = 0 and w = 0 }; mode Cold invariant { x >= 0 and x <= 1 } flow { x' = 1; w' = 0; } transition to Cold when { x = 1 } reset { x = 0 and w = 0 }; }` and its point lasso from `(Warm, x = 0, w = 1)` at time 0 by transition 0 (`before` `(time 1, x = 1, w = 1)`) to `(Cold, x = 0, w = 0)` at time 1 and by `Cold`'s transition (`before` `(time 2, x = 1, w = 0)`) to `(Cold, x = 0, w = 0)` at time 2 with `loop_to` 1 (prefix `[0, 1)` on which `w = 1`, period `P = 1` on which `w = 0`), the claim `on each (q: Phase) { once [0, 1] once [0, 1] holds(h.w = 1) }` has past reach `R = 2`: its obligations hold at every activation instant at most 3 and fail at every activation instant after 3, replay decides the prefix and three periods, and the provider `refuted` trace reproduces with kind `Formula` and settles `refuted`; a replay that decided only the prefix and two periods would return `NotReproduced`. | Test (TC-1221) |
| FR-450-AC-13 | Over FSpec FR-652-AC-3's `Grow` and its point trace from `v = 1`: a provider's `refuted` entry of kind `Formula` carrying the nine-step witness settles `refuted` with one `counterexamples` entry and replay settlement `reproduced-with-validated-witness`, reached through `HybridVerdict::Refuted` and not only through `solver-inconclusive`; the same entry claiming kind `UndefinedEvaluation` settles `inconclusive`, `ReplayParity(HybridTrace(NotExact{counterexample}))`, with no `counterexamples` entry; the same `Formula` entry under a `work_units` bound that lets static and exact replay finish but stops the witness check settles `incomplete`, `LimitReached{work_units, value, accounting.work_units}`, with `certificate-rejected{work-limit}` recorded and no counterexample. Over FSpec FR-193-AC-17's `Braking` point trace, which exact replay reproduces, a provider's `refuted` entry carrying a structurally valid witness that FR-751 would reject settles `refuted` by exact replay, its witness recorded unchecked. | Test (TC-1221) |

## Dependencies

- FSpec FR-193 "Hybrid trace" and "Trace replay" (the trace shape and the
  replay rules); FSpec FR-331 (the `HybridCounterexample`); FSpec FR-364
  (`reproduced-with-evaluated-witness` and `replay-parity`).
- FSpec FR-146 (recursive function measures).
- ADR-026 HY-2; ADR-013 O-16.
- FR-255 (accounting limits and the `accounting.work_units` setting).
- [FR-447](FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md)
  (the checked hybrid model and claim),
  [FR-250](FR-250-map-a-hybrid-solver-result-to-a-verdict.md) (the verdict
  map and `HybridTrace`),
  [FR-290](FR-290-settle-plugin-results-as-typed-terminal-records.md) (the plugin reader).
