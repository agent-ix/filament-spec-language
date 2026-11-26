---
id: ADR-038
title: "Timed analysis: one concrete run layer and one route admission table"
type: ADR
relationships:
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-416
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-421
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-751
    type: depends_on
---

# ADR-038: Timed analysis: one concrete run layer and one route admission table

Status: Proposed (QSL-3490, step 1 of QSL-3489). This record decides
structure and route admission. FSpec FR-416 owns the meaning of a timed run
and FSpec FR-421 the admission rule for exact probabilistic claims; this record
states the layering and the justification of each route. Code that disagrees
with it is changed by the later steps of QSL-3489.

## Context

The same rules for a timed run were stated, and implemented, in several places
that disagreed: exact checking said a strict constraint or a delay
distribution under every scheduler is `unsupported` in one record and decided
over regions in another; vacuity was stated for every claim over a time-locked
state while probabilistic items count such runs; a requirement said only
probabilistic items use digital clocks while its own behaviour routed a
temporal item there; and EN-5 was required to reuse `model_check`'s product
while its checker is required to share nothing with it. Digital clocks were
admitted for forms that no cited theorem covers.

## Decision

### Layers

| ID | Rule |
| --- | --- |
| RL-1 | **One concrete run layer.** The qualified core (`fsl-eval`) holds the one implementation of the concrete timed run of FSpec FR-416: exact clock values, delay admissibility and strictness, the discrete step, positions and time stamps, the idle tail, the local time-lock test, and the run, position and outcome types. Every producer, sampler, replay and certificate checker uses it. No other crate defines a second position, run or timed outcome type. |
| RL-2 | **Abstractions are written twice.** Zones, integer clocks and regions, their successors, races, claim monitors, progress tests and component decompositions are written separately in the producer and in the checker (FSL FR-921, FSpec FR-751). What RL-1 shares is the concrete run only: a monitor or progression function, such as `advance_nested_timed_deadline`, is not part of it, and neither is a product `TransitionSystem` or an end-component decomposition. |
| RL-3 | **A region MDP is symbolic.** The region route's state is a symbolic region key (each clock's integer part, whether its fraction is zero, and the order of the fractions) and its delay move is the region's time successor. A walk over a finer grid of clock values is not a region MDP and settles no exact probabilistic timed claim. While the region MDP a claim needs is not built, the item settles a typed `Incomplete`, never a digital-clock value and never `unsupported`. |
| RL-4 | **A dense-time falsification oracle, in tests only.** Tests hold an oracle that enumerates rational delay schedules over the RL-1 run layer and compares the probability or verdict each gives with the value a route claims. It can only refute a value. No product path calls it and it emits no certificate. |
| RL-5 | **One type per product state key.** A key of a timed product, the region key included, is one type with one validator and one encoder, used by the certificate writer and the checker. The region time successor, the reading of a guard on a region and the computation of a region's fraction classes are written separately on each side (RL-2). |
| RL-6 | **Witness replay does not sample.** Replay of a sampled witness re-derives the run from the record on the RL-1 layer, checks every recorded draw against its window, its distribution and its recomputation from the seed (FSpec FR-420, FSL FR-254), and evaluates the claim with the shared evaluator. It does not call the sampler. |

RL-2 does not decide one sharing that exists today: the zone producer and the
zone checker both translate a clause with `fsl-eval`'s position automaton and
window claim, a reading of the formula and not of the clocks. The requirement
that declares each certificate checker's trusted base decides it.

**Wire change RL-6 needs** (not specified here; QSL-3493 specifies it in FSpec
FR-331, FR-410, FR-420 and FSL FR-193). A sampled-witness step carries
`scheduled`, `random` and `delay`, and the delay is the winning one. RL-6
needs, for each step, the identity, drawn delay and choice index of every
identity that raced, and the step's rewards; and, on the witness, how the run
ended (the step that crosses the horizon, whose reward a weighted fraction
reads as in FSL FR-254-AC-5; the idle tail; or a time-lock) with its time
stamp, a race that advances time and takes no step included.

**Zeno trap.** A Zeno trap with no exit is a time-lock (FSpec FR-416
"Time-locks"; FSpec FR-421 and FR-753 treat an end component with no exit so).
On regions a trap can hold delay moves and still not diverge, so each side
applies the progress condition of FSpec FR-753 and not a zero-delay test
alone.

### Drafted amendment to FSL FR-921 (for the arch ruling; not applied)

FR-921 item 1 has the checker compute the route with code EN-5 does not share,
and item 5 the same for the end components and time-locks. One admission table
read by both (QSL-3494), and one reading of a locked run, contradict that if
"route" and "time-lock" mean the declaration as well as the facts. The draft
separates them:

- **Shared declarations.** The route admission table (its rows and the
  conditions each row tests) and the mapping of FSpec FR-416 "How claims read
  a time-lock" from a claim monitor's kind and reading to a value are declared
  once, in the qualified core, and read by EN-5 and its checker. Neither
  computes a fact about a subject.
