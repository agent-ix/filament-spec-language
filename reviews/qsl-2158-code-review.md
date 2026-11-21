---
id: SR-202611210022442313144108508
title: "QSL-2158 PR 3285 FR-1043 finite-horizon measure profile code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@986e56a8e157cc8cf694ff4daed7cd2dcdc144e0 (merge-base f3aa681d7ed753a2cdd998bc24842ef38dda3c86); PR 3285 Rust diff: fsl-eval/src/probabilistic.rs, fsl-eval/src/probabilistic/key.rs, fsl-eval/src/simulation/model.rs, fsl-eval/src/simulation/model/encode/bounded.rs, fsl-analyze/src/analyze/exact.rs, fsl-analyze/src/analyze/exact/measure.rs, fsl-analyze/src/analyze/exact/undefined.rs, fsl-analyze/src/exact_probabilistic.rs, fsl-analyze/src/exact_probabilistic/{build,finite_horizon,plan,witness}.rs, fsl-analyze/src/statistical/model.rs, fsl-replay/src/probability_certificate.rs, fsl-replay/src/probability_certificate/document.rs, fsl-replay/src/probability_certificate/document/{measure,measure/bounds,fairness,key,writer}.rs, fsl-replay/src/probability_certificate/witness.rs, fsl-replay/src/probability_certificate/witness/{measure,step,undefined,bounded}.rs, fsl-replay/src/{certificate,certificate/simulation,lib,proof_result}.rs, fsl-replay/src/model_check/{counterexample,replay}.rs, fsl-replay/src/outcome/counterexample.rs, tools/fsl-probability-check/src/main.rs, tests fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs, fsl-analyze/tests/it/exact_probabilistic_item.rs, fsl-analyze/tests/it/state_probabilistic_replay.rs, tools/fsl-probability-check/tests/it/certificate_kinds.rs, Cargo.toml, Cargo.lock"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-1043", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-1042", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-202", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-203", type: references}
---

## Summary

Ticket: QSL-2158, PR 3285. This review covers the implementation at frozen
head `986e56a8e1` against the accepted normative contracts. Spec text is not
re-reviewed. The amended FR-1043, FR-1042, FR-201, FR-202 and FR-203 text was
accepted by SR-202611210022442313144108507 (findings 001 to 013 fixed, plus
FractionValue AC-11 PASS; `/tmp/qsl2158-mean-spec-review/fraction-value/report.md`).
The Rust lane follows `rust-review` and `rust-style` and the repository's
`AGENTS.md`.

The core soundness boundaries hold. The checker rebuilds the complete
eight-member product with the shared `fsl-eval` control. It recomputes each
decided payoff from the key and Bellman-checks every action and post-state.
It rederives ranks, and it rejects at the canonical locus. The vacuity check
explores the full product and returns value-free `AcceptedVacuous` with
`Exhaustive`. Numeric offers for no-activation claims reject `InequalityFails`
at the initial key. Mean refutation applies strict comparisons, the
role-direction `Malformed{kind}` rule and no-fallback hints. Native
FractionValue replay re-executes the actual step and its rewards before any
outside-window read.

The defects found are three behavioral gaps against SHALL clauses, two
duplications of existing codecs, two places where tests do not assert their
AC, and eleven low-severity cleanups.

## Verdict

**CONDITIONAL**. There are no high findings. Medium and low findings must be
fixed in this PR, per the dispatch brief.

Gates: no cargo was run, per the brief. Remote focused acceptance (33 exact
tests listed in `/tmp/qsl2158-fr1043-focused-986e.txt`) is pending on draco
and is not claimed here. The most recent recorded compile (`ec6`) predates
this head. `rustfmt --edition 2021 --check` on every changed `.rs` file at
`986e56a8e1` shows no diffs (read-only). The `model.rs` import reordering fixes
fmt drift that was already present at base. The PR does not change
`.github/workflows`.

Verified clean, with no findings:
- The FR-1043-AC-1 to AC-11 and FR-202-AC-27 to AC-30 behavior paths are
  implemented.
