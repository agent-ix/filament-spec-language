---
id: FR-204
title: "Check probabilistic timed automata through digital clocks"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-197
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-198
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-195
    type: depends_on
---
# FR-204: Check probabilistic timed automata through digital clocks

## Description

EN-5 SHALL decide a probabilistic claim with exact evidence over a timed
subject whose clock constraints are all closed by exploring its digital
MDP (ADR-028 TA-1 to TA-7): integer clocks after scaling every constant,
unit delay moves, and discrete moves as the timed step with integer clocks.
The claim's scheduler SHALL resolve each delay (RU-6): over every scheduler
the scheduler chooses delays and identities; under a workload, the workload
resolves the identity and every delay a `discrete` delay distribution
gives, so the subject is a DTMC over digital timed states; a delay no
distribution gives stays the scheduler's choice and is decided by its
minimum and maximum. Strict constraints, other bounded-window form shapes,
delay distributions over every scheduler, zero-delay cycles and time-locks
are decided exactly by the rules under Admission; only continuous delay
distributions, whose probabilities exact evidence cannot state, go to the
statistical engine.

## Use case

A protocol designer checks that a retransmitting sender delivers within
4 ms with probability at least 0.99 against any choice of send times inside
the timing constraints. EN-5 explores the digital MDP, finds the minimum
exactly `99/100`, and proves the claim at equality. With the send delay
declared as `discrete {1 ms: 1, 2 ms: 1}` under a workload, it computes the
exact probability of the resulting chain instead.

## Inputs

- FR-196's request over a timed subject (ADR-026 TS-1), with its clocks,
  guards, time invariants, urgency, and any `delay` declarations.

## Outputs

FR-196's `ProbProduct` over digital timed states, whose `ProductKey` adds an
integer value per clock and, for a deadline event, the deadline clock, plus
FR-196's complete tail control. Each edge has the mandatory closed
ProductAction choice. Delay/tail observation operation projections remain
`action: None`; this projection cannot identify the policy choice. Delay
choices carry one digital unit of elapsed time; IdleObserve carries zero. The results of
FR-197 and FR-198 over it, or over the region MDP for strict constraints;
and, for exact evidence over a continuous delay distribution only, FR-203's
unsupported cause `DelayDistribution{operation}`.

## Behavior

### Admission

- Every claim over a timed subject whose delays are deterministic or given by
  `discrete` distributions SHALL be decided with exact evidence; no form,
  constraint or scheduler of that class settles `unsupported`. The digital MDP
  below decides it when every clock constraint is closed; the following rules
  decide the rest.
- **Strict constraints.** When a guard, a time invariant or a timed interval
  of the claim is strict, EN-5 SHALL decide the claim over the region MDP
  instead of the digital MDP: each clock keeps its integer part, capped as in
  the digital MDP, together with whether its fractional part is zero and the
  order of the clocks' fractional parts. Region MDPs give exact minimum and
  maximum probabilities and expected times for these forms, which digital
  clocks do not once a constraint is strict, so the strict case keeps an
  exact verdict. Its certificate keys each value by region in place of
  digital clock values.
- **Form shapes.** Every bounded-window formula FR-186 admits under the timed
  profile SHALL be decided. `always[0, D] holds(P)` SHALL be decided as the
  complement of `eventually[0, D] holds(not P)`, a minimum becoming a
  maximum and the reverse. Every other bounded-window formula SHALL be
  decided over the product of the MDP with the claim's monitor, which keeps
  one deadline clock per interval operator occurrence, capped at its upper
  bound plus one.
- **Delay distributions over every scheduler.** Over every scheduler, a
  `discrete` delay distribution SHALL apply as it does under a workload: the
  race draws given delays with their probabilities, and the scheduler chooses
  identities and every free delay, so the subject is an MDP decided by its
  minimum and maximum.
- **Continuous delays.** A `uniform`, `uniform[a, b]` or `exponential` delay
  makes the subject a continuous-time stochastic process whose probabilities
  are in general not rational, so exact evidence cannot state them. Such an
  item SHALL be negotiated to the statistical engine (FR-189), which samples
  the continuous delays and settles `measured`. An item that requests exact
  evidence for it SHALL settle `unsupported` with
  `DelayDistribution{operation}` naming the first such operation and the
  locus of its `delay` declaration.
- **Zero-delay cycles.** A behaviour of a timed subject lets time diverge
  (FSpec FR-416), so no scheduler may stay forever in a cycle of zero-delay
  moves. EN-5 SHALL decide over the MDP with every end component that holds
  no delay move made transient: its states take the value of their best exit
  for the objective, and a scheduler's choice inside it is read as a choice
  of that exit. An end component with no exit is a time-lock, decided as
  below.
- **Time-locks.** A digital or region state from which time cannot advance
  and no discrete move is enabled (under a workload, a state where no
  identity races and delay is bounded) SHALL be terminal: its run ends there
  with FR-196's terminal stutter, time frozen, so a deadline not yet met is
  never met. The verdict is exact.

### The digital MDP

- EN-5 SHALL scale every constant to an integer by the least common
  denominator of the model's and the claim's constants, and give each clock
  integer values capped at its largest compared constant plus one.
- A delay move SHALL add one unit to every clock below its cap; it SHALL be
  admissible when the state is not urgent and every applicable time
  invariant holds after it. A discrete move SHALL be ADR-026 TS-2's step with
  integer clocks.
