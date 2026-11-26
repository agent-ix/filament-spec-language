---
id: FR-254
title: "Sample timed runs for statistical checking"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-231
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-234
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-193
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-237
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-253
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-038
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-416
    type: depends_on
---
# FR-254: Sample timed runs for statistical checking

## Description

The statistical engine EN-4 SHALL sample a stochastic timed model by
repeating the race (FR-253) from the initial state, with every delay an
exact rational drawn on a grid of `2^-q` in the uniform value, and read
timed events and measures over the samples (ADR-026 SS-1 to SS-7). A
`measured` verdict is a measurement of the quantised model and records `q`.
A sampled witness replays through `replay_sampled_witness` (FR-193), which applies FR-237's timed steps and recomputes each draw. Exact probabilistic
checking over a timed model routes to EN-5.

## Use case

A verification operator asks whether a late reply has probability at most
1%, the 99th percentile of reply latency, and the fraction of time a
server is available. Each estimate comes with its confidence and the grid
`q`, and any sample can be replayed exactly.

## Inputs

- A probabilistic claim over a stochastic timed subject, its workload, a
  seed, the method settings of the statistical design (its ST-8), and
  `TimedSampling { q: u32, max_sample_steps: u64 }`: `q`, the grid
  exponent, default 64, a caller-set method parameter; `max_sample_steps`,
  the steps one sample may take before passing its horizon, default
  16_777_216 (2^24), an ADR-014 B-5 budget.

## Outputs

- The statistical design's `measured` verdict, with `q` in its basis, or
  its `undecided`, `incomplete` and `unsupported` dispositions; sampled witnesses
  carrying their delays.

## Behavior

- **Events and measures.** Under the timed profile, an event SHALL be a
  `BoundedWindow` formula on origin, decided by the positions with time
  stamp up to its horizon. A measure `within h` under `model-time` SHALL
  take `h` as a time quantity. `elapsed from holds(A) until holds(B) within
  h` SHALL be the time-stamp difference of the two positions. `weighted by
  delay` SHALL weight each position by the delay of the step that leaves
  it.
- **Time windows.** For a time horizon `h`, EN-4 SHALL read the window FSpec
  FR-407 "Time windows" states: every position from the activating one whose
  stamp is at most `h` after it, inclusive, each step's post-state its own
  position, several at one stamp included; a delay and the instant `h` add
  none. This holds for a formula, a `fraction`, `steps` and `accumulate`: an
  unweighted fraction is the positions where `P` holds over the positions in the window,
  never over `h + 1`, and the positions of `accumulate` and `steps` are
  positions of the window. No form with a time horizon SHALL be read with a
  step horizon.
- **Elapsed clock.** For `elapsed from holds(A) until holds(B) within h`,
  EN-4 SHALL start the elapsed time at the first position at or after the
  activating one where `A` holds, with no limit on reaching it, and censor as
  FSpec FR-407 "Elapsed" states (FSpec FR-753).
- **Step rewards.** Each sampled timed step SHALL carry its rewards. Under
  `weighted by R`, EN-4 SHALL draw the step that leaves a window position even
  when it lands after the window's end, weigh the position by its reward and
  add no position for it.
- **Tick subjects.** Under a `time tick` source EN-4 SHALL run no race and
  dispose no identity as free: it takes the statistical design's untimed step
  (FSpec FR-420 "Tick subjects") and stamps each position with the period
  times the tick-operation steps up to it.
- **Idle tail under a tick source.** Where a `time tick` sample reaches an
  idle tail before its window ends, EN-4 SHALL read the terminal stutter
  positions stamped within the window.
- **Idle tail under dense time.** Where a `time dense` sample reaches an idle
  tail before its window ends, EN-4 SHALL decide the sample's reading over
  every completion of FSpec FR-420 "Windows on an idle tail", by the tests
  stated there, never by trying chosen values of `m`.
- Where all completions give one reading, EN-4 SHALL settle the sample with
  that reading.
- If two completions give different readings, then EN-4 SHALL stop the run at
  the first such sample in trace-index order and settle the item
  `Unsupported(NotStochasticIdleTail{state})`, `state` the idle tail's first
  timed state (FR-253; wire `not-stochastic` with `state` and `idle_tail` true, no
  `identity`, FSpec FR-331).
- **Undecided completions.** Where EN-4 cannot yet decide a timed formula,
  or a form whose predicate or `when` guard reads a clock, over every
  completion of a dense idle tail, EN-4 MAY stop the run at that sample and
  settle the item `Incomplete(IdleTailUndecided{state, reason})` (FR-253;
  FSpec FR-420 "Undecided completions").
- That stop SHALL give no reading, verdict or `unsupported`, and satisfies no
  criterion that states one; the acceptance criteria below are unchanged by
  it.
- If deciding the completions reaches a request limit, then EN-4 SHALL settle
  the item `Incomplete(LimitReached{limit, value, setting})` naming it.
- A release whose EN-4 takes the `IdleTailUndecided` stop SHALL name it in its
  release notes (QSL-3447).
