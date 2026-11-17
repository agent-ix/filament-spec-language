---
id: SR-2026111518064800000004
title: "Code and Rust review — QSL-1991 workload weights"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/probabilistic.rs; fsl-semantics/src/check/probabilistic/model.rs; fsl-semantics/tests/it/probabilistic_declarations.rs; FR-185-AC-3; FR-185-AC-10; FR-187-AC-5"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-185", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/FR-187", type: reviews }
---

## Summary

Ticket: QSL-1991. Reviewed PR #2729 (`peter/qsl-1991-workload-weights`) against `origin/main`. The Rust change admits zero workload weights while retaining positive-only distribution weights; the new tests assert recorded zero values, omitted-operation refusal, and unknown-operation refusal. The changed checker path is direct and preserves exact rational parsing and typed refusal behavior. Focused evidence supplied by the dispatching lead: eight workload tests passed, one random-distribution-zero regression passed, and focused fmt/Clippy passed. No aggregate gate was run at the lead's direction; the sole aggregate gate is assigned to the lead after review/dispositions.

## Verdict

**FAIL** — one high finding: the newly accepted zero-weight workloads are not supported by the runtime evaluator, which emits zero-probability steps and fails when the enabled workload weights sum to zero. One low documentation finding remains.

## Scope examined

- `fsl-semantics/src/check/probabilistic.rs:1-20, 80-140`
- `fsl-semantics/src/check/probabilistic/model.rs:125-140, 740-765, 895-915, 1240-1275`
- `fsl-semantics/tests/it/probabilistic_declarations.rs:3216-3310`
- `fsl-eval/src/simulation/model/probabilistic.rs:884-908` and its ignored zero-weight test at `fsl-eval/src/simulation/model/probabilistic/tests.rs:541-555`
- FR-185-AC-3, FR-185-AC-10, and FR-187-AC-5

FR-185-AC-3 and FR-185-AC-10 were examined against the checker and tests. FR-187-AC-5 was examined as the runtime contract required by the accepted zero-weight values. No applicable AssuranceProfile was found. No spec or plan files changed.

## Rust review

The shared exact-weight helper now distinguishes distribution and workload rules at its two call sites: distributions pass `false`, workloads pass `true`. Negative values remain refused in both cases, and zero remains refused for random distributions. The tests exercise the checked graph and assert exact stored rationals; omission and unknown operations still produce their specified named refusals. No new panic, unsafe code, dependency, wire conversion, concurrency, or weakened CI gate is present in the diff.

The cross-crate runtime contract is still unmet. `ModelSystem::weighted_steps` sums every enabled operation's weight and divides each action's weight by that total. It neither removes zero-weight operations when positive alternatives are enabled nor falls back to uniform weights when all enabled weights are zero. The existing FR-187-AC-5 test that expresses these cases is ignored as not implemented. Since this PR makes these workloads check successfully, the evaluator must handle the checked values before the feature is complete.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | high | Accepted zero-weight workloads are not supported by `weighted_steps`: zero-weight enabled operations remain in the result, and an all-zero enabled set yields a zero divisor instead of a uniform choice. | fsl-eval/src/simulation/model/probabilistic.rs:884-908; FR-187-AC-5 |
| FND-002 | low | The checked-workload documentation still promises positive weights after the checker admits zero weights. | fsl-semantics/src/check/probabilistic.rs:10; fsl-semantics/src/check/probabilistic/model.rs:132-133 |

## Verification

- `quoin matrix --repo /home/peter/dev/worktrees/qsl-1991-workload-weights --json` — completed; scoped criterion results are recorded in the separate gap-analysis artifact.
- Aggregate Rust gates — not run in this review pass per lead coordination; focused results are those supplied in the dispatch brief.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `weighted_steps` now drops zero-weight operations when any enabled operation has positive weight and assigns unit weights when the enabled set is all zero. |
| FND-002 | fixed | Both the module-level description and public `Workload` documentation now say non-negative weights. |

After excerpts:

```rust
let has_positive_weight = operations.values().any(|(weight, _)| !weight.is_zero());
let total = operations.values().fold(Rational::from_integer(Integer::zero()), |total, (weight, _)| {
    if has_positive_weight { total.add(weight) } else { total.add(&Rational::from_integer(Integer::one())) }
});
if has_positive_weight && weight.is_zero() { continue; }
let effective_weight = if has_positive_weight { weight.clone() } else { Rational::from_integer(Integer::one()) };
```

```rust
//! and one non-negative weight per operation of it. Each refusal is the
/// A checked workload: one non-negative weight per operation of its model
```
