---
id: SR-2026111919340478491681419
title: "code-review of QSL-2529 activation-state exact replay and QSL-2604 flow order"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2529, QSL-2604; PR3155 head 5fa6e0026eb527cfe3290746113dc951136bb73e; git diff origin/main...HEAD; fsl-replay/src/hybrid_replay.rs; fsl-replay/src/hybrid_replay/{class.rs,eval.rs,exact.rs,exact/activation.rs,expand.rs,signal.rs,tests.rs,tests/activation_tests.rs}; fsl-analyze/tests/it/{hybrid_provider.rs,hybrid_requests.rs}"
review_set: subset
---

## Summary

Ticket: QSL-2529 (activation-state half) and QSL-2604, PR filament-spec-language#3155, frozen head 5fa6e0026eb527cfe3290746113dc951136bb73e. This is a code-review with the rust-review lane folded in. The change removes `ExactGap::ActivationState`. It decides `on each` claims under `model-time` that read activation state, treating tau as a second real variable (new `exact/activation.rs`, which uses the poly2 projection from #3119). It cuts at the roots of the projection by a lazy fixpoint, decides `1 + ceil(R/P)` periods by past reach, and reports an open infimum. It adds the gap `ExactGap::ActivationInfimum`. It also corrects the flow-order doc and adds a test (QSL-2604).

Reviewer probes, in the review worktree only, removed before the end and never pushed:
- Probe 1 (nested windows): a model `Ramp` with `v' = 1; w' = 0` and a finite trace with `end` at time 10. The claim is `on each (q) when (q.v > 0 and q.v < 1) { capture v0 = q.v; eventually [0, 5] always [0, 1] holds(h.v >= 2 and h.v <= 2 + 4 * v0) }`. The result is `NotReproduced { Claim }`: no projection root falls in (0, 1), so the one sample is 1/2, where the obligation holds. The same claim with guard `q.v < 1/8` gives `Reproduced(Formula)`. That control shows the obligations for tau in (0, 1/8) are false, so the first answer is wrong.
- Probe 2 (origin capture): `on origin { capture v0: Rational[0, 2; 1, 2] = h.v; eventually [0, 1] holds(h.v >= v0 + 2) }` over a moving Shuttle dwell returns `Err(Exact(IllKindedTerm))`.
- Probe 3 (mutation): I deleted all six explicit `meter.charge()?` calls in activation.rs. All six activation tests still pass.
- Probe 4 (mutation): I set `periods = zero()` in `decided_until`. Four activation tests fail, so the period count is pinned.

Gates: `-p fsl-replay --lib hybrid` 70 passed. `-p fsl-analyze --test it hybrid_requests::` 9 passed, 12 ignored, and every ignore names QSL-2531 or QSL-2532. `hybrid_provider::flow_equations` 1 passed. `-p fsl-hybrid-reference --lib` 3 passed. Clippy (`-p fsl-replay -p fsl-analyze -p fsl-hybrid-reference --all-targets`, cap-lints warn) reports nothing in the diff; the one hit at hybrid_requests.rs:906 is pre-existing (e008f7baa7). The diff adds no `#[allow]`, no `unsafe` and no non-test panic surface.

## Verdict

NOT MERGEABLE as is. Two findings are blocking.

FND-001: the tau decomposition only cuts where an atom's root curve meets a line `t = tau + c`. Nested windows over atoms that depend on tau also change truth where the distance between two roots crosses a window width. One rational sample per open cell then misses false obligations, so replay returns `NotReproduced` for a refuting trace. That is a wrong exact answer: no gap is raised and no ticket tracks it.

FND-002: `rational_of` failing now raises `IllKindedTerm`, which settles `failed` as a broken invariant. Before this change it raised `ActivationState`, which settled `NotExact`. An `on origin` capture over a moving dwell hits this path, so a well-checked claim now settles `failed`.