- **Given delays only.** Over a `time dense` subject, if an identity with a
  free delay (FR-253) races at a sampled timed state, then EN-4 SHALL settle
  the item `unsupported`, `NotStochastic{state, identity}`, at the first such
  state.
- **Draws.** A `discrete` draw SHALL select by exact integer weights. A
  `uniform`, `uniform[a, b]` or `exponential` draw SHALL take a uniform value `u` on a grid
  of `2^-q` from the generator and return the conditioned inverse
  distribution function at `u`, computed by the statistical design's
  rational method and rounded up, an exact rational. Each delay draw and
  each tie-break SHALL be one more choice with its own choice index.
- A sample SHALL end when its next position's time stamp passes the end of
  its window, or at an idle tail. The window ends its time horizon after the
  activating position's stamp, and for `elapsed` after the stamp of the first
  position where `A` holds.
- If a sample reaches a local time-lock before its horizon, where no
  positive delay is admissible and no transition identity is enabled, then
  the sample ends there, and EN-4 SHALL read it as FSpec FR-416 "How claims
  read a time-lock" states: an event not yet met is not met, an `always` that
  the positions read satisfy holds, and an uncompleted measure is censored.
  Only a fraction or long-run claim, whose reading of such a run FSpec FR-420
  leaves undecided, SHALL settle `unsupported`, `TimeLockedSample{state}`,
  naming the state, with no witness kept. The time-lock-freedom item reports
  the time-lock.
- **Budgets.** If a sample takes more than `max_sample_steps` steps without
  passing its horizon, then EN-4 SHALL stop the run as the statistical
  design's budgets state, naming `max_sample_steps`, its value and the
  request member that raises it; every budget and `q` SHALL be a caller-set value with a
  published default.
- **Regeneration.** The regenerative method SHALL cut a run at each return
  to the initial discrete state with every clock 0; a model that never
  returns SHALL stop when a cycle reaches `max_cycle_steps` and settle
  `incomplete`, `limit-reached` naming `statistical.max_cycle_steps`, as
  ADR-024 ST-7 does.
- **Witnesses.** A sampled witness SHALL carry each step's delay and replay
  through `replay_sampled_witness` (FR-193) over the checked subject the sampler
  ran over, with no recompile since S4 does not emit a timed package that has a
  workload, applying FR-237's timed steps (delay admissibility, enabledness,
  successor selection, resets and time invariants), plus recomputation of each draw from the seed, trace index,
  step, choice index and `q`.
- **Exact route.** The request writer SHALL route a request for `exact`
  evidence over a timed subject to EN-5, whose route (ADR-038,
  ADR-028 TA-1, TA-2) decides which constraints and delay distributions it admits.
- EN-4 SHALL compute the verdict as a function of the claim, the subject,
  the workload, the seed and the settings.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-254-AC-1 | `RareLate` (`probability <= 0.01 [eventually[0 ms, 3 ms] holds(c.late)]`) over FR-253-AC-1's model, whose exact event probability is `1/20`, settles `measured`, `Rejected`, with `q` 64 in its basis; with `reply` under `uniform` it settles `measured`, `Accepted`. Two runs with one seed give equal verdicts. | Test (TC-709) |