- Over every scheduler, each digital timed state's actions SHALL be its
  admissible UnitDelay and enabled scheduled identities in running mode.
  A TS-5 quiescent running state additionally admits EnterIdleDelay: a
  positive unit delay committing irreversibly to idle-delayed. In idle-ready
  or idle-delayed, IdleDelay applies the actual delay guards/invariants,
  advances one unit and yields idle-delayed. IdleObserve is enabled only in
  idle-delayed, advances no clock/time, emits the current clock-valued
  stutter letter and yields idle-ready. Idle mode has no operation/reset
  action. Distinct literal action bodies and exact source modes are FR-196;
  no fictitious ScheduledIdentity names these choices.
- Under a workload, each racing identity SHALL draw its `discrete` delay
  conditioned on its window. The smallest draw wins; when several identities
  of one operation tie, SD-3 repeatedly selects a still-enabled identity in
  operation-weight order without replacement, using the remaining identity
  shares defined by ADR-024 PM-2. Each race outcome SHALL be one product edge:
  it advances time once by the winning delay, applies the selected identities
  in order to the evolving model state at that same instant, and ends at the
  product key after the last still-enabled identity is applied. Its edge label
  retains the complete ordered sequence of identities that fired, its
  probability is the product of the conditioned draw mass and each ordered
  selection probability, and its rewards are the ordered firings' rewards.
  A rejection of a certificate for this outcome is located at the source
  product key and this complete edge label (not an intermediate same-instant
  state). Replay expands the label to one trace position for each fired
  identity, all at the same elapsed time, with each position's actual
  intermediate model state. A delay no distribution gives SHALL be an action of
  the scheduler. A state where no identity races and delay is bounded SHALL
  is a time-lock, decided by Admission's time-lock rule.
- Under a workload, a state whose racing identities all have given delays
  (`discrete` distributions) SHALL resolve as above, each race edge carrying
  `Identity(r)` with `r` the first racing identity in canonical order, and a
  `fired` sequence containing the actual ordered identities applied at the
  instant. When the winning identity disables every other tied identity, that
  sequence has one element.
- Under a workload, a state whose racing identities all have free delays
  SHALL offer `UnitDelay` and a single `WorkloadResolve{identity, delay: 0}`
  whose `identity` is the first enabled identity and whose probability mass
  splits over every enabled identity in proportion to the workload's weights.
- Under a workload, a state in which given-delay identities `G` and free-delay
  identities `F` race together SHALL NOT be refused. The given vectors `g` are
  the product of each given identity's `discrete` support conditioned on its
  window, in digital units, each with the product of the conditioned
  probabilities `P(g)`. The state SHALL offer the scheduler's choice of every
  free delay after the given delays are drawn (ADR-026 SD-3) as the MDP action
  `FreeDelays` (FR-196): one action per function `f` that maps each given
  vector `g` to an assignment `f(g)` of one delay in its window, in digital
  units, to every identity in `F`. One row of the action is one `g` with its
  `f(g)`; a row names every free racing identity, not one.
- The race of a row SHALL be ADR-026 SD-3's race over the delays `g` and
  `f(g)` together. The least delay `m` wins; the tied identities fire in the
  PM-2 ordered-without-replacement sequence, applying each still-enabled
  identity to the evolving state at the same instant. The action is one
  distribution of total probability 1, the sum over rows of `P(g)` times the
  row's ordered race outcome. Each outcome is one product edge labeled by its
  complete ordered firing sequence and ends at the product key after all
  still-enabled tied identities have fired.
- `WorkloadResolve{identity, delay}` names one free identity and is made
  before the draw. In a mixed state with one free identity it is exactly the
  `FreeDelays` action whose rows all assign that delay to that identity; with
  two or more free identities it names no SD-3 choice, because the race needs
  a delay for every free identity at once. A row is not a sequence of
  `WorkloadResolve` choices: the race is one step that delays once. The mixed
  state SHALL therefore offer `FreeDelays` only; `WorkloadResolve` remains the
  choice of a state whose racing identities all have free delays, where no
  given draw exists and before and after the draw coincide.
- For every objective EN-5 decides (finite-horizon backward induction, `Reach`,
  expected rewards), the one-step operator at a FreeDelays state is linear in
  the successor distribution, so for a successor value vector `V` and
  `Q(g, a) = Σ_o race(g, a)(o) · V(o)`, optimizing over functions factors by
  row: `opt_f Σ_g P(g) Q(g, f(g)) = Σ_g P(g) opt_a Q(g, a)`, because each
  term depends on its own row only and every `P(g) > 0`. This is the value of
  ADR-028 SCH-2's intermediate-state form, one decision state per drawn `g`.
  The action form therefore loses nothing, adds no ProductKey member, and is
  the representation; a randomized choice over functions induces the same
  per-row distributions as a randomized choice per row.
- The set of all functions is the semantic choice set; the product builds row
  choices only (FR-196), and EN-5 SHALL evaluate a FreeDelays state through
  the row-factored operator `Σ_g P(g) opt_a Q(g, a)`. Graph precomputations
  (Prob0, Prob1, end components) read an action's support as the union of its
  row choices' supports, so a state has an action that stays inside a set
  exactly when every row has a row choice that stays inside it.
