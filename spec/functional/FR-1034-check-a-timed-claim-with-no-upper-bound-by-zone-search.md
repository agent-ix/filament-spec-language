---
id: FR-1034
title: "Check a timed claim with a lower-bounded interval and no upper bound by zone search"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-234
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-239
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-240
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-242
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-244
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-943
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-766
    type: depends_on
---
# FR-1034: Check a timed claim with a lower-bounded interval and no upper bound by zone search

## Description

EN-6 SHALL decide a timed claim over a timed subject whose formula holds an
interval `[q, *]` or `(q, *)` (FR-943, FSpec FR-766) by the symbolic zone
search of FR-239, with the claim automaton of FR-242 extended by a
**lower-bound-only** component for each such operator: its clock carries the
lower-bound guard and no upper guard, one clock serves every start of the
operator, and the zone abstraction that already keeps the search finite
(ADR-026 EZ-4) covers the clock because its only constant is `q`. Such a
claim is never refused for having no upper bound.

### Decision: one clock, the lower bound as its only constant

An interval with no upper end has no deadline, so the two devices FR-242 uses
for bounded intervals do not apply: the horizon clock of a `BoundedWindow`
needs every upper bound, and MightyL's block copies of an operator started at
many positions grow with `a / (b - a)`, which has no finite value when `b` is
absent. Neither is needed. For a lower bound alone the pending instances of
one operator are ordered by implication, so the automaton keeps the one that
implies the rest and needs one clock however many positions start the
operator; and a clock compared only with `q` takes finitely many abstract
values, so the LU bounds of FR-239 extrapolate it with no new mechanism. MITL
with non-punctual intervals, `[q, *]` included, is decidable (ADR-026 DF-5),
so deciding these claims is a complete procedure, not a bounded
approximation.

## Inputs

- A checked timed clause (FR-234) over a timed subject, with at least one
  interval operator whose upper end is `*`, and its `TimedPropertyForm`.

## Outputs

- The claim automaton of FR-242, with one lower-bound-only clock per such
  operator, its acceptance sets and its constants; and EN-6's verdict and
  certificate (FR-239, FR-240, FR-244).

## Behavior

### Form

- A formula that holds an interval with no upper end SHALL NOT classify as
  `BoundedWindow`, whose horizon sums upper bounds (FR-234); it SHALL classify
  as `TimedSafety` when it is in ADR-014 A-4's safety fragment and as
  `TimedLiveness` otherwise, and SHALL be translated by FR-242's claim
  automaton for TT-3 and TT-4. `[q, *]` is non-punctual for every finite `q`,
  so FR-234's `PunctualInterval` disposition SHALL NOT apply to it.
- `[0 ms, *]` SHALL translate as the operator with no interval (FR-943).

### Future operators (`until`, `release` and their derived forms)

In the negation normal form of FR-242 (`eventually_I` is `true until_I`,
`always_I` is `false release_I`):

- **Guard.** A pending `ψ until[q, *] χ` instance SHALL be discharged by `χ`
  at a position whose clock `x` satisfies `x >= q` (`x > q` for `(q, *)`),
  SHALL stay pending by `ψ` at any clock value, and SHALL never die by the
  passing of a deadline: there is no upper guard. A pending
  `ψ release[q, *] χ` instance SHALL require `χ` at every position whose
  clock satisfies the lower guard until a position holds `ψ`, and is never
  discharged by the passing of a deadline.
- **Acceptance.** Because no deadline forces a pending `until[q, *]`
  instance to settle, the operator SHALL carry an acceptance set (FR-240).
  That set SHALL be fulfilled at a step where the operator is not pending,
  where it is discharged, or where it is kept pending at a position that
  holds both `ψ` and `χ`. On a time-divergent run (EN-6's divergence set,
  ADR-026 EZ-6), a run that fulfils the set infinitely often satisfies every
  instance the operator started: each kept start is followed by a position
  holding `χ` with `ψ` between, and time divergence carries the clock of
  some such position past `q`. The rule does not ask the kept instance itself
  to be discharged, because under keep-newest with a reset at each start a
  position that both starts and witnesses the operator restarts its clock
  below `q`, so the newest instance can stay pending forever on a run that
  satisfies every instance, such as `always (p implies eventually[q, *] r)`
  with `p` and `r` true at every position.
- **One clock per operator.** An operator started at many positions SHALL
  keep one clock, never blocks of copies. Of two pending `until[q, *]`
  instances the newer implies the older, since a witness at least `q` after
  the newer start is at least `q` after the older one and the older's `ψ`
  obligation covers the newer's span; the automaton SHALL keep the newer and
  reset the clock at each new start. Of two pending `release[q, *]`
  instances the older implies the newer, since its region `[t + q, ...)`
  contains the newer's; the automaton SHALL keep the older and SHALL NOT
  reset the clock at a later start. This is the reverse of FR-242's choice
  for an interval from a closed 0 (older `until`, newer `release`), because
  the bound that orders the instances is the lower end here, the upper end
  there.

### Past operators (`since`, `triggered` and their derived forms)

