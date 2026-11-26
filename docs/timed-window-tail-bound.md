# Idle tails of a timed window monitor: the bound argument

This note holds the argument behind FR-196 "Idle tail of a timed window
monitor" (QSL-3324). It is a hand argument. One author wrote it and one
independent checker went through it step by step. Nothing in it was run or
machine-checked. Each claim is marked:

- **[proved]**: argued in full here, and the checker found it holds.
- **[sketch]**: argued at the level of a standard textbook lemma, not written
  out in full.
- **[cited]**: a literature result recalled, not re-proved here.
- **[open]**: not proved either way.

The rule in FR-196 relies only on §2, §3, §4 and item 7's divergence fact
(§5). §6 explains why some cases end at the limit stop. §7 records the
criterion values the argument confirms.

## 1. Definitions

- **Subject.** A probabilistic timed automaton with discrete state, clocks,
  guards and invariants. After scaling, every constant is an integer, and
  each constraint is closed or strict. Discrete steps can have
  probabilistic outcomes, and delays can be `discrete`. A scheduler chooses
  delays and actions as any measurable function of the whole timed history,
  dense time stamps included.
- **Positions** (FSpec FR-416, FR-196, FR-204 "Positions"). Position 0 is at
  time 0. Every discrete step adds its post-state at its time stamp. When
  the behaviour has ended its discrete steps at a quiescent state at time
  `T`, it continues with idle observation stutters at times
  `T < s_1 < s_2 < …`, which diverge and are chosen by the scheduler. A
  delay is not a position. After the first idle stutter no discrete step
  follows.
- **Formula.** `φ` is in negation normal form, `on origin`, with every
  interval bounded and integer-ended. `n` is the number of interval operator
  occurrences. The horizon `H` is the largest sum of upper ends along a
  root-to-leaf chain. An obligation of occurrence `o` started at time `u`
  concerns only positions in `u + J_o`, and every such window lies in
  `[0, H]`.
- **Pointwise meaning.** `eventually_J ψ` holds at `i` iff some `j >= i` has
  `τ_j − τ_i ∈ J` and `ψ` at `j`. `p until_J q` holds iff some such `j` has
  `q` and every `k ∈ [i, j)` has `p`. `always` and `release` are the duals.
- **Monitor.** This is FR-196 "Timed window monitor key". The residual is a
  positive Boolean combination of obligations `(kind, o, elapsed)`. A read
  applies `R` to every obligation present, and `S` starts new occurrences
  at elapsed 0. A delay advances every elapsed clock. An obligation past its
  upper end becomes `reject` (eventually, until) or `accept` (always,
  release).
- **Fragment F.** These are formulas without clock atoms, built from `∧`,
  `∨`, `eventually_J` and `always_J`. They may also use `p until_J q` and
  `p release_J q`, but only when `p` is a Boolean combination of state
  predicates.

## 2. The region product with the monitor is exact

**Lemma 2.1 [sketch].** Let `M` be the largest constant among guards,
invariants and interval ends. Two product states are equivalent when they
agree on all four of:

1. the discrete state and the tail phase;
2. the set of obligation clocks;
3. the region of the combined vector of model clocks and obligation clocks:
   integer parts capped at `M`, which fractional parts are zero, and the
   order of the fractional parts;
4. the residual.

This equivalence is a time-abstract probabilistic bisimulation. It
preserves action labels, outcome probabilities and acceptance.

The argument is that every monitor update reads only three facts: whether
the elapsed time is in `J_o`, expiry, and exact equality of two obligation
clocks (used for deduplication). Each is a clock constraint with constant
at most `M`, or a region property. A new obligation is a fresh clock reset
to 0. The Alur–Dill region lemma holds for any finite clock set, so it
applies state by state while the clock set changes **[cited: Alur and
Dill 1994]**. Outcomes depend only on guards, and the region decides the
guards.

**Consequences.**

- (a) On any finite part of the product, the region MDP's extrema equal the
  dense infimum and supremum, provided the schedulers are restricted to
  time-divergent ones (§5). They are attained by region-memoryless
  schedulers, because each region move can be realised by a concrete delay
  **[cited: Kwiatkowska, Norman, Segala and Sproston, TCS 2002]**.
- (b) Pointwise MTL truth is not a function of the model clocks' region.
  That region forgets the time stamps of earlier positions that pending
  obligations refer to, and the obligation clocks carry exactly those time
  stamps. The only obstruction left is that the number of obligation clocks
  can be unbounded.

## 3. Idle tails: the witness bound for F

Fix a prefix that ends at a quiescent state at time `T` with residual `ρ`
over `m` pending obligations. Every tail position has the same discrete
state, so every state predicate is constant on the tail. No probabilistic
outcome remains, and the scheduler chooses only the stutter set
`S ⊂ (T, ∞)`. Positions after `H` affect nothing. `S ∩ (T, H]` is finite
when `S` diverges.

**Lemma 3.1 (removal) [proved].** Removing a stutter from a tail leaves an
admitted tail. Two positive delays merge into one, the state stays
quiescent, and the other positions keep their time stamps and letters.

