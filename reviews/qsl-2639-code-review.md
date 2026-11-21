---
id: SR-202611210524090
title: "Code review — QSL-2639 PR #3262 joint Fourier-Motzkin elimination of coupled free post-jump variables"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3262 diff at 93e630d56f2d27995545728b7b87a942e8acf8d5: fsl-replay/src/hybrid_replay/exact.rs, fsl-replay/src/hybrid_replay/exact/joint.rs, fsl-replay/src/hybrid_replay/tests/quantifier_tests.rs"
review_set: subset
evaluated_revision: "93e630d56f2d27995545728b7b87a942e8acf8d5"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-450
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-750
    type: references
---

## Summary

Ticket: QSL-2639 (PARTIAL slice; the ticket stays open). PR #3262 at head `93e630d5` groups free post-jump variables that one conjunct couples (transitively, with their single-variable conjuncts) into a `Joint` (exact.rs:1222-1268) and decides existence at each dwell instant by Fourier-Motzkin elimination in the new fsl-replay/src/hybrid_replay/exact/joint.rs. The rust-review lane (idioms, panic surface, metering, test conventions and tags) is folded into this file.

Soundness checked and clean: conditions are normalised to `form < 0` / `form <= 0` (joint.rs:216-228), `=` as two non-strict inequalities, `!=` refused as `CoupledReset`; a positive coefficient is an upper bound and a negative one a lower bound (joint.rs:257-267); pairing multiplies the lower condition by `c_high > 0` and the upper by `-c_low > 0` so the variable cancels and the result is strict iff either side is strict (joint.rs:269-284), which is exact real FM for mixed strict/non-strict systems; a coefficient zero at the instant drops the term and keeps the condition (joint.rs:261-264); a variable with bounds on one side only is eliminated with no residue (the unbounded case); the final constant check is `< 0` / `<= 0` read exactly (joint.rs:288-300). Every coefficient and constant sign is read with `ExactReal::sign_of` at the (possibly algebraic) instant and pushed as a decision, and `cells_of` (eval.rs:401-450) splits each open cell at the roots of every decision until no root is interior, so per-cell evaluation follows one branch path; RatFunc denominators are non-zero at the instant (divisors are checked, `reduced` only cancels common factors). An undefined subterm makes the joint false, matching `Enabled::holds` and `Section::exists`. Product of two free-variable forms, a conditional, a range check, `not`/`or` and `!=` remain `ExactGap::CoupledReset` (mapped to `NotExact`), never a guess. Uncoupled variables keep the bivariate `Section::exists` path unchanged.

Tests: filtered `coupled_free hybrid_replay::` run at head (with the two brief skips), 93 passed. Probes and mutations in a throwaway worktree (`/home/peter/dev/worktrees/QSL-2639-review-mut`, never pushed): probes (point-only enabledness with `<=` vs `<`, coefficient `u = t - 1/2` changing sign, zero coefficient at an isolated instant, unbounded variable, non-linear `v * w` → `NotExact`, uncoupled regression) all agree with hand analysis. Mutations: swapping lower/upper (M5) fails both PR tests. Pairing strictness `||`→`&&` (M1), reading `<` as `<=` (M2) and a non-strict final check (M7) pass both PR tests and are killed only by the reviewer's point-enabledness probe. Dropping the coefficient sign decision (M3), treating a zero coefficient as an upper bound (M4), dropping the final constant decision (M6) and dropping a zero-coefficient condition (M8) pass both PR tests and every fairness-level probe.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The two new tests use only non-strict bounds and constant coefficients, so mutations to strictness (pairing `strict: low.strict && high.strict`, `<` read as `<=`, final check non-strict) and to the sign machinery (coefficient sign decision dropped, zero coefficient treated as an upper bound, zero-coefficient condition dropped, final constant decision dropped) all pass them; strictness is observable through a point-only enabled set (reset `w >= 1`, `v >= v`, target `v + w <= 1` vs `< 1`), and the sign/zero paths need a direct `Joint::exists` unit test at a zero-coefficient and an irrational instant asserting the truth value and decisions | fsl-replay/src/hybrid_replay/tests/quantifier_tests.rs:339-432, fsl-replay/src/hybrid_replay/exact/joint.rs:257-299 |
| FND-002 | low | `ExactGap::CoupledReset`'s doc and the TEMPORARY comment still describe every coupled reset ("two or more variables free that the target mode's invariant couples in one condition"), but after this PR the gap is only a non-linear coupling (product of free variables, conditional, range, `!=`, non-comparison) | fsl-replay/src/hybrid_replay.rs:398-401, fsl-replay/src/hybrid_replay.rs:529-533 |
| FND-003 | low | Sign reads in the elimination use the unmetered `ExactReal::sign_of` (Infallible charge) on coefficients whose degree grows with each pairing, whereas the single-variable `Section` path meters its sign reads via `sign_with`; the meter is charged only per conjunct and per pairing (size product), so bisection work at an algebraic instant is unaccounted (exhaustion still yields the limit refusal, never a wrong verdict) | fsl-replay/src/hybrid_replay/exact/joint.rs:257-259, fsl-replay/src/hybrid_replay/exact/joint.rs:289-291 |

## Verdict

CONDITIONAL — the elimination is sound and exact for linearly coupled free variables (strict/non-strict pairing, exact sign reads at algebraic instants, zero coefficients, one-sided bounds, empty joint solutions, no regression for uncoupled variables, non-linear coupling honestly `NotExact`). FND-001 should be closed in this PR with a strict point-enabledness test and a direct `Joint::exists` test; FND-002 and FND-003 are small fixes.