- The witness entry at a FreeDelays state SHALL be the action FR-198's witness
  policy rules choose: for a maximum the MEC exit rule first, otherwise the
  attaining action with ties broken by FR-196's canonical transition order.
  Because an action attains exactly when every row attains its row optimum,
  and that order is numeric row by row, the least attaining action is the
  least attaining assignment of each row. Where a row's least attaining
  assignment stays inside an end component and another attaining assignment
  leaves it, the MEC exit rule takes the leaving one: with a free `B` that
  resets the clock and returns to `Idle` at delay 1 and races a given `A` at 2
  at delay 2, both attain the maximum's upper iterate, and the witness takes
  delay 2, the exit, so replay reaches the target.
- An every-scheduler run SHALL carry `Identity` where a workload run carries
  `WorkloadResolve` or `FreeDelays`, because there the scheduler chooses the
  identity itself.

### Evidence

- A witness scheduler SHALL name FR-202's closed ProductAction choices
  for every digital evidence/continuation state: actual enabled identity or
  residual PostState, WorkloadResolve, FreeDelays, UnitDelay, EnterIdleDelay, IdleDelay or IdleObserve
  according to the source mode. Under a workload it resolves only actual
  free-delay/observation choices; fixed race weights/distributions remain
  unchanged. A timed path names the same choice, exact elapsed amount and
  complete post-key; a race edge of a mixed given and free state names
  its `FreeDelays` action, and the witness scheduler's entry at that state is
  that action, one row per given vector. Neither action None nor a local node/index chooses an
  observation. Randomized exact-rational entries are allowed for derived
  TS-5 even without an authored fairness declaration.
- A timed witness path SHALL carry ADR-026 CT-1 delays and replay by CT-3
  through FR-202; replay SHALL check that its last timed state has no
  admissible discrete step before the event's time horizon passes.
- A certificate SHALL be FR-920's `timed-product/v1` document, keyed by
  digital, region or boundary-region product states, and FR-921's checker
  SHALL rebuild the digital or region MDP itself (FR-922 to FR-924, FR-877). Optimization and evidence SHALL
  apply ADR-028 §13a/FR-198 infimum/supremum rules over almost-surely
  TS-5-admissible schedulers, including positive finite-wait components
  with admissible exits. Original continuation graphs, observation edges
  and actual authored fairness are checked separately from objective
  absorption. No unqualified deterministic-memoryless attainment, finite
  waiting bound or new fairness premise decides elapsed expectations.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-204-AC-1 | §15.5 `Deadline` over every scheduler: `x` takes the values 0 to 2 and the deadline clock 0 to 5; the minimum is `99/100` (send at `x = 2`) and the maximum `9999/10000` (send at `x = 1`); the item settles `proved`, `ExactValue{99/100}`. At threshold 0.995 it settles `refuted` with a path of two losses at 2 ms and 4 ms of probability `1/100`, whose replay checks that the next `send` needs `x >= 1`, after 4 ms. | Test (TC-639) |
