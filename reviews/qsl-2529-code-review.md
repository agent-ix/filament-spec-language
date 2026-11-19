---
id: SR-2026111919340478491681417
title: "code-review of QSL-2529 exact replay of non-deterministic reset fairness"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2529; PR3119 head 28af05612194b1fcca6fb378909e51a3933a6cfb; git diff origin/main...HEAD; fsl-replay/src/hybrid_replay.rs; fsl-replay/src/hybrid_replay/{poly2.rs,section.rs,eval.rs,exact.rs,expand.rs,tests.rs,tests/quantifier_tests.rs}; fsl-analyze/tests/it/hybrid_requests.rs"
review_set: subset
---

## Summary

Ticket: QSL-2529 (slice of QSL-2234), PR filament-spec-language#3119, frozen head 28af05612194b1fcca6fb378909e51a3933a6cfb. Code-review with the rust-review lane folded in. The change replaces `ExactGap::NondeterministicReset` with exact real quantifier elimination over each free post-jump variable (Collins-style bivariate projection in `poly2.rs`, Sturm-Tarski sections at rational or real-algebraic instants in `section.rs`) and adds a new honest gap `ExactGap::CoupledReset`. `ExactGap::ActivationState` stays (remaining QSL-2529 work).

## Verdict

CONDITIONAL. The algebra is sound and exact. Hand-checked: (1) projection of `x^2 - t` is `{-t, -4t}` (resultant of `x^2 - t` and `2x` is `-4t` from the 3x3 Sylvester determinant); (2) Collins without reducta is sound here because every coefficient is in the projection set, so on each open `t`-cell every coefficient is nonvanishing (or the zero polynomial), degrees are constant, and every psc is nonvanishing or identically zero, so gcd degrees, real-root counts and pairwise common-root counts are invariant (delineability); point cells are evaluated directly; (3) the psc matrix uses formal degrees with coefficients read through `get(degree)` after `at_t`, so a vanishing leading coefficient at an interpolation node is handled, and the interpolation degree bound `(n-j)deg_t(a) + (m-j)deg_t(b)` is right; (4) the pseudo-remainder sign bookkeeping (`lead^steps`) in `negated_remainder` is right, the cancelled top coefficient is removed exactly, and `Sturm-Tarski(p, p'q)` with `Below`/`Above` signs `lead*(-1)^deg`/`lead` matches the standard TaQ; (5) root comparison, refinement and between-root sampling terminate and are exact; (6) strict vs non-strict and open vs closed are decided by sampling every root and every gap exactly, and `cells_of` keeps endpoints as point cells; (7) the lazy fixpoint in `Section::exists` and `cells_of` is sound: every polynomial read on the evaluation path at a sample is sign-invariant over its cell, so every point of the cell follows the same path. No floats anywhere. `cargo-slot cargo test -p fsl-replay --lib hybrid` 62 passed (plus two reviewer probes, reverted) and `-p fsl-analyze --test it hybrid_requests::` 8 passed, 13 ignored. Clippy (-p fsl-replay -p fsl-analyze --all-targets, cap-lints warn): one warning, at hybrid_requests.rs:907, pre-existing (e008f7baa7), none in the diff. No `#[allow]`, no unsafe, no new `pub` beyond the `ExactGap::CoupledReset` variant replacing `NondeterministicReset`.

The `CoupledReset` gap is an acceptable tracked partial, not a spec violation to block on: it deviates from FR-450 :224-234 and :235-238 exactly as `ActivationState` does, is marked TEMPORARY, settles honestly `NotExact`, and is tracked by QSL-2639 (sub-issue of QSL-2529, Specced). The code cites QSL-2529 instead of QSL-2639 (FND-004).

Blocking: FND-001 (a decidable reset shape silently settles `NotExact` under the wrong gap) and FND-002 (the metering test passes with every elimination charge deleted).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Guard and fixed conjuncts are evaluated with `activation: &[]`; an `=` reset reading a free variable's pre-jump value (e.g. `reset { v >= 1 and w = v + 1 }`) makes the target conjunct read `Activation(v)`, raising `ExactGap::ActivationState` and settling `NotExact` for a decidable claim (reproduced; `activation: self.solution` fixes it) | fsl-replay/src/hybrid_replay/exact.rs:1222-1228 |
| FND-002 | medium | `quantifier_elimination_is_charged_to_the_callers_meter` cannot fail if elimination stops charging: with both elimination charges deleted the bound-30 meter still exhausts (segment charges alone consume 49; with charges 137) | fsl-replay/src/hybrid_replay/tests/quantifier_tests.rs:233-253 |
| FND-003 | medium | Seven quantifier tests claim FR-450-AC-6 (Bounce periodic reproduction) and the metering test claims FR-450-AC-7 (500,000-segment metering); neither AC covers quantifier elimination or non-deterministic resets | fsl-replay/src/hybrid_replay/tests/quantifier_tests.rs:74,103,119,135,157,215,236,257 |
| FND-004 | low | `CoupledReset` doc, the TEMPORARY comment and the coupled test cite "Remaining work: QSL-2529"; the tracking ticket is QSL-2639 | fsl-replay/src/hybrid_replay.rs:392-394,512-517, fsl-replay/src/hybrid_replay/tests/quantifier_tests.rs:255-259 |
| FND-005 | low | Elimination work beyond one charge per x-sample and per enablement is uncharged: psc interpolation (bound+1 determinants each), pseudo-remainder Sturm sequences with no content reduction, and root isolation/refinement loops | fsl-replay/src/hybrid_replay/section.rs:85-170, fsl-replay/src/hybrid_replay/poly2.rs:236-319 |

