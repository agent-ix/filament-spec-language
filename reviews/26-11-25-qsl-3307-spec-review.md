---
id: SR-202611251216002
title: "spec-review of PR 3917 (QSL-3307) rational budget of unbounded exact solves"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR #3917 head d1e9e1b118ba2292e36eced6d15426a53a82d8e3; ADR-028 UR-4/LM-1, FR-198 Rational budget and FR-198-AC-7..9, FR-201 cyclic until/always emission, FR-1042 Inputs, TC-633"
review_set: subset
---

## Summary

Ticket: QSL-3307. This spec review covers `git diff origin/main...HEAD` at head d1e9e1b118ba2292e36eced6d15426a53a82d8e3, a spec-only PR with five files. Each ruling was checked against FSpec origin/main 5ea68995b7cead077ad96fbff5ab0a564d593525: FR-411 Arithmetic, Unbounded steps 3 and 4, Verdicts, and TC-365 XL-01. It was also checked for consistency with ADR-028 UR-3/UR-4/UR-5/LM-1/LM-2, FR-196, FR-197, FR-198, FR-201, FR-203, FR-1041, FR-1042 and TC-633. The numbers in the new ACs were worked out by hand against the solver code: `reachability_threshold`, `interval_sweep`, `round_dyadic` and `initial_intervals_decide` in fsl-analyze/src/exact_probabilistic/unbounded.rs, and the unbounded path in fsl-analyze/src/analyze/exact.rs. EARS and integrity sub-analyses are folded into this artifact and were not run as separate files. `quire validate` passes on all five edited files. No test embeds the edited spec files; the only `include_str!` spec readers name FR-255, FR-499, FR-500 and FR-521.

## Verdict

NOT MERGEABLE at d1e9e1b118ba2292e36eced6d15426a53a82d8e3 until FND-001 is fixed.

The rulings themselves hold:

- **Per-part measure.** No FSpec requirement forces a different answer. FSpec FR-411 says only "a value's bit length". FSL FR-197 and the FR-1042 scalar_bits rule already measure per part. The statement that a probability v and 1 − v share a reduced denominator no numerator exceeds is correct, because gcd(b−a, b) = gcd(a, b) = 1.
- **No rounded exact value.** Past the budget, the solve falls back to interval iteration at doubled precision, with PrecisionBudget at max_precision_bits. This matches FSpec FR-411 Arithmetic ("reruns over dyadic intervals ... doubling p up to max_precision_bits while the interval contains the threshold"), the Verdicts table and TC-365 XL-01.
- **LimitReached for cyclic until and always.** FR-201's stop is kept and stays consistent with FR-201-AC-65 and FR-1042 Bases.

The amendment weakens nothing relative to main. It removes rounding of exact values into Bounds and widens the FR-201 cyclic trigger from the final violated quantity to any value the solve computes.

The hand arithmetic checks out:

- **FR-198-AC-7, Loop.** 1/2 takes 1 and 2 bits, and every state and action value is in {0, 1/2, 1}. The sum+1 measure would give 4 bits, so the AC tells the two measures apart.
- **FR-198-AC-7, Link.** 24 and 25 take 5 bits each.
- **FR-198-AC-8 at p = 4.** The lower iterate sticks at 7/16 and the upper at 9/16, which contains 12/25, so the run doubles to p = 8. At p = 8 the third sweep gives lower 123/256 > 12/25 and upper 133/256, so the result is Bounds at precision_bits 8. With max_precision_bits 4 the interval [7/16, 9/16] gives PrecisionBudget.
- **FR-198-AC-9.** The complement interval at p = 4 is [0, 1/16], so the claim's own interval is [15/16, 1], which contains 0.95 and 24/25. That gives PrecisionBudget on the claim's own value, not LimitReached, because acyclic Link uses the state-predicate/v1 base.
- **AC strength.** AC-7 and AC-8 fail on main. AC-9 fails if the complement is not taken or if the cyclic LimitReached stop is used.

On the three divergences the author reported:

- **Exact solve before interval iteration (exact.rs).** This amendment must address it, because FR-198-AC-7's Link half hard-codes it (FND-001). The code reordering itself can stay separate once no new AC depends on it.
- **Fair path writes an interval whenever max_rational_bits < 8 * max_precision_bits.** This is separate: the amendment explicitly leaves fairness to FR-1041, and the fair code already diverges from FR-1041's "when the exact values exceed" on main.
- **decide_expected_reward_threshold rejects a multi-action MDP.** This is a separate code defect, but the new ruling now requires the fallback there and no AC exercises it (FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-198-AC-7 (and TC-633 step 7) requires Link's until and always claims at threshold 0.95 with default precision limits to settle `ExactValue{24/25}`. FR-198 Reachability bullets 3-4, ADR-028 UR-3/UR-4 and FSpec FR-411 Unbounded steps 3-4 run interval iteration first and stop once the interval separates. Link is acyclic, so at precision 64 the interval reaches 24/25 within 2^-64 in about three sweeps and separates from 0.95. A conformant EN-5 returns Bounds and settles ValueBounds{IntervalIteration{64, …}} with lower and upper documents. Only main's exact-first analyze path (analyze/exact.rs:1693-1710) yields ExactValue, so the AC hard-codes the pre-existing ordering divergence. Fix: use the equality threshold `rational(24,25)` with a small `max_iterations`, as AC-3 does (the interval cannot decide at equality), or amend UR-3/UR-4, which FSpec FR-411 would also have to change. | spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md:192; spec/test-cases/TC-633-unbounded-reachability-and-expected-rewards.md:36-38 |
| FND-002 | low | The Rational budget rule requires a fallback Bounds to carry "that iteration's ... sweeps", but no AC pins sweeps, and main's reachability_threshold accumulates sweeps across precision restarts (unbounded.rs:577, 600; not reset per precision). AC-8's run would record 103 sweeps on main and 3 under the rule. Either pin the sweep count in FR-198-AC-8 or state that it is cumulative. | spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md:139-142; fsl-analyze/src/exact_probabilistic/unbounded.rs:577 |
| FND-003 | medium | The Rational budget rules also govern the expected-reward fallback (UR-5 "as in UR-4"; FR-1042 Bases names cyclic-mdp-expected-reward/v1). No new AC exercises an expected reward over an MDP with choices past the budget. On main, decide_expected_reward_threshold refuses such a product with InvalidState (reward_interval.rs:64-72), which is the author's third divergence. Add an AC, for example Link's `cost` claim (FR-198-AC-2, minimum 11/10) with a small max_rational_bits, so the code fix for this ruling is tested. | spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md:122-153; fsl-analyze/src/exact_probabilistic/reward_interval.rs:64-72 |

## Coverage

- Examined: ADR-028 UR-4 and LM-1 edits; FR-198 Rational budget bullets 1-5; FR-198-AC-7, FR-198-AC-8 and FR-198-AC-9; FR-201 cyclic until/always emission edit; FR-1042 Inputs bullet; TC-633 description, scope, steps 7-9 and expected results.
- Context: FSpec FR-411 (Unbounded, Arithmetic, Verdicts), TC-365 XL-01; FR-197 per-part rule; FR-203 limits/defaults (precision_bits 64) and settlement table; FR-1041 budget bullet; FR-1042 Bases, scalar_bits, FR-1042-AC-7/AC-20..24; FR-201-AC-62/AC-65; Link fixture (fsl-analyze/tests/it/prob_world.rs:840); solver code above.
- Not run: cargo (not needed for a spec-only PR).