| FR-204-AC-2 | §15.5 `MeanTime` settles `proved`, `ExactValue{20/9 ms}` (maximum; the minimum is `10/9 ms`). The no-operation x>=1/cap2 oracle of FR-196 has elapsed infimum1 and supremum+infinity despite eventual observation almost surely; at threshold3 its geometric IdleDelay3/4/IdleObserve1/4 policy has exact value5 and a checked infinite observation continuation. A finite maximum3 or delay-only witness fails. With the guard `self.x > 1 ms` the items are decided over the region MDP, never `unsupported`: the maximum is `20/9 ms`, attained by taking every delay at 2 ms, and the minimum is the infimum `10/9 ms`, approached by delays just above 1 ms and not attained (FR-204-AC-13). | Test (TC-639) |
| FR-204-AC-3 | `Retx` with `delay Msg::send ~ discrete { 1 ms: 1, 2 ms: 1 };`, under a workload `One` with weight 1 on `send`, is a DTMC over digital timed states; `probability >= 0.99 [eventually[0 ms, 4 ms] holds(m.delivered)]` settles `proved`, `ExactValue{159129/160000}`: after every reset `send` waits 1 or 2 ms with probability 1/2 each, both inside its window `[1, 2]`, loses with `1/10`, and attempts at time stamps up to 4 ms inclusive count. The same claim `under every scheduler` is decided with the distribution applied (FR-204-AC-15); with `delay Msg::send ~ exponential(1 per ms);` under `One` and exact evidence requested it settles `unsupported`, `DelayDistribution`, naming `send`. | Test (TC-639) |
| FR-204-AC-4 | A variant of `Retx` whose `send` has no `x >= 1 ms` guard and no reset has a zero-delay cycle at its initial state; it is decided with that cycle's end component made transient: once `x` reaches 2 ms time cannot pass before delivery, every time-divergent scheduler delivers with probability 1, so `Deadline`'s minimum and maximum are both `1` and it settles `proved`, never `unsupported` (FR-204-AC-16). `probability >= 0.99 [always[0 ms, 4 ms] holds(not m.delivered)]` over `Retx` is decided as the complement of `eventually[0 ms, 4 ms] holds(m.delivered)`: its minimum is `1 - 9999/10000 = 1/10000` and its maximum `1 - 99/100 = 1/100`, so it settles `refuted`, its minimum being below 0.99 (FR-204-AC-14). | Test (TC-639) |
| FR-204-AC-5 | §15.5 `Retx`, whose `send` has no `delay` declaration, under the workload `One` (weight 1 on `send`): the workload resolves the scheduled identity and `send`'s delay stays the scheduler's choice in `[1, 2]` ms after each reset, with no distribution assumed for it. `probability >= 0.99 [eventually[0 ms, 4 ms] holds(m.delivered)]` under `One` settles `proved`, `ExactValue{99/100}`, its entry marked a minimum. `probability <= 0.999` over the same event under `One` settles `refuted` on the maximum `9999/10000`, its entry marked a maximum and its witness scheduler choosing `UnitDelay` at `x = 0` and `WorkloadResolve{send, 0}` at `x = 1`. At the same quiescent D=1 idle-delayed key, IdleObserve reads success at D with elapsed0 while IdleDelay crosses D with elapsed1; replay/certificates preserve the distinct actions, post-keys and predicate positions. | Test (TC-642) |
| FR-204-AC-6 | The model `Race` (`time dense unit ms`) has `phase` (`Idle` or `Done`) and `b`; the operation `A` is guarded by `phase = Idle`, sets `Done` and declares `delay Race::A ~ discrete { 2 ms: 1 }`; the operation `B` has no `delay` declaration, is guarded by `phase = Idle and x >= 1 ms` under the time invariant `x <= 3 ms` while `Idle`, sets `Done` and `b`; the workload `Both` weights `A` and `B` 1. Under `Both`, `probability >= 0.5 [eventually[0 ms, 5 ms] holds(r.b)]` settles `refuted` on the minimum `ExactValue{0}`, its witness scheduler's `FreeDelays` entry holding the one row given `[2]` to `B` delay 3 (`A` wins at 2 ms, mass 1); `probability <= 0.5` settles `refuted` on the maximum `ExactValue{1}`, its witness entry holding the row given `[2]` to `B` delay 1 (`B` wins at 1 ms, mass 1). The row given `[2]` to `B` delay 2 is the tie at `m = 2`: `B` and `A` each win with probability `1/2`. The state offers the three `FreeDelays` actions and no `WorkloadResolve`. | Test (TC-642) |
| FR-204-AC-7 | With `B`'s guard changed to `phase = Idle and x >= 2 ms`, the choices of `B` are the delays 2 and 3, so the maximum is `1/2` (the tie) and the minimum `0`: `probability <= 0.5 [eventually[0 ms, 5 ms] holds(r.b)]` under `Both` settles `proved`, `ExactValue{1/2}`, its entry marked a maximum, and `probability <= 0.4` settles `refuted` on that maximum with the `FreeDelays` row given `[2]` to `B` delay 2 as its witness choice. The same claim `under every scheduler` is decided with `A`'s distribution applied, not refused. | Test (TC-642) |
| FR-204-AC-8 | In `Race` with `A` declaring `delay Race::A ~ discrete { 1 ms: 1, 3 ms: 1 }`, `B` guarded by `phase = Idle and x >= 1 ms` under the invariant `x <= 3 ms` while `Idle`, and `Both` as before: `probability <= 0.7 [eventually[0 ms, 5 ms] holds(r.b)]` settles `refuted` on the maximum `ExactValue{3/4}` (with the draw `1`, `B` at delay 1 ties for `1/2`; with the draw `3`, `B` at delay 1 or 2 wins), and `probability >= 0.3` settles `refuted` on the minimum `ExactValue{1/4}` (with the draw `1`, `B` at delay 2 or 3 loses; with the draw `3`, `B` at delay 3 ties for `1/2`). | Test (TC-642) |
| FR-204-AC-9 | Over AC-8's `Race` the initial state offers nine `FreeDelays` actions (two rows, given `[1]` and `[3]`, each assigning `B` one of 1, 2, 3) and no `WorkloadResolve`. The witness scheduler for the maximum `3/4` holds the `FreeDelays` entry with the rows given `[1]` to `B` delay 1 and given `[3]` to `B` delay 1, and for the minimum `1/4` the rows given `[1]` to `B` delay 2 and given `[3]` to `B` delay 3 (least attaining delay per row). A single race whose payoff falls as `B`'s delay rises gives equal optima before and after the draw, so AC-8 cannot separate them; AC-10 does. | Test (TC-642) |
| FR-204-AC-10 | Separating vector. The model `Detour` (`time dense unit ms`) has `phase` (`Idle`, `Half` or `Done`) and `b`. `A`, guarded by `phase = Idle`, sets `Done` and `b` and declares `delay Detour::A ~ discrete { 1 ms: 1, 3 ms: 1 }`; `B` has no `delay` declaration, is guarded by `phase = Idle and x >= 1 ms` under the time invariant `x <= 3 ms` while `Idle`, and sets `Half`; `G` and `L` are guarded by `phase = Half` and declare `delay Detour::G ~ discrete { 1 ms: 1 }` and `delay Detour::L ~ discrete { 1 ms: 1 }`; `G` sets `Done` and `b`, `L` sets `Done`. The workload `All` weights `A`, `B`, `G` and `L` 1. For `eventually[0 ms, 2 ms] holds(d.b)`, write `Q(g, d)` for the probability after the draw `g` with `B` at delay `d`: `Q(1, 1) = 3/4` (tie: `A` wins `1/2`; `B` wins `1/2`, then `G` wins the tie at 2 ms with `1/2`), `Q(1, 2) = Q(1, 3) = 1` (`A` wins at 1 ms); `Q(3, 1) = 1/2` (`B` at 1 ms, then `G` at 2 ms), `Q(3, 2) = Q(3, 3) = 0` (nothing sets `b` by 2 ms). After the draw the maximum is `(1 + 1/2)/2 = 3/4` and the minimum `(3/4 + 0)/2 = 3/8`; a choice before the draw gives `5/8`, `1/2` and `1/2` for `B` at 1, 2 and 3, so maximum `5/8` and minimum `1/2`. Under `All`, `probability <= 0.7` settles `refuted` on the maximum `ExactValue{3/4}`, its witness entry the `FreeDelays` rows given `[1]` to `B` delay 2 and given `[3]` to `B` delay 1, and with `max_witness_paths` at least 2 its path set (`A` at 1 ms, `1/2`; `B` at 1 ms then `G` at 2 ms, `1/4`) replays `refuted`; `probability >= 0.45` settles `refuted` on the minimum `ExactValue{3/8}`, its witness entry the rows given `[1]` to `B` delay 1 and given `[3]` to `B` delay 2. A product that chose before the draw would prove both claims. | Test (TC-642) |
| FR-204-AC-11 | Every free identity in each row. AC-6's `Race` with a second free operation `C`, guarded by `phase = Idle and x >= 1 ms` under the same invariant, setting `Done` and not `b`, and `Both` weighting `A`, `B` and `C` 1: the initial state offers nine `FreeDelays` actions, each one row given `[2]` assigning both `B` and `C` a delay in `[1, 3]`. The maximum is `ExactValue{1}` with the row `B` delay 1, `C` delay 2; the minimum is `ExactValue{0}` with the row `B` delay 2, `C` delay 1 (`C` wins at 1 ms). A witness row naming `B` alone refuses `invalid_runtime_input`/`invalid-value` at replay. | Test (TC-642) |
| FR-204-AC-12 | Witness at an end component. The model `Loop` (`time dense unit ms`) has `phase` (`Idle` or `Done`) and `b`; `A`, guarded by `phase = Idle`, sets `Done` and `b` and declares `delay Loop::A ~ discrete { 2 ms: 1 }`; `B` has no `delay` declaration, is guarded by `phase = Idle and x >= 1 ms` under the time invariant `x <= 2 ms` while `Idle`, resets `x` and keeps `Idle`; the workload `Both` weights `A` and `B` 1. The initial state offers two `FreeDelays` actions, one row given `[2]` each: `B` delay 1 (`B` wins and returns to the initial state) and `B` delay 2 (tie: either order reaches `b`: `A` first sets `b`; `B` first resets `x` and `A`, still enabled, sets `b`). Both attain the maximum's upper iterate 1. The `Reach` claim `probability <= 0.9 [eventually holds(r.b)]` under `Both` settles `refuted` on the maximum `ExactValue{1}`, its witness entry the row given `[2]` to `B` delay 2 (the MEC exit, not the least delay 1), and the refutation replays `refuted`; the chain the entry `B` delay 1 would induce never reaches `b` (value `0`), which is why the least attaining delay is not the witness. | Test (TC-642) |
| FR-204-AC-13 | Over `Retx` with the guard `self.x > 1 ms`, over every scheduler, `Deadline` builds an MDP product over region states, with no refusal; its exact minimum is `99/100` (attempts at 2 ms and 4 ms) and its exact maximum `999/1000` (three attempts, each just after 1 ms past the last, fit in 4 ms, a fourth does not), so `Deadline` at 0.99 settles `proved` at equality. | Test (TC-2172) |
| FR-204-AC-14 | Over `Retx` over every scheduler, `probability >= 0.99 [always[0 ms, 4 ms] holds(c.late)]` builds a product, with no `TimedFormShape` refusal, and its exact minimum is `0`, because `c.late` is false at the initial state. | Test (TC-2173) |
| FR-204-AC-15 | Over `Retx` with `delay M::Call::send ~ discrete { 1 ms: 1, 2 ms: 1 };`, `Deadline` over every scheduler builds an MDP product with no `DelayDistribution` refusal. | Test (TC-2174) |
| FR-204-AC-16 | Over the `Retx` variant whose `send` has no guard and no reset, `Deadline` over every scheduler builds a product with no `ZeroDelayCycle` refusal, and its exact minimum and maximum are both `1`. | Test (TC-2175) |
| FR-204-AC-17 | Under a workload, `Retx` with `delay M::Call::send ~ discrete { 5 ms: 1 };`, whose `send` window excludes its only delay, builds a product whose time-locked state is terminal, with no `NotStochastic` refusal. | Test (TC-2176) |
| FR-204-AC-18 | Under the workload `Skew` (weights `A` 1, `B` 2, `C` 3), the model `Trio` of the worked example below, whose `A`, `B` and `C` race with the given delays `discrete { 1 ms: 1, 2 ms: 1 }`, `discrete { 2 ms: 1, 3 ms: 1 }` and `discrete { 2 ms: 1, 3 ms: 3 }`, has at its initial state exactly four race edges and no other action (no `UnitDelay`, `FreeDelays` or `WorkloadResolve`), each edge `Identity(A)`, with `A` at 1 ms `1/2`, `A` at 2 ms `53/192`, `B` at 2 ms `7/48` and `C` at 2 ms `5/64`; `probability >= 0.13 [eventually[0 ms, 3 ms] holds(t.b)]` settles `proved`, `ExactValue{7/48}`, `probability >= 0.07 [eventually[0 ms, 3 ms] holds(t.c)]` settles `proved`, `ExactValue{5/64}`, `probability <= 0.8 [eventually[0 ms, 3 ms] holds(t.a)]` settles `proved`, `ExactValue{149/192}`, and `probability >= 0.5 [eventually[0 ms, 1 ms] holds(t.a)]` settles `proved`, `ExactValue{1/2}`; the `t.b` item's `Exact` certificate with values computed with even ties (initial value `11/96`) and the one computed with ties taken in canonical order (initial value `0`) are each rejected `InequalityFails` at the initial state's `ProbabilisticProductState`, and the test submits both. | Test (TC-3204) |
| FR-204-AC-19 | Under the workload `Share` (weights `fire` 4, `G` 1), the model `Pair` of the worked example below, whose operation `fire` has the two tied identities `F1` and `F2` with `discrete { 2 ms: 1 }` and whose `G` has `discrete { 2 ms: 1, 3 ms: 1 }`, divides `fire`'s weight between `F1` and `F2` (2 each against `G` 1): its initial state has exactly three race edges and no other action, `F1` at 2 ms `9/20`, `F2` at 2 ms `9/20` and `G` at 2 ms `1/10`; `probability >= 0.45 [eventually[0 ms, 3 ms] holds(n1.hit)]` and `probability <= 0.45 [eventually[0 ms, 3 ms] holds(n1.hit)]` each settle `proved`, `ExactValue{9/20}`; the `n1.hit` item's `Exact` certificates with the weight not divided (initial value `17/36`) and with even ties (initial value `5/12`) are each rejected `InequalityFails` at the initial state's `ProbabilisticProductState`, and the test submits both. | Test (TC-3205) |
| FR-204-AC-20 | Under the workload `Order` (weights `X` 1, `Y` 2, `Z` 3), the model `Chain` of the worked example below, whose `X`, `Y` and `Z` tie at 2 ms and each fire when still enabled at its turn, orders the tie without replacement by the weights, so `probability >= 0.58 [eventually[0 ms, 3 ms] holds(c.n = 3 and c.last = X)]` settles `proved`, `ExactValue{7/12}`, `probability <= 0.2 [eventually[0 ms, 3 ms] holds(c.n = 3 and c.last = Z)]` settles `proved`, `ExactValue{3/20}`, and `probability <= 0.3 [eventually[0 ms, 3 ms] holds(c.n = 3 and c.last = Y)]` settles `proved`, `ExactValue{4/15}`; the `c.last = Z` item's `Exact` certificates with the first pick weighted and the rest in canonical order (initial value `1/2`), with even orders (initial value `1/3`) and with canonical order (initial value `1`) are each rejected `InequalityFails`, and the test submits all three. | Test (TC-3206) |