- FR-203-AC-10 vacuity record: `Proved{Exhaustive, Certified}`, no
  `ExactValue`, document `{"mode":"vacuous"}` with empty values. The pinned
  FSpec `Result` schema conformance test is present.
- The P95 `24233/25000` and 2 ms one-path refutation are asserted.
- TC-2065 `quantile 0.4 <= 3` shifted `11/135` is accepted in a separate
  process, and level 0.5 is refuted.
- The removed `#[ignore]` markers are justified, and no tests were deleted or
  weakened.
- FSpec pin `d6d06609` appears only in `Cargo.toml`/`Cargo.lock`.
- No `unsafe` code, `todo!` or `unimplemented!` was introduced. No reachable
  panic was found.
- A quantile over a fraction is refused by S3 (`QuantileOverFraction`), so the
  fraction `activated=false` flag cannot fabricate vacuity.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A partial `lower` mean refutation still enumerates the complete product before checking the named subset. FR-1043 requires expanding only the named states and their successors under the checker's limits. A product that exceeds `states`/`edges` but whose named lower subset fits returns `incomplete` instead of an accepted refutation. | fsl-replay/src/probability_certificate/document/measure/bounds.rs:306 |
| FND-002 | medium | An undefined reward on an operation with an object-typed parameter skips FR-187 admission (`let Some(domains) = domains else { continue }`). It is then flattened to an internal BROKEN expansion fault at runtime, not FR-202 `Undefined` evidence as FR-1043 requires ("an undefined reward ... SHALL settle by FR-202's Undefined evidence"). The coder's preflight-refusal grounding (frames.rs:92-116, symbolic.rs:409-428) covers only operations whose parameters all have finite domains. | fsl-eval/src/simulation/model/probabilistic.rs:567 |
| FND-003 | medium | The FR-1043 rule that workload ratios with both activating and never-activating initial keys follow FR-201's per-key vacuity is not implemented. The checker refuses any non-singleton initial set as `Unsupported`, and the producer returns `not_yet`. No test covers it. | fsl-replay/src/probability_certificate/document/measure.rs:466 |
| FND-004 | medium | The `fsl-eval` key encoder re-states the UnitId descriptor and decimal-integer wire grammar (`["declared",node]` / `["compound",[{unit,exponent}]]`, `unwrap_or_default`) that `fsl-replay` `writer::Text::unit`/`integer` already define. That leaves two owners of one encoding. | fsl-eval/src/probabilistic/key.rs:111 |
| FND-005 | medium | `bounds.rs` hand-implements TransitionIdentity choice decoding and encoding (`TransitionShape`, the `Transition` encoder, ad-hoc `starts_with('{')` checks) instead of reusing the `ModelTransition` canonical codec. The test helper `mean_bound` mirrors that hand form. | fsl-replay/src/probability_certificate/document/measure/bounds.rs:121 |
| FND-006 | medium | The vacuity test asserts only `ProductState(_)` for the numeric no-activation rejection, not the initial complete product key the AC requires. It does not assert the `inconclusive CertificateRejected` settlement. FR-203-AC-11's cancellation, caller-limit and engine-only no-acceptance cases for vacuity are not exercised. | fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs:858 |
| FND-007 | medium | The FR-1043-AC-7 limits test asserts only checker `Err(Limit)` values at zero limits. It never asserts the settled `incomplete` record with the actual limit and no truth value. It does not exercise `scalar_bits`/RationalBits or a limit reached during expansion (`states: 0` stops at the read-time row-count check). | fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs:459 |
| FND-008 | low | The same MeasureError-to-ProbabilityDocumentError mapping is written three times (`scalar_error`, `measure_error`, `scalar_failure`). The ExactProbLimits-to-ProbabilityDocumentLimits literal gains five more copies (7 to 12), and the measure-form `matches!` predicate is repeated across six dispatch sites. | fsl-replay/src/probability_certificate/document/measure.rs:117 |
| FND-009 | low | The checker's enumeration and the witness terminal-stutter replay hand-build zero-reward lists with different filters (subject-model filter versus all declarations, one entry per declaration). EN-5 uses `ModelSystem::zero_rewards()`, the owner of stutter rewards. | fsl-replay/src/probability_certificate/document/measure.rs:633 |
| FND-010 | low | The canonical full-product-action order is re-stated as a local tag table (`""`/`"identity"`/`"post-state"`/`"terminal-stutter"` plus raw bytes) instead of ordering by `ProductAction::to_bytes()`. | fsl-replay/src/probability_certificate/document/measure/bounds.rs:420 |
| FND-011 | low | The lower-rank check does a linear `document.ranking.iter().find` inside the per-value loop. That is quadratic work, uncharged to the caller's work meter, although `ranking` is sorted and `checked_values` uses `binary_search`. | fsl-replay/src/probability_certificate/document/measure/bounds.rs:381 |
| FND-012 | low | `weighted_value` inlines `validate_probability_mass`'s non-negative, unit-mass rule as a second implementation instead of reusing it. | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:1147 |
| FND-013 | low | The public `measure_probability_product_key` path maps structural mismatches (Formula/Deadline control, a missing reward descriptor) to `Error::ReservationDenied{requested:0}`. A malformed caller input is therefore reported as a budget denial. | fsl-eval/src/probabilistic/key.rs:182 |
| FND-014 | low | New unchecked indexing: `state.choices[index]` in exact mean refutation, and `initial[0]`/`initial[1]` in the ratio check. Neither is reachable as a panic, but rust-style prefers `get`/destructuring, as the adjacent code uses. | fsl-replay/src/probability_certificate/document/measure.rs:2306 |
| FND-015 | low | Redundant full enumerations charged to the caller budget. The ratio check enumerates the product a third time only to take `graph.initial`. The producer's vacuity route builds a complete checker document only to clear its ranking in `into_vacuous`. | fsl-replay/src/probability_certificate/document/measure.rs:1829 |
| FND-016 | low | The producer's `Arithmetic::retain` charges storage for every intermediate `mass`/`term`/`sum` cumulatively and never releases it. Producer working storage is overstated relative to the checker, so EN-5 can settle `StorageBytes` incomplete on an item the checker accepts. | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:699 |
| FND-017 | low | The P95 FR-1043-AC-4 test does not assert the AC's "denominator is 1 at the initial key" or "`threshold` 5 in its unit". | fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs:579 |
| FND-018 | low | The public `ProbabilityCertificatePart::Post`/`Reset` variants are added with no producer or consumer in this PR (speculative API). In `fsl-eval/src/probabilistic.rs`, the public `MeasurePlan` derives no standard traits, and `mod tests` sits mid-file before `MeasureComparison`/`MeasurePayoff`. | fsl-replay/src/probability_certificate.rs:595 |

## Coverage

- Methods: `code-review` with its `rust-review` lane folded in, under the
  `rust-style` baseline (API, implementation, safety, wire and testing
  references). The duplication and vendoring check found no vendored files;
  the in-repo duplications are FND-004/005/008/009/010/012. The ceremony
  check found none.
- Checked and clean: identity refused before enumeration. Strict canonical
  decode with depth limit 1. Values/ranking exact coverage. `RankBroken` at the
  first differing key. Shifted range `[-level, 1-level]`. Tight-activation
  search. Ratio `ThresholdMissed`. Vacuity precedence. Mean strict refutation
  and equality. No-fallback hints. Witness-policy parity. Subsystem and paths
  mutually exclusive. Path set refused for a mean. Positive actual draw.
  Residual post-state policy required. FractionValue locus
  (`h+1`, actual post-state, pre-step control/sums). `ReplayParity` on forged
  locus, position, control, sums or cause. Cancellation polls on every loop.
- Not run: cargo build, tests, clippy and deny (per the brief). Runtime
  results are pending in the parent's remote focused batch.