- The memory of `ψ since[q, *] χ` SHALL be one clock measured from the
  **earliest live candidate**: a position holding `χ` with `ψ` at every later
  position up to the current one. A position where `ψ` is false SHALL clear
  the candidate, then a `χ` at that position SHALL start a new one; a `χ`
  while a candidate is live SHALL NOT reset the clock. The operator SHALL be
  true at a position exactly when a candidate is live and its clock
  satisfies the lower guard. `once[q, *] χ` is the case `ψ` true: its clock
  starts at the first `χ` and is never reset.
- `triggered` and `historically` SHALL be read as the duals
  (`not (not ψ since not χ)`, `not once not χ`) over the same memory.

### Clocks and abstraction

- Each lower-bound-only clock SHALL be compared with `q` only; `q` SHALL join
  that clock's `L` and `U` bounds of FR-239 and the scale, and the absent
  upper end SHALL contribute no constant. The aLU simulation of FR-239 then
  identifies every valuation of the clock above `q`, so the clock takes
  finitely many abstract values and the search stays finite with no limit
  added for it.
- A lower-bound-only clock SHALL be **inactive** wherever its value cannot
  change a guard: with no pending instance, with no live candidate, and,
  for `once[q, *]`, once the guard has held (the operator stays true). An
  inactive clock is unconstrained in the zone (FR-230), so symbolic states
  that differ only in it coincide.
- The certificate (FR-244) SHALL carry these clocks and their LU bounds like
  any other claim-automaton clock, and the core checker (FR-245) SHALL
  rebuild the same automaton; no certificate field is added.

### Agreement

- For every timed trace the automaton SHALL accept exactly the traces on
  which the timed evaluator (FR-231) returns `false` for the clause, as
  FR-242 requires of every claim automaton. The batch evaluator's reading of
  `[q, *]` (every stamp difference from `q` on) is the oracle.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-1034-AC-1 | Each of `eventually[1 ms, *] holds(q)`, `always (holds(p) implies eventually[1 ms, *] holds(q))`, `holds(p) until(1 ms, *] holds(q)`, `eventually always[1 ms, *] holds(q)`, `always (holds(q) implies once[1 ms, *] holds(p))` and `eventually (holds(q) and (holds(p) since(1 ms, *] holds(r)))` translates to a claim automaton, never refused `OpenTimedInterval` or `ClockLimit`; on 1,000 random timed lassos per formula with stamps in steps of `1/4` ms and length at most 8, the automaton accepts exactly the lassos on which the evaluator returns `false`, and over all formulas some lasso holds and some is refuted. | Test (TC-2969) |
| FR-1034-AC-2 | `always (holds(p) implies eventually[2 ms, *] holds(q))`, `eventually always[2 ms, *] holds(q)` (a `release` started at many positions) and `eventually (holds(q) and once[2 ms, *] holds(p))` each translate with exactly one automaton clock, whose only constant is 2 ms. | Test (TC-2969) |
| FR-1034-AC-3 | Over ADR-026 §11's `Rpc` with client deadline 4 ms (reply at a clock of at least 1 ms and at most 3 ms after `send`), EN-6 proves `always (holds(c.phase = 1) implies eventually[1 ms, *] holds(not (c.phase = 1)))` and `always (holds(c.phase = 2) implies once[1 ms, *] holds(c.phase = 1))`, each certificate accepted by the core checker; it refutes `always (holds(c.phase = 2) implies once(1 ms, *] holds(c.phase = 1))` with `send` then `reply` at delays 0 and 1, the exact lower bound, and that counterexample replays; and it refutes `always (holds(c.phase = 1) implies (holds(c.phase = 1) until[4 ms, *] holds(c.phase = 2)))`, the server deadline forcing the reply before 4 ms. No outcome is `Stopped` and none refuses `OpenTimedInterval`. | Test (TC-2969) |
| FR-1034-AC-4 | On the time-divergent timed lasso of two positions stamped 0 ms and 1 ms, both at level 1, re-entering position 0 2 ms after it, the automaton of `always (holds(c.level = 1) implies eventually[1 ms, *] holds(c.level = 1))`, whose `p` and `r` hold at every position, does not accept the lasso and the evaluator returns `true`; on the same lasso the automaton of `always (holds(c.level = 1) implies eventually[1 ms, *] holds(c.level = 2))`, whose `until[1 ms, *]` is never discharged, accepts it and the evaluator returns `false`. | Test (TC-2980) |

## Dependencies

- [FR-242](FR-242-translate-a-timed-formula-into-a-claim-automaton.md) owns the
  claim automaton this requirement extends; [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md)
  owns the search and the aLU abstraction; [FR-240](FR-240-decide-timed-liveness-and-time-lock-freedom-on-the-symbolic-graph.md)
  owns acceptance and divergence; [FR-943](FR-943-check-and-plan-a-timed-interval-with-no-upper-bound.md)
  owns the checking of `[q, *]`.
- FSpec FR-766 owns the `[q, *]` interval under the timed profile.

## References

- R. Alur, T. Feder and T. A. Henzinger, "The benefits of relaxing
  punctuality", JACM, 1996 (ADR-026 References).
- G. Behrmann, P. Bouyer, K. G. Larsen and R. Pelánek, "Lower and upper
  bounds in zone-based abstractions of timed automata", STTT, 2006.