The Chain check also asserts that the six firing orders listed in the worked
example are represented as six outcome edges from the initial product key,
with probabilities and destination keys as listed; each edge has one delay
of 2 ms and an ordered identity-sequence label. Its replay witness has one
trace position for each identity in that sequence, at the same timestamp.
Changing any edge's probability or destination is rejected at the initial
product key and that edge label.

## Worked example: given delays racing under weights

`Trio` (`time dense unit ms`) has `phase` (`Idle` or `Done`) and the flags
`a`, `b` and `c`; the operations `A`, `B` and `C` are each guarded by
`phase = Idle` under the time invariant `x <= 3 ms` while `Idle`, set `Done`
and respectively `a`, `b` and `c`, and declare
`delay Trio::A ~ discrete { 1 ms: 1, 2 ms: 1 };`,
`delay Trio::B ~ discrete { 2 ms: 1, 3 ms: 1 };` and
`delay Trio::C ~ discrete { 2 ms: 1, 3 ms: 3 };`. The workload `Skew`
weights `A` 1, `B` 2 and `C` 3. Of the eight draws, four have `A` drawing 1
and win with no tie, `(2, 3, 3)` has `A` alone at 2 ms, `(2, 2, 3)` and
`(2, 3, 2)` are two-way ties and `(2, 2, 2)` the three-way tie. Each edge is
labelled `Identity(A)`, the first racing identity in canonical order
(FR-196). A checker that breaks each tie evenly gets `11/96`, `5/96` and
`5/6` for the `b`, `c` and `a` claims, and refutes all three; one that takes
the first tied identity in canonical order gets `0`, `0` and `1`.

