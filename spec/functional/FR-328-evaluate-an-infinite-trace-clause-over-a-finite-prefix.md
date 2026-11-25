---
id: FR-328
title: "Evaluate an infinite-trace clause three-valued over a finite prefix"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-161
    type: depends_on
---
# FR-328: Evaluate an infinite-trace clause three-valued over a finite prefix

## Description

When the TemporalTrace evaluator (FR-327) evaluates a clause under
`filament.temporal.infinite-trace/v1` over a finite trace, it SHALL read the
trace as a prefix of an unknown infinite continuation and return holds,
violation or pending (ADR-014 §5 A-4, FSpec FR-161-AC-2). It SHALL return
holds for a **good prefix**, one every infinite extension of which satisfies
the formula at its activation position (`eventually p` once `p` has held),
whatever fragment the formula is in. It SHALL return violation only for a
formula in the safety fragment whose prefix is a **bad prefix**: every
infinite extension of it makes the formula false. The safety fragment
admits interval operators (ADR-018 IV-4). Every other prefix is pending.
This reading is sound both ways: a holds is a verdict every extension
confirms, and so is a violation, so neither depends on the unknown
continuation.

## Use case

A runtime monitor watches a service under an infinite-trace claim. When a
request goes unanswered past its deadline, the monitor reports the violation
as soon as the prefix shows it. When an eventuality has been met, every
continuation keeps it met, so it reports holds. When the prefix settles
nothing, such as an `always` with no counterexample yet, an eventuality not
met yet or a deadline that has not passed, it reports pending, so a short
observation never reads as a guarantee it does not give.

## Inputs

- A `CheckedTemporalClause` under the infinite-trace profile (FR-326,
  FR-123).
- A finite `TemporalTrace` with no loop (FR-327), the `over` binding, and a
  `Meter`.

## Outputs

- `Completed(true)` (holds) for a good prefix at the activation position;
- `Completed(false)` (violation) with the `TemporalPosition` of the
  activation that the bad prefix falsifies; `Undefined` (violation) as
  FR-327 states; or `Pending` (category pending); or
  `MissingFairnessPremise{constraint}` (O-16 unsupported); or `Incomplete`
  with its charge point.

## Behavior

- When the clause's fairness set is non-empty, the evaluator SHALL return
  `MissingFairnessPremise{constraint}` naming the first constraint of the
  checked fairness set, and SHALL evaluate no formula: a supplied trace
  carries no enabledness, so the fairness premise is missing (FSpec FR-362,
  wire cause `unsupported_projection`/`missing-fairness-premise`). It maps
  to O-16 unsupported.
- The evaluator SHALL treat every position after the finite trace as
  unknown, with no stutter or false-extension; terminal stutter applies only
  to a model behaviour that ends at a terminal state (FR-125, ADR-018 SM-4).
- The safety fragment SHALL be the formulas whose negation normal form uses
  only atoms, negated atoms, `and`, `or`, `always`, `release`, past
  operators and interval operators, `eventually[a,*]` and `until[a,*]`
  excluded since they are as unbounded as `eventually` and `until`
  (ADR-014 A-4 as amended by ADR-018 IV-4). Whether a clause is in the fragment SHALL be read from its checked
  formula.
- When the clause is in the safety fragment and every infinite extension of
  the prefix makes it false at its activation position, the evaluator SHALL
  return `Completed(false)` at that position.
- When every infinite extension of the prefix makes the clause true at its
  activation position (a good prefix), the evaluator SHALL return
  `Completed(true)`, whether or not the clause is in the safety fragment.
  Unknown positions after the prefix may take any values, so a good prefix
  is one whose verdict no continuation changes: an `eventually`, `until` or
  `eventually[a,b]` whose witness the prefix holds, or a formula whose
  satisfied part the prefix decides.
- When a letter of the prefix is undefined, the evaluator SHALL return
  `Undefined` by FR-327's rule, whatever the fragment: every extension of
  the prefix holds that letter.
- In every other case, including a bad prefix of a formula outside the
  safety fragment, the evaluator SHALL return `Pending`. An undefined letter
  takes precedence over a good prefix, as over a bad one.
- A past operator, with or without an interval, SHALL read every atomic
  predicate as false before position 0 (ADR-018 IV-2, FSpec FR-092's
  complete-history rule).