- **Item 1** becomes: every condition a row tests (closedness, delay class,
  whether a time-lock is reachable, claim form, position read), each
  established by the checker's own code; and the route the declared table
  gives those conditions.
- **Item 5** becomes: the maximal end components, their exits, their progress
  and the time-locks it finds, by its own code; a locked state's value is the
  shared mapping.

A wrong row or a wrong mapping is then a defect both sides carry. The theorem
cited by each row, review, and the RL-4 oracle are what catch it. Without the
amendment each side writes the table and the mapping itself, and the two can
drift apart, as the producer's and checker's region scale rules do today.

### Route admission

A route decides a claim only where a row admits it. A subject is **closed**
when every clock constraint, time invariant and claim interval is non-strict;
every constraint is diagonal-free (ADR-026 CK-4). The delay classes `given`,
`free` and `mixed` and the time-lock condition are those of FSpec FR-421
"Closed and strict conditions".

"Checked" means the author of this record read the cited statement. A row
marked "argument below" rests on an argument written here that no published
theorem states and that no second reader has checked.

| Route | Claim form | Subject class | Justification |
| --- | --- | --- | --- |
| EN-6 zones | Every non-probabilistic timed item: TT-1 to TT-4, deadlock freedom, time-lock freedom, state-graph items | Any timed subject | Region equivalence is a time-abstract bisimulation (Alur and Dill, 1994), with zone extrapolation for diagonal-free automata (Behrmann, Bouyer, Larsen and Pelanek, 2006). Statements not checked; existing route, unchanged |
| EN-1 `digital-clocks` | TT-1 over the discrete state; location reachability (`possible`) | Closed | KNPS 2006 Lemma 17 (checked: `t - t' ~ c` for `~` in `<=`, `=`, `>=` survives rounding with any threshold) and Lemma 20 (a path diverges exactly when its rounding does) |
| EN-1 `digital-clocks` | Deadlock freedom | Closed | Lemmas 17 and 20, argument D below |
| EN-1 `digital-clocks` | The digitisable claims of FR-243 (a) to (f) | Closed | Lemmas 17 and 20, argument A below |
| EN-5 digital | The rows of FSpec FR-421's table marked `all` and `given only` | Closed, class `given` | Argument G below |
| EN-5 digital | Reachability and until over state predicates; `expected accumulate`; measures of the discrete step sequence | Closed, class `free` or `mixed` (argument M below), no time-lock | KNPS 2006 Theorem 24 (locations, minimum and maximum), Theorem 27 (expected cost) and Proposition 23 (for each threshold an integer-time adversary gives the rounded paths their probabilities), all checked, over adversaries that diverge with probability 1 (Definition 8) |
| EN-5 digital | `expected elapsed until holds(B)` | the same | KNPS 2006 Theorem 27 (checked) |
| EN-5 digital | Deadline events, `steps` and `accumulate` within a time horizon, each `on origin`; `M <= c` of an elapsed measure that starts at position 0 on every run | the same | Proposition 23, argument B below |
| EN-5 region | Every other exact probabilistic timed claim with `discrete` or free delays | Any such subject | Region graph of a probabilistic timed automaton (Kwiatkowska, Norman, Segala and Sproston, 2002). Statement not checked. A claim monitor adds clocks to the region key |
| EN-4 sampler | Statistical items | Every racing delay given | A measurement, not a proof (ADR-026 SS-2, SS-3) |
| Replay | A counterexample or witness of any route | Any | Executes the concrete run (RL-1); no abstraction |
| Certificate checkers | Zone and timed-product certificates | As the route certified | Rebuild their own abstraction (RL-2) |

KNPS 2006 is M. Kwiatkowska, G. Norman, D. Parker and J. Sproston,
"Performance analysis of probabilistic timed automata using digital clocks",
Formal Methods in System Design 29.

**Not on digital clocks.** Everything outside the table: a strict constraint
or interval; any other bounded-window formula; in classes `free` and `mixed`,
a form read from a position after position 0, a fraction over a time window, a
`weighted by delay` long-run fraction, and any claim over a subject with a
reachable time-lock; an unweighted fraction over a time window in every class;
a mixed subject whose drawn window reads a clock that need not be whole; and,
on EN-1, `always possible`, `unique path` and every claim FR-243 declines.

**One case this table does not decide.** A workload race among two or more
identities with free delays: choosing "this identity later than the winner"
is a strict comparison, so the race is not a closed automaton edge and no row
above is shown to cover it. The free-delay workload rule is held by a
separate ruling; until it is made the route stays as FR-204 states.

**Region measures with no route.** A `weighted by delay` measure turns on
exact delays that a region key does not hold. Outside class `given` no route
above is shown to decide it, and it settles a typed `Incomplete`.

### Arguments

**A (one reference time, EN-1 temporal claims).** A temporal claim is refuted
by one behaviour, so the rounding threshold may be chosen for that behaviour.
Closed comparisons survive every threshold (Lemma 17); only a strict one, from
a violated interval, can be lost. FR-243 (c) to (e) leave at most one
occurrence whose violation needs strict comparisons, all measured from one
position at time `r`. For a one-sided upper bound `t - r > D` over later
positions, rounding with threshold equal to the fraction of `r` keeps every
such comparison; for the other one-sided cases a threshold just below that
fraction, or equal to it, does the same by symmetry.