The derivation. A tied
racer wins first with its weight over the tied weights; each draw's
probability is the product of the three conditioned probabilities
(`A` 1/2 at 1 or 2; `B` 1/2 at 2 or 3; `C` 1/4 at 2, 3/4 at 3). Masses are in
units of `1/192`.

| Draw `(a, b, c)` | Probability | Race | `A` wins | `B` wins | `C` wins |
|---|---|---|---|---|---|
| `(1, 2, 2)`, `(1, 2, 3)`, `(1, 3, 2)`, `(1, 3, 3)` | `1/16 + 3/16 + 1/16 + 3/16 = 1/2` (96) | `A` alone at 1 ms | 96 | 0 | 0 |
| `(2, 3, 3)` | `3/16` (36) | `A` alone at 2 ms | 36 | 0 | 0 |
| `(2, 2, 3)` | `3/16` (36) | two-way tie `A`, `B` at 2 ms, weights 1 and 2 | 12 | 24 | 0 |
| `(2, 3, 2)` | `1/16` (12) | two-way tie `A`, `C` at 2 ms, weights 1 and 3 | 3 | 0 | 9 |
| `(2, 2, 2)` | `1/16` (12) | three-way tie at 2 ms, weights 1, 2, 3 over 6 | 2 | 4 | 6 |
| Total | `1` (192) | | 149 | 28 | 15 |