**Lemma 3.2 (F in the tail) [proved].** At a tail position with constant
`p`:

- `p until_J q` is `eventually_J q` when `p` is true, and `q ∧ 0 ∈ J` when
  `p` is false.
- `p release_J q` is `0 ∈ J → q` when `p` is true, and `always_J q` when `p`
  is false.

The checker added that an `until` with `p` false, or a `release` with `p`
true, is folded to a constant when the position at `T` is read. So neither
is pending at tail entry.

**Theorem 3.3 (tail witness bound) [proved].** Suppose some stutter set `S`
makes `ρ` true. Then some `C ⊆ S` with at most `K` stutters in `(T, H]` also
makes `ρ` true, where:

```text
K = (m + 2) · Σ_{d=0..D} n^d
D = (n + 1)(⌊H⌋ + 1)
```

The same holds for "makes `ρ` false", by applying the theorem to the NNF
dual of `ρ`. The dual is in F again, because negation turns `p until q`
into `¬p release ¬q` and the left operand is still a state predicate.

*Proof.*

1. **Requirements.** Fix a disjunct choice that is true on `S`, taking the
   first true disjunct each time. A requirement is a pair `(t, ψ)`: a point
   (a tail stutter, or the start time of a pending obligation) and a
   subformula occurrence that holds at `t` on `S`.
2. **Latest witnesses.** For a requirement `(t, eventually_J χ)`, let
   `w(t, o)` be the latest `x ∈ S ∩ (t + J)` at which `χ` holds on `S`. It
   exists because the set is finite and the requirement holds.
3. **Closure.** Start `C` with the witnesses of the root requirements (at
   most `m`) and the first stutter of `S`. Then close it under two rules:
   - every `(t, always_J χ)` gives `(x, χ)` for every `x ∈ C ∩ (t + J)`;
   - every `(t, eventually_J χ)` adds `w(t, o)` with the requirement
     `(w, χ)`.

   `C ⊆ S`, so the closure terminates.
4. **`C` satisfies `ρ`.** The proof is by induction on the subformula.
   Atoms are unchanged by Lemma 3.1. An `eventually` keeps its witness. An
   `always` requirement is closed over every point of `C` in its window.
   Removed points were not witnesses, so removing them only shrinks the
   ranges of the universal operators.
5. **Counting.** Each added point has a creator `(t, o)` with `t` earlier,
   and at most one child per occurrence, so it branches at most `n` ways.
   Take a creator chain `w_0 < w_1 < … < w_L`. Suppose `o_k = o_{k'}` for
   some `k < k'`. Then `w_{k'}` is later than `w_k` and has `χ_o` on `S`,
   but `w_k` is the latest such point in `w_{k−1} + J_o`. So
   `w_{k'} ≥ w_{k−1} + sup J_o`, and `sup J_o ≥ 1` because
   `w_k − w_{k−1} > 0` lies in `J_o` and the ends are integers. Any `n + 1`
   consecutive links repeat an occurrence, so they advance time by at
   least 1. All points lie in `[0, H]`, so `L < D`. There are at most
   `m + 2` roots, which gives the bound. ∎

**Corollary 3.4 [proved, with a measurability gap].** The extrema over
every scheduler equal the extrema over schedulers with at most `K` tail
stutters in `(T, H]`. To see this, replace each scheduler's tail by the
reduced tail of Theorem 3.3. The gap is that the replaced scheduler must be
measurable, which is not written out. The checker's view is that it is
easy: a canonical region path whose delays are piecewise-affine in the
entry valuation, for example each new fraction placed at the midpoint of
its neighbours, gives a Borel choice. FR-196 avoids the replacement
altogether. It defines the tail-entry state's value directly as the
indicator of "some tail reaches the target", and region exactness
(Lemma 2.1) makes that value a function of the key.

**Size.** `K` is not usable as a working budget. For
`always[0, 4] eventually[2, 2]` in milliseconds, `n = 2`, `H = 6` and
`D = 21`, so `K` is about `4 · 2^22`. Every criterion case is decided with
at most 5 tail stutters (§7), but only the hand derivations show that. No
general theorem gives a smaller bound. FR-196 therefore uses `K` only as a
completeness bound, its stop (c).

**Not covered [open].** Theorem 3.3 does not cover two cases:

- Clock atoms. The tail letters then vary with time, and Lemma 3.2 fails.
- `until` or `release` with a temporal left operand. The latest-witness
  choice does not preserve the left-operand constraint over `[i, j)`.

FR-196 applies stop (c) only in F.

## 4. The `["window",residual]` key as written

**4.1 Not finite on `Retx` [proved].** Take
`always[0, 4] eventually[2, 2] D`, with the first attempt at 1 ms delivered
(`T = 1`). The outer `always` obligation from 0 stays pending until elapsed
time exceeds 4. Each stutter in `(1, 4]` starts an `eventually[2, 2]`
obligation, and `k` stutters at distinct fractions in `(1, 2)` give `k`
distinct obligations. So the reachable keys are infinite, and the idle tail
needs FR-196's tail rule.