## Evidence

FND-001 probe (reviewer-only, reverted): model `Shift` with `Run -> Gone` reset `v >= 1 and w = v + 1`, `Gone` invariant `v <= 2 and w >= 1`, claim `fair Run::Gone; always holds(h.v >= 1)` over the point lasso `Run` 0..1 with `loop_to` 0. Hand-derived: pre-jump `v` in `[0, 1]` gives `w = v + 1 >= 1`, and post-jump `v` in `[1, 2]` exists, so the transition is enabled at every instant and never taken: unfair, `NotReproduced { at: "", check: Claim }`. Head returns `Ok(NotExact)`; with `activation: self.solution` in `Enabled::quantified`'s Uni env it returns `NotReproduced { at: "", check: Claim }`.

FND-002 probe (reviewer-only, reverted): with `self.meter.charge()?` removed from `Enabled::at` and `meter.charge()?` removed from `Section::exists`, the metering test still passes; Pulse `Strong` consumption is 49 (137 at head).

## Dispositions

Round 1, head 0164e023bb27ec3330a142f6549b621bdea6bf53 (fix commit 0164e023b over merge 47be83434). Reviewer probes ran in a throwaway worktree and were reverted. `cargo-slot cargo test -p fsl-replay --lib hybrid`: 63 passed. `-p fsl-analyze --test it hybrid_requests::`: 8 passed, 13 ignored.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `Enabled::quantified` now uses `activation: self.solution`. The new test `a_reset_reading_a_free_variables_pre_jump_value_is_decided` FAILS with `activation: &[]` restored (panic at quantifier_tests.rs:307) and passes at head. |
| FND-002 | still-open | The rewritten test still passes with both elimination charges deleted (`self.meter.charge()?` in `Enabled::at` and `meter.charge()?` in `Section::exists`). Charges outside the elimination already make the quantified Pulse cost more than its deterministic twin, so `with > without` and the bound-at-twin exhaustion hold without any elimination charge. Pin the elimination's own charges instead. For example, call `Section::exists` on `x * x = t` at `t = 4` with a meter of the hand-counted sample count (1 sample on the first pass, 5 after the roots +-2 are added: 6 charges). Assert that 6 succeeds and 5 exhausts. |
| FND-003 | fixed | Every quantifier test is now tagged `#[trace("TC-3037", "FR-750-AC-12")]`, the only AC that binds FR-450's non-deterministic reset statement. The pure algebra unit tests support AC-12 (b) and do not exercise its Shuttle inputs. That is accepted as their nearest binding. |
| FND-004 | fixed | `CoupledReset` doc, the TEMPORARY comment and the coupled test now cite QSL-2639. |
| FND-005 | still-open | No charge was added. The added module doc is not a charge, and FR-450 :149-157 charges each exact replay work step before the step. The doc is also inaccurate. It says roots "are distinct after squarefree reduction", but section.rs has no squarefree reduction. It also says "the metered total bounds the unmetered inner work". Pseudo-remainder Sturm sequences without content reduction grow coefficient degree with sequence length, while the charged samples grow only with the root count. Either charge per remainder step, per psc evaluation point and per refinement, or file a tracked follow-up and correct the doc. |

Round 2, head a1f0506e5a4e4a33db2b3bb27d7279016e5cb68b (fix commit a1f0506e5). `-p fsl-replay --lib hybrid`: 64 passed. `-p fsl-analyze --test it hybrid_requests::`: 8 passed, 13 ignored. Reviewer mutations ran in a throwaway worktree, one at a time, and were reverted. Deleting the per-point charge in `principal_subresultant` fails `a_tiny_meter_is_exhausted_inside_a_projection`. Deleting the charge in `Section::compare` fails `quantifier_elimination_charges_every_step_before_it_is_taken`, and so does deleting the first charge in `nonroot_from`. Deleting the per-enablement charge in `Enabled::at` fails no test. That charge is now redundant with the charged inner steps, so this is not a finding.

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | `quantifier_elimination_charges_every_step_before_it_is_taken` runs `Section::exists` on `x * x = t` at `t = 4`. It pins exactly 39 units, succeeds at a bound of 39, and is refused at 38 and at 1. `a_tiny_meter_is_exhausted_inside_a_projection` refuses at a bound of 0. Each test fails under the reviewer's charge-deletion mutations listed above. The 39 is an observed pin, not a hand derivation. It is still mutation-sensitive and is accepted as a change detector. |
| FND-005 | fixed | `Section` owns the meter and charges before every pseudo-remainder step, every sequence start, every non-root candidate, every enclosing-interval step, every isolation split, every refinement, every root comparison and every sample. `projection` charges per subresultant evaluation point. An exhaustion met inside `Probe::sign` is stored and returned as the refusal after evaluation, never a panic. The module doc is corrected: it describes unreduced pseudo-remainder sequences and Sturm-Tarski root counting with no squarefree reduction. The remaining uncharged work is the arithmetic on single coefficients inside one charged step. That is acceptable: FR-450 :149-157 meters exact replay work steps, not arithmetic operations, and the univariate exact machinery already follows the same convention. |