What is right:
- QSL-2604 is correct. FR-447 orders variables by first appearance as an equation's left side, and FR-447-AC-2 requires every mode to write every variable. First appearance is therefore mode 0's written order, which is the variable declaration order FR-1019 cites. The inversion the brief worried about, a variable first appearing in a later mode, cannot occur.
- The `1 + ceil(R/P)` period count is correct and pinned. I recomputed it: `Past` has R = 1, so tau runs to 2; `Twice` has R = 2, so tau runs to 1 + 3 = 4.
- The AC-12(a) infimum is correct. The capture `tau + 1/2` is out of `[0, 1]` exactly on (1/2, 1), the open cell's start gives 1/2 with `open`, and a point record wins over the open one.
- The unevaluated-irrational-section rule is conservative. It raises `ActivationInfimum` and never answers wrongly.
- The lazy-CAD branch concern does not hold. A `Conditional`'s condition polynomial is read and projected, so the taken branch is constant across a tau-cell. Its x-sign structure is delineable, because coefficients, discriminants and the segment-end lines are all in the projection set. I could not build a counterexample.
- The lookup misses in `env.activation` and `env.captures` that now return `IllKindedTerm` cannot be reached by construction, so reusing that error there is acceptable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The tau decomposition projects only atom polynomials, the lines `x - tau - c` for window offsets and the segment-end lines. Nested windows over tau-dependent atoms change truth where two roots are a window width apart (`r_j(tau) - r_i(tau) = c`), and no resultant of `P(tau, x)` with `P(tau, x + c)` is taken. One sample per open cell then hides false obligations. Probe 1 returns `NotReproduced` for a refuted claim. The answer is wrong, not `NotExact`, and no ticket tracks it. | fsl-replay/src/hybrid_replay/exact/activation.rs:129-175, fsl-replay/src/hybrid_replay/exact/activation.rs:259-263, fsl-replay/src/hybrid_replay/exact/activation.rs:444-493 |
| FND-002 | high | `rational_of(&value).ok_or(ExactGap::IllKindedTerm)` in `Claim::captures` turns a capture whose value is a non-constant function of t (an `on origin` capture read on the first dwell) into `failed`/`hybrid-term-kind`. On main it raised `ActivationState` and settled `NotExact`. Probe 2 reproduces it. | fsl-replay/src/hybrid_replay/exact.rs:885-888 |
| FND-003 | medium | `the_activation_scan_is_charged_to_the_meter` cannot detect a deleted activation charge. With all six explicit `meter.charge()?` calls in activation.rs removed, the test still passes (Probe 3), because it measures `spent` from the same code and the other charges keep the count above 100. | fsl-replay/src/hybrid_replay/tests/activation_tests.rs:184-204 |
| FND-004 | medium | The metering test is tagged `FR-450-AC-7` (Bounce, 500,000 segments). That AC does not cover the activation scan. This repeats #3119 FND-003. | fsl-replay/src/hybrid_replay/tests/activation_tests.rs:186 |
| FND-005 | medium | `flow_equations_are_walked_in_variable_declaration_order` is tagged `FR-1019-AC-14`, which is the C1 to C6 class fixtures. The flow-order criterion is FR-1019-AC-15, added by #3143, whose TC-2869 step 3c this test does not follow (one mode, two declaration orders). | fsl-analyze/tests/it/hybrid_provider.rs:539-543 |
| FND-006 | low | The `ActivationInfimum` doc and the TEMPORARY comment in `exact_replay` cite "Remaining work: QSL-2529". The gap's tracking ticket is QSL-2690, a sub-issue. The PR body says the same. | fsl-replay/src/hybrid_replay.rs:389-395, fsl-replay/src/hybrid_replay.rs:515-520 |
| FND-007 | low | `each_time` raises `ActivationInfimum` for any non-activation and non-capture record in an open tau-cell, including an atom undefined at a fixed t that does not move with tau. The variant doc claims only tau-moving instants. The behaviour is conservative, but the doc understates the gap. | fsl-replay/src/hybrid_replay/exact/activation.rs:529-541, fsl-replay/src/hybrid_replay.rs:389-395 |
| FND-008 | low | Some scan work is uncharged: `roots_in` for every projected polynomial at every sample, and the re-partition on every fixpoint pass. `formula_signal`'s per-cell work is charged as one batch per segment before the call. | fsl-replay/src/hybrid_replay/exact/activation.rs:419-428, fsl-replay/src/hybrid_replay/exact/activation.rs:454-481 |
| FND-009 | low | `cargo fmt --check` flags the widened `pub(super) fn partition` line (104 chars). The PR body says rustfmt is clean. | fsl-replay/src/hybrid_replay/signal.rs:102 |