- `Pending` SHALL map to the category pending (FR-285), `Completed(false)` to
  violation (FSpec FR-324-AC-2) and `Completed(true)` to success with truth
  `true`, the category a lasso's `Completed(true)` maps to; a finite-replay
  item settles it `proved` with disposition kind `holds` (FR-941). The
  evaluator SHALL never return `tested` over a finite prefix.
- The verdicts SHALL equal the tick monitor's (FR-251) over the same
  position-counted prefix: the monitor settles an `eventually` instance
  `true` at its witness and leaves an `always` `Pending`, never decided
  `true` by the absence of a counterexample.
- The evaluator SHALL charge and exhaust work as FR-327 states.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-328-AC-1 | Over the `Counter` prefix with `c.value` 0, 1, 2 under infinite-trace: `always holds(c.value <= 1)` is `Completed(false)` at position 0; `always holds(c.value <= 5)` is `Pending`; `eventually holds(c.value = 2)` is `Completed(true)` (a good prefix: position 2 is a witness every extension keeps), and `eventually holds(c.value = 9)` is `Pending`. | Test (TC-838) |
| FR-328-AC-2 | Over the same prefix, `always (holds(c.value = 1) implies eventually[0,1] holds(c.value = 3))` is `Completed(false)` at position 0 (positions 1 and 2 are both present and neither has value 3); `always (holds(c.value = 2) implies eventually[0,1] holds(c.value = 3))` is `Pending` (position 3 is unknown). | Test (TC-838) |
| FR-328-AC-3 | Over the same prefix, `always (holds(c.value = 0) implies once[1,1] holds(c.value = 9))` is `Completed(false)` at position 0, because `once[1,1]` at position 0 reads before position 0, where atoms are false. | Test (TC-838) |
| FR-328-AC-4 | No clause in AC-1 to AC-3 yields `tested`; each `Pending` maps to the category pending, and AC-1's `Completed(true)` maps to success with truth `true`. | Test (TC-838) |
| FR-328-AC-5 | Over the prefix of the record subject `c: Counter` (attribute `level`) with `c.level` 0, 1, 2 under infinite-trace, the clause checked by the temporal family alone (S3 would refuse its condition's integer `/`) and its letter supplied by the test, which reads a zero divisor as undefined with cause `division-by-zero`, `eventually holds(2 / (2 - c.level) = 3)`, outside the safety fragment, returns `Undefined{where: 2, cause: division-by-zero}`, not `Pending`. | Test (TC-847) |
| FR-328-AC-6 | Over the `Counter` prefix with `c.value` 0, 1, 2 under infinite-trace, `always holds(c.value <= 1)` with the fairness set `fair weak inc` returns `MissingFairnessPremise` naming `fair weak whole inc`, O-16 unsupported, and evaluates no position; with an empty fairness set it is `Completed(false)` at position 0. | Test (TC-848) |
| FR-328-AC-7 | Over the `Counter` prefix with `c.value` 0, 1, 2 under infinite-trace: `eventually holds(c.value = 1)`, `holds(c.value <= 1) until holds(c.value = 2)` and `eventually[0,2] holds(c.value = 2)` are each `Completed(true)` at position 0 (good prefixes, outside the safety fragment or not); `always eventually holds(c.value = 2)` and `eventually always holds(c.value = 2)` are `Pending` (no finite prefix is good or bad for them); `eventually holds(c.value = 9) or always holds(c.value <= 1)` is `Pending` although its right disjunct is violated, since a bad prefix outside the safety fragment does not settle; and the tick monitor of FR-251 over the same three positions gives the same verdict for each of `eventually holds(c.value = 1)` and `always holds(c.value <= 5)`. | Test (TC-2974) |

## Dependencies

- ADR-014 §5 A-4; ADR-018 §2 SM-1 and SM-4, §11 IV-2 and IV-4.
- [FR-327](FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md).
- FSpec FR-161-AC-2, FR-324-AC-2, FR-092, FR-362 (the missing fairness
  premise).

## References

- Linear QSL-384 (spec ticket); QSL-43 (implementation).
- FSpec FR-367 and FR-362: Linear STD-131 (QS-4, QS-13).