| FR-254-AC-2 | Every sampled delay is an exact rational; a sampled witness replays through `replay_sampled_witness` (FR-193) with FR-237's timed steps, and its draws recompute from the seed; the same witness with one delay changed fails draw recomputation. With `q` 8, the basis records 8 and each sampled `uniform` or `exponential` delay equals the rounded-up conditioned inverse distribution function at a multiple of `2^-8`. | Test (TC-709) |
| FR-254-AC-3 | `elapsed from holds(c.phase = Waiting) until holds(c.phase != Waiting) within 3 ms` reads each sample's time-stamp difference; a model that never returns to its initial discrete state with every clock 0 settles `incomplete`, `limit-reached` naming `statistical.max_cycle_steps` under the regenerative method; an `exact` request over the strict-guard `Rpc` variant is decided over clock regions (FR-204), never admitted as untimed. `RareLate` with no `delay` declaration on `timeout` settles `unsupported`, `NotStochastic`, naming the state just after `send` and `timeout`. `RareLate` with `max_sample_steps` 1 stops `Incomplete(LimitReached{limit, value, setting})` naming `max_sample_steps`, 1 and `statistical.max_sample_steps` (FR-255's engine table). | Test (TC-709) |
| FR-254-AC-4 | A stochastic `Lock` variant (one object `c` with `phase: {A, B}` and a clock `x` never reset; `go` moves `A` to `B`, with `delay Lock::go ~ discrete { 1/2 ms: 1 };`; `time invariant when phase = B { x <= 1 ms }`; no operation enabled in `B`) samples `go` at `1/2` and delays to `(B, x = 1)`, where no positive delay is admissible and no identity is enabled, and each sample ends there: `probability <= 0.5 [eventually[0 ms, 2 ms] holds(c.phase = B)]` has estimate 1, the position stamped `1/2` ms being in `B`, and settles `Rejected`; `mean of (fraction holds(c.phase = B) over [0 ms, 2 ms]) <= 0.5` settles `unsupported`, `TimeLockedSample` naming that state, with no witness kept. | Test (TC-709) |
| FR-254-AC-5 | On TC-709's `Send` fixture (`send` at 2 ms, `reply` 10 ms later), `mean of (fraction holds(c.phase = 1) over [0, 5 ms]) <= 0.25` reads the positions stamped 0 and 2 ms and settles `Rejected` with estimate `1/2`; with `reward cost` 1 on `send` and 3 on `reply`, the `weighted by cost` form settles `Rejected` with estimate `3/4`, reading the reward of the `reply` step that lands at 12 ms. | Test (TC-709) |
| FR-254-AC-6 | On TC-709's `Tick` fixture (`time tick`, period 2 ms), `mean of (fraction holds(c.phase = 1) over [0, 5 ms]) <= 0.25` reads the positions stamped 0, 0, 2 and 4 ms and settles `Rejected` with estimate `3/4`; `probability >= 0.5 [eventually[3 ms, 5 ms] holds(c.phase = 1)]` settles `Accepted` with estimate 1, and with `[3 ms, 3 ms]` `Rejected` with estimate 0. No item is `NotStochastic`. | Test (TC-709) |
| FR-254-AC-7 | On TC-709's `Idle` fixture (`send` at 2 ms, then an idle tail), each is decided, as every completion gives one reading: `probability >= 0.5 [fraction holds(c.phase = 1) over [0, 5 ms] >= rational(2, 5)]` estimate 1, `Accepted`; the same with `<= rational(2, 5)` estimate 0, `Rejected`; `mean of (fraction holds(c.phase = 0 or c.phase = 1) over [0, 5 ms]) <= 0.25` estimate 1, `Rejected`; `mean of (fraction holds(c.phase = 2) over [0, 5 ms]) <= 0.25` estimate 0, `Accepted`; with `reward cost` 1 on `send`, `mean of (fraction holds(c.phase = 1) over [0, 5 ms] weighted by cost) <= 0.25` estimate 0, `Accepted`; `probability >= 0.5 [steps from holds(c.phase = 0) until holds(c.phase = 1) within 5 ms >= 2]` estimate 0, `Rejected`; `probability >= 0.5 [steps from holds(c.phase = 0) until holds(c.phase = 2) within 5 ms <= 3]` censored, estimate 0, `Rejected`; `probability >= 0.5 [eventually[3 ms, 5 ms] holds(c.phase = 0)]` estimate 0, `Rejected`; `probability >= 0.5 [always[3 ms, 5 ms] holds(c.phase = 1)]` estimate 1, `Accepted`. | Test (TC-709) |
| FR-254-AC-8 | On the `Idle` fixture, each of these settles `Unsupported(NotStochasticIdleTail{state})`, `state` the timed state just after `send` (wire `not-stochastic` with `idle_tail` true and no `identity`): `mean of (fraction holds(c.phase = 1) over [0, 5 ms]) <= 0.25` (values `(1 + m)/(2 + m)`); `probability >= 0.5 [fraction holds(c.phase = 1) over [0, 5 ms] <= rational(3, 5)]` (true at `m = 0`, false for large `m`); `probability >= 0.5 [eventually[3 ms, 5 ms] holds(c.phase = 1)]` (false with no stutter position, true with one at 4 ms). With `send` at 5 ms the first claim settles `Rejected` with estimate `1/2`. | Test (TC-709) |
| FR-254-AC-9 | `elapsed` starts at the first `A`. On TC-709's `Send` fixture (`phase` 1 first at 2 ms, 2 first at 12 ms), `probability >= 0.5 [elapsed from holds(c.phase = 1) until holds(c.phase = 2) within 10 ms <= 10 ms]` settles `Accepted` with estimate 1 (elapsed `12 - 2 = 10`), and with `within 9 ms <= 9 ms` it is censored, estimate 0, `Rejected`. On the `Send` variant whose `reply` is `discrete { 1 ms: 1 }`, `probability >= 0.5 [elapsed from holds(c.phase = 1) until holds(c.phase = 2) within 1 ms <= 1 ms]` settles `Accepted` with estimate 1 (elapsed `3 - 2 = 1`), though `A` first holds later than `h`. | Test (TC-709) |

## Dependencies

- ADR-026 §10 SS-1 to SS-7.
- [FR-231](FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md),
  [FR-234](FR-234-check-timed-intervals-and-classify-timed-property-forms.md),
  [FR-193](FR-193-keep-and-replay-sampled-witnesses.md),
  [FR-237](FR-237-replay-a-timed-counterexample.md),
  [FR-253](FR-253-check-delay-distributions-and-define-the-race.md).
- The statistical design (ADR-024 PF-1 to PF-6, ST-1 to ST-9, SV-1 to
  SV-8) and the exact probabilistic engine (ADR-028), which this extends and
  routes to.

## References

- FSpec half: FSpec FR-420 (Linear STD-139) owns timed events and
  measures, free delays, the sampler's delay draws on the `2^-q` grid, tick
  subjects and windows on an idle tail
  (ADR-026 OV-8); FSpec FR-407 owns time windows; FSpec FR-408 (Linear STD-137) owns the
  statistical methods.