Totals: `A` `149/192`, `B` `28/192 = 7/48`, `C` `15/192 = 5/64`, and
`149 + 28 + 15 = 192`. `A` wins at 1 ms with `96/192 = 1/2` and at 2 ms with
`53/192`. The exact values of the claims are `P(b) = 7/48`, `P(c) = 5/64`,
`P(a) = 149/192` over the window `[0 ms, 3 ms]` the claim uses (all of
`A`'s mass is by 2 ms, so `[0 ms, 2 ms]` gives the same) and
`P(a) = 1/2` over `[0 ms, 1 ms]`. Breaking ties
evenly changes the three tie rows to `A`/`B` 18/18, `A`/`C` 6/6 and
`A`/`B`/`C` 4/4/4, giving `A` 160, `B` 22, `C` 10 (of 192), so `P(b) = 11/96`,
`P(c) = 5/96`: with `0.13` and `0.07` the even checker refutes what the
weights prove. Ties by canonical order give `A` 192, `B` 0, `C` 0.

The without-replacement orders give the winner marginals above: for the
three-way tie `A` first is `1/6`, `B` first `1/3` and `C` first `1/2`, and the
second pick is weighted over the remaining two (`ABC` `1/6 · 2/5 = 1/15`,
`ACB` `1/6 · 3/5 = 1/10`, `BAC` `1/3 · 1/4 = 1/12`, `BCA` `1/3 · 3/4 = 1/4`,
`CAB` `1/2 · 1/3 = 1/6`, `CBA` `1/2 · 2/3 = 1/3`; total 1). Each operation
sets `Done`, so only the first is enabled at its turn. `A`, `B` and `C` are
three distinct operations of one identity each, so no operation's weight is
shared among tied identities here; AC-19 covers that case and AC-20 covers
the picks after the first.

The `t.b` certificates of AC-18 are `Exact` documents. Their Bellman equality
fails at the initial state, where the rebuilt successors weight the ties by
`Skew` (value `7/48`) and the document says `11/96` or `0`; every state after
the race has `phase = Done` and no race, so the initial state is the first
failing key. A `Lower` or value-bounds document with `11/96` or `0` would
satisfy its lower inequality, and meets FR-201's `ThresholdMissed` against
`0.13` instead; AC-18 is stated for the `Exact` kind only.

## Worked example: an operation's weight shared among its identities

Model `Pair` (`time dense unit ms`) has `phase` (`Idle` or `Done`), a
population of two `Node` objects `n1` and `n2` each with `hit`, and the flag
`g`. The operation `Node::fire` has one identity per object (`F1` for `n1`,
`F2` for `n2`), guarded by `phase = Idle`, setting `Done` and its object's
`hit`, with `delay Pair::Node::fire ~ discrete { 2 ms: 1 };` for each identity; the
operation `G` is guarded by `phase = Idle`, sets `Done` and `g`, with
`delay Pair::G ~ discrete { 2 ms: 1, 3 ms: 1 };`. The invariant is
`x <= 3 ms` while `Idle`. The workload `Share` weights `fire` 4 and `G` 1.
ADR-026 SD-3 orders the tied identities by the operation weights and then
uniformly within an operation, so the weight of `fire` is divided between
its two tied identities: `F1` and `F2` carry 2 each against `G` 1 when all
three tie, and 2 each between themselves when `G` draws 3 ms.

| `G` draw | Probability | Race | `F1` wins | `F2` wins | `G` wins |
|---|---|---|---|---|---|
| 2 ms | `1/2` | three-way tie at 2 ms, weights 2, 2, 1 over 5 | `1/2 · 2/5 = 1/5` | `1/5` | `1/2 · 1/5 = 1/10` |
| 3 ms | `1/2` | `F1`, `F2` tie at 2 ms, weights 2, 2 | `1/2 · 1/2 = 1/4` | `1/4` | 0 |
| Total | `1` | | `1/5 + 1/4 = 9/20` | `9/20` | `1/10` |

`9/20 + 9/20 + 1/10 = 9/20 + 9/20 + 2/20 = 20/20`. All three edges are at
2 ms. A checker that gives each of `F1` and `F2` the whole weight 4 (the
operation's weight not divided) gets `F1` `4/9` for the `G` draw 2 row, so
`F1` wins `1/2 · 4/9 + 1/4 = 2/9 + 1/4 = 8/36 + 9/36 = 17/36` and `G` wins
`1/2 · 1/9 = 1/18`. A checker that ties evenly gets `F1` `1/2 · 1/3 + 1/4 =
1/6 + 1/4 = 5/12` and `G` `1/6`. Over 180, `9/20 = 81`, `17/36 = 85` and
`5/12 = 75`. Against the threshold `0.45 = 9/20`, the claim `>= 0.45` holds
at 81 and 85 and fails at 75, and the claim `<= 0.45` holds at 81 and 75 and
fails at 85, so the pair holds only at the exact value. The `G` edge mass is
`1/10` (exact), `1/18` undivided and `1/6` even.

QSL-2984 resolves subsequent picks: each identity begins with its operation's
weight divided by the number of that operation's tied enabled identities, and
the picked identity's share is removed. A still-enabled sibling retains only
its original divided share.

## Worked example: picks after the first

Model `Chain` (`time dense unit ms`) has the counter `n` (`Int[0, 3]`, initially
0) and `last` (`None`, `X`, `Y` or `Z`, initially `None`), and the flags `x`,
`y`, `z`. The operations `X`, `Y` and `Z` are guarded by `not x`, `not y` and
`not z` respectively (no shared guard), each sets its flag, increments `n` and
sets `last` to its own name, and each declares `delay Chain::X ~ discrete { 2 ms: 1 };`
(and the same for `Y` and `Z`). All three tie at 2 ms with certainty, the
model delays once and takes them in the drawn order at that instant, each
still enabled at its turn, so all three fire and `last` is the final one of
the order. The workload `Order` weights `X` 1, `Y` 2 and `Z` 3. The order is
drawn without replacement:

| Order | Probability | `last` |
|---|---|---|
| `XYZ` | `1/6 · 2/5 = 1/15` | `Z` |
| `XZY` | `1/6 · 3/5 = 1/10` | `Y` |
| `YXZ` | `1/3 · 1/4 = 1/12` | `Z` |
| `YZX` | `1/3 · 3/4 = 1/4` | `X` |
| `ZXY` | `1/2 · 1/3 = 1/6` | `Y` |
| `ZYX` | `1/2 · 2/3 = 1/3` | `X` |

Over 60: `4 + 6 + 5 + 15 + 10 + 20 = 60`. `P(last = X) = 1/4 + 1/3 = 7/12`,
`P(last = Y) = 1/10 + 1/6 = 3/30 + 5/30 = 4/15` and
`P(last = Z) = 1/15 + 1/12 = 4/60 + 5/60 = 3/20`; over 60 these are 35, 16 and
9, total 60. A checker that weights the first pick only and takes the rest in
canonical order gets `XYZ` `1/6`, `YXZ` `1/3` and `ZXY` `1/2`, so `last = X`
`0`, `Y` `1/2` and `Z` `1/2`. A checker that orders evenly gets `1/3` each. A
checker that takes canonical order gets `last = Z` with 1. The claims hold
only at the exact values: `probability >= 0.58 [eventually[0 ms, 3 ms]
holds(c.n = 3 and c.last = X)]` (`7/12 = 0.5833`) refutes under all three
wrong rules (`0`, `1/3`, `0`), and `probability <= 0.2 [eventually[0 ms,
3 ms] holds(c.n = 3 and c.last = Z)]` (`3/20 = 0.15`) refutes under all
three (`1/2`, `1/3`, `1`).

QSL-2983 resolves the product representation: each ordered tie outcome is one
product edge, while replay exposes one trace position per firing identity at
the shared instant. A certificate rejection is located at the source product
key and the complete ordered edge label.

## Rulings

Lead rulings of 2026-11-04 on the post-draw choice (QSL-1667), relayed from
the FSL plan lead (not an owner answer; Peter may veto). Each row is decided;
the last column names the fact that reopens it.

| Question | Ruling | Reason | Reopen when |
| --- | --- | --- | --- |
| How the post-draw choice is represented | As the MDP action `FreeDelays` (FR-196) at the pre-draw state, one action per function from given vectors to free assignments as the semantic choice set, built and evaluated row by row (FR-196), not as an intermediate chance-then-decision state with a new ProductKey member. | ADR-026 SD-3 names the choice; the factorization in Behavior shows the action form has the value of ADR-028 SCH-2's intermediate-state form for every objective EN-5 decides, so no new ProductKey tag is needed. | An objective whose one-step operator is not linear in the successor distribution is admitted at a mixed race; then a pending given-draw ProductKey member with canonical bytes replaces the row-factored operator. The built size is `s · w^k` row choices either way, so size is no reason to reopen. |
| Which free identities a row covers | Every free racing identity at the state; a row naming a subset refuses. `WorkloadResolve` names one free identity before the draw; in a mixed state with one free identity it equals the constant `FreeDelays` action, with two or more it names no SD-3 choice, and the mixed state offers `FreeDelays` only (author decision under this ruling). | SD-3's race delays once over every racing identity, so it needs every free delay in one step; keeping both names in a mixed state would give one decision two spellings. | SD-3 is amended to let the workload exclude free identities from a race. |
| Witness types in scope | `WitnessScheduler` and `WitnessEntry` (FR-202) are specified here; FR-202's separate `WitnessChoice` wrapper is removed, the choice is FR-196's `ProductAction`, of which `FreeDelays` is a member (author decision under this ruling). | One closed choice type gives one canonical body for the product edge, the witness entry and the certificate policy; a second enum with a second `free-delays` spelling would duplicate it. | A witness choice is needed that is not a product action. |

## Dependencies

- The post-draw `FreeDelays` action (AC-6 to AC-12) is implemented under
  QSL-1667, in the lane of the EN-5 provider ticket QSL-1498. That change
  replaces the mixed-state `WorkloadResolve` path in the digital product in
  the same PR; it is not a Layer 4 deletion-table unit.
- ADR-028 TA-1 to TA-7, TA-1a, RU-6, XV-4; ADR-026 TS-1, TS-2, TS-4, CK-4,
  CK-7, SD-1, SD-3, SD-4, CT-1, CT-3, EZ-10 (on its own draft branch).
- [FR-196](FR-196-build-the-probabilistic-product.md),
  [FR-197](FR-197-decide-finite-horizon-forms-by-exact-backward-induction.md),
  [FR-198](FR-198-decide-unbounded-reachability-and-expected-rewards.md),
  [FR-202](FR-202-replay-a-probabilistic-witness.md).

## References

- FSpec half, which owns the digital route's semantics: FSpec FR-421
  (Linear STD-139), with FR-420's given and free delays.
- ADR-026 (timed subjects, delay distributions and the race) is on its own
  draft branch: Linear QSL-373.
- Owning ticket: Linear QSL-371.