**4.2 Exact [proved, given Lemma 2.1].** Two states with the same key bytes
are bisimilar.

**4.3 Growth outside F with no new clock [proved].** Take
`φ = (eventually[0, 1] a) until[0, 2] (eventually[1, 1] b)` over a subject
with a zero-delay step that leaves `a` and `b` false.

- Reading position 0 gives `r0 = or(E'0, and(E0, U0))`.
- A second read at the same time stamp gives
  `or(E'0, and(E0, or(E'0, and(E0, U0))))`.

The second residual is the same Boolean function as `r0` by absorption,
but its bytes differ, and every further read adds one level. FR-196's
normalisation has no absorption rule. So outside F a zero-delay cycle can
make the product infinite, and such runs end at the limit stop.

**F residuals [sketch].** The claim is that in F, re-reading at the same
time stamp with the same letter is idempotent after flattening and
deduplication, so a zero-delay cycle adds no new residual. The missing
lemma is short (induction on the formula) but is not written out.

## 5. Time divergence

Every obligation descends from position 0 and expires by `H`, so once time
passes `H` every residual is `accept` or `reject` **[proved]**. In a
bounded-window product, a behaviour that stays forever in undecided states
therefore keeps time at most `H`, so it is Zeno. Schedulers must let time
diverge with probability 1. Every end component made only of undecided
states must therefore be left, and its value is its best exit (FR-196 item
7, FR-924). The checker flagged this as gap G1 in the original argument,
which stated the region-MDP equality without restricting to divergent
schedulers. The criterion values do not change.

The rule reads divergence as almost-sure. Under the every-path reading,
§6's `Try` maximum is a supremum that no scheduler attains. FSpec FR-416
owns that choice **[open]**.

## 6. Positions before the tail: an unbounded case

**Model `Try` [proved].**

- Dense time in milliseconds, with one object holding Booleans `ok` and `X`
  (both false at start).
- `try` draws `r ~ {true: 1, false: 1}` and sets `ok = r`.
- `mark` sets `X = true`.
- No guards and no invariants.

The claim is:

```text
Φ = eventually[0, 1] holds(ok) and
    always[0, 1] (holds(ok) or (eventually[1, 1] holds(X) and always[0, 0] holds(not ok)))
```

- **Claim 6.1.** `max P(Φ) = 1`, attained under almost-sure divergence.
  The scheduler makes its k-th try at `1 − 2^−k` and stops at the first
  success. It then makes a `mark` at `s + 1` for every failed stamp `s`, and
  one at 1 for position 0.
- **Claim 6.2.** Every scheduler that keeps at most `B` pending obligations
  has `P(Φ) ≤ 1 − 2^−(B+1)`. A success needs a fresh time stamp, and each
  fresh failure leaves its own pending `eventually[1, 1] X`.

So every truncation of the product is inexact. No finite exact abstraction
is known for such models, and none is known to be impossible **[open]**.
FR-196 stops them with `Incomplete(LimitReached)`. Whether `Φ` passes
FSL's admission grammar exactly as written was not checked. It meets
FR-186's closed, bounded, `on origin` conditions.

## 7. Criterion values confirmed

The checker recomputed each of these by hand. All formulas are in F.

| Criterion | Formula | Values |
| --- | --- | --- |
| FR-204-AC-21 | `eventually[0, 4] always[1, 1] D` | min `99999/100000`, max `1` |
| FR-204-AC-21, FR-498-AC-8 | `always[0, 4] eventually[2, 2] D` | min `0`, max `99/100`; rejection value `9/10` |
| FR-204-AC-21 | single-operator and `until` forms | min `99/100`, max `9999/10000` |
| FR-204-AC-26 | Retx `always[0, 4] always[2, 2] false` | min `0`, max `1`, digital max `9/10` |
| FR-204-AC-26 | Gap `always[1, 2] false` | min `0`, max `1`, digital max `0` |

The extremal schedulers use at most 5 tail stutters in the window.

For the tail rule specifically, FR-196's reviewer checked every
criterion's tail-entry states by hand:

- Each is decided at layer `L_0` to `L_5` by stop (a), or by folding.
- In AC-21's `eventually`-then-`always` form, entries fold to accept.
- Gap and the `Step` items are decided before tail entry.

The layer sizes against the default `exact_prob.max_states` were estimated
only, not measured.

## 8. Summary of what is not proved

- Lemma 2.1 and the F-residual lemma are sketches.
- The measurability in Corollary 3.4 is not written out. FR-196 does not
  depend on it, because it defines the tail value directly.
- There is no completeness bound for tails outside F (clock atoms, or a
  temporal left operand of `until` or `release`).
- There is no finite exact abstraction for `Try`-like prefixes.
- Attainment on infinite products is open in general, and depends on the
  divergence reading.
- `K` is not claimed tight.
- The cited literature results were recalled, not re-proved.
- A claim that models of some Bounded-MTL formulas need doubly exponentially
  many events per time unit (Bouyer, Markey, Ouaknine and Worrell, LICS
  2007) was read but not checked. Nothing in FR-196 depends on it.