**B (origin).** Rounding every time stamp up fixes time 0 and, for a whole
`D`, `t <= D` holds exactly when the rounded `t <= D`. The threshold does not
depend on the run, so Proposition 23 gives one integer-time adversary under
which a deadline event, or a count or sum over the positions stamped at most
`D`, has the probabilities it has densely. Rounding down does the same for
`t < D`. A reading from a later position `r` needs a threshold that depends on
`r`, which differs between runs, so no single adversary is given and no row
admits it. FSpec FR-421's examples and the vectors below show such readings
are wrong on digital clocks.

**M (mixed delays).** Where every clock that bounds a drawn identity's window
is whole at the draw, the conditioned distribution depends only on whole
values, so each draw is a probabilistic edge of a closed automaton and the
`free` rows apply. Where one is not whole, rounding can admit a delay the
dense window excludes and change the distribution, so no row applies.

**D (deadlock freedom).** A dense deadlock is a quiescent state from which no
step is ever enabled. Delay it past every constant and round: the integer run
reaches the same discrete state with every clock past every constant, where
each closed guard has the same truth, so it is a deadlock there. In the other
direction, a closed guard enabled after some real delay from an integer
valuation is enabled after a whole delay.

**G (given delays).** With every delay drawn from a `discrete` distribution
and constants scaled to integers, every time stamp before an idle tail is a
whole number on every run: the digital process is the dense process. Only the
idle tail's stutter stamps are chosen freely; a form that reads state
predicates, or weights the tail by 0 or by the whole rest of its window, reads
the same on every choice. A count of positions in a time window does not.

### Hand vectors for the oracle

Each has a closed, diagonal-free subject.

- **Positive lower bound** (FR-204 `Gap`): `always[1 ms, 2 ms] holds(false)`
  has dense maximum 1, integer maximum 0.
- **Reading from a later position:** `first` at `t` in `[0, 1]` ms resets `x`
  and draws `lost` with 1/2. A lost run starts a measure at `t` and ends it at
  `y = 2`, so `M = 2 - t`; otherwise it starts at `y = 1` and ends at `x = 2`,
  so `M = t + 1`. The dense minimum of `Pr(M <= 1 ms)` is 0, the integer one
  1/2. This applies to an elapsed quantile, and equally to a deadline, `steps`
  or `accumulate` horizon read from the position that starts `M`.
- **Time-lock** (FSpec TC-374 `Split`): the dense minimum of reaching `Done`
  is 0, the integer one 1/2.
- **Count over an idle tail** (FSpec TC-374 `Burst`):
  `fraction holds(phase = 1) over [0, 1 ms]` is `(1 + m)/(2 + m)` for every
  `m >= 0`, supremum 1; whole-unit stutters give at most 2/3.

## Passages replaced

This record replaces the statements below; each now points here.

| Passage | Was | Is |
| --- | --- | --- |
| ADR-026 SS-7, §9, EZ-10 | Exact claims need closed constraints; strict ones and distributions under every scheduler are `unsupported`; only probabilistic claims use digital clocks | The route admission table; regions decide the rest; EN-1 also takes temporal claims on request |
| ADR-026 TD-1, TD-6 | Every claim over a time-locked state is vacuous | Temporal claims only; probabilistic items count the run (FSpec FR-416) |
| ADR-028 TA-1, TA-3, TA-4 | A timed claim is read on the digital route when closed | The route admission table |
| ADR-028 CE-7 | The product construction is shared with EN-1 | RL-1 and RL-2 |
| FSL FR-196 Description | EN-5 reuses `model_check`'s product, monitors and decompositions | The concrete run is shared; EN-5's abstraction and products are its own |
| FSL FR-243 Description | Only a probabilistic claim takes the digital-clock route | A temporal claim takes it on request |
| FSL FR-204, FR-877, FR-924, FR-254 | Their own route and time-lock statements | FSpec FR-416, FSpec FR-421 and this table |

## Consequences

- Items move from a digital-clock verdict to the region route, and settle
  `Incomplete` until RL-3 is built: forms read from a later position, fractions
  over a time window, and claims over subjects with a time-lock, in the
  classes named above.
- Digital admission in classes `free` and `mixed` needs a time-lock test over
  the dense runs. EN-6's time-lock-freedom item asks only that some run lets
  time diverge, which is weaker than "with probability 1" and does not answer
  it.
- The sampler reads a time-locked sample (FSL FR-254).

## References

- ADR-026, ADR-028, ADR-029.
- T. A. Henzinger, Z. Manna and A. Pnueli, "What good are digital clocks?",
  1992. Its statements were not available to check; no row rests on it.
- R. Alur and D. Dill, "A theory of timed automata", 1994.
- M. Kwiatkowska, G. Norman, R. Segala and J. Sproston, "Automatic
  verification of real-time systems with discrete probability distributions",
  Theoretical Computer Science 282, 2002.
