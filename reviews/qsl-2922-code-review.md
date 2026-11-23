---
id: SR-967175026302197768900474181941935084768
title: "Code review of PR3544 per-model-state expansion cache in the exact measure enumerator"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3544 55779853a92c576237e176f10f6e227028021518; fsl-replay/src/probability_certificate/document/measure.rs, fsl-analyze/tests/it/exact_probabilistic_item.rs, fsl-analyze/tests/it/exact_probabilistic_item/fraction_window.rs"
review_set: subset
---

## Summary

Ticket: QSL-2922. Code review with its Rust lane (rust-review) of `git diff origin/main...HEAD`. The change caches,
per call of `enumerate`, each queued model state's canonical key bytes (`raws`) and each model state's
expansion together with its successors' key bytes (`expansions`), so a model state that stands under many
(remaining, count) product states is expanded and encoded once. Two integration tests pin the window-10
fraction mean at `10999/11000` and the exact work boundary 265,325.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Expansion cache is keyed on canonical model bytes alone, but expansion depends on provenance (Anchored vs RestoredFromKey): on the checker's named path a restored state and an anchored state with the same bytes share one cached expansion, so the outcome depends on BFS order; existing fsl-eval caches key on (bytes, anchor) | fsl-replay/src/probability_certificate/document/measure.rs:840-1007 |
| FND-002 | medium | Both new tests are tagged FR-203-AC-1, whose criterion (§15.2 P95 at 5 ms, §15.1 ValueBounds, §15.4 Deliver refuted) neither test exercises; the tags inflate AC-1 coverage | fsl-analyze/tests/it/exact_probabilistic_item/fraction_window.rs:30,49 |
| FND-003 | low | Post-state keys are now encoded eagerly for every successor before the per-edge `edges` limit check and post-state predicate read, so under tight budgets the first reported limit or refusal can differ from origin/main; no test pins the new precedence | fsl-replay/src/probability_certificate/document/measure.rs:983-990 |
| FND-004 | low | The one-short boundary run asserts only `Incomplete(LimitReached { .. })`; it does not pin `EvaluationMeter { counter: WorkUnits }` and value 265,324, so a stop on any other limit would pass | fsl-analyze/tests/it/exact_probabilistic_item/fraction_window.rs:58-65 |
| FND-005 | low | The cache keeps every expansion (successor ModelSimStates) for the whole enumeration and deep-clones the full distribution vector on every hit; the storage charge covers only `size_of::<Successor>()` per successor plus 3x the raw key, not the retained post-state heap | fsl-replay/src/probability_certificate/document/measure.rs:849-1006 |

## Verdict

CONDITIONAL. The fix is correct for the reported growth and keeps producer and checker independent. Each
`enumerate` call builds its own `raws` and `expansions` maps as locals. `measure_probability_certificate_document`
(producer, measure.rs:1809) and every checker call (measure.rs:2140, 2199, 2355, 2663) build their own `Meter`
and storage counters, and the document carries no expansion data. The checker therefore rebuilds and re-charges
its own product from the system and trusts nothing from the producer's cache.

FR-120's cumulative work model charges each expansion actually performed and forbids replenishing the allowance.
It does not require a repeated expansion of the same model state to be charged again, so a skipped re-expansion
is allowed. Both maps are `BTreeMap` and queue order is unchanged, so charge totals are deterministic.

The re-indent hides no logic change. `git diff -w` shows only the cache wrapper, the `raw_of`/`product_key`
split, the `Option<Vec<u8>>` post key on `Successor::Position` and the eager post-key loop (FND-003).

The new tests fail on origin/main. With measure.rs temporarily reverted, the 1,000,000 run settles
`Incomplete(LimitReached{EvaluationMeter{WorkUnits}, 1000000})` and the boundary test fails at its proved
assertion. The file was then restored. On the head, the 61 `exact_probabilistic_item::` tests pass, including the
named partial-lower and limit-precedence tests. rustfmt is clean on the touched files, and the build shows no new
warnings from them.

FND-001 and FND-002 need fixing before merge; FND-003 to FND-005 can be fixed in the same round.

## Scope and evidence

- measure.rs `enumerate` (whole function, head and origin/main), `measure_probability_product_key`, `restore_named`, producer and checker call sites of `enumerate`: examined.
- fsl-eval `ModelSimState` provenance, `required_anchor`, `contracts::invariants`, `walk::expand_enabled_*`, `actions_with_budget`, the anchor-keyed caches at fsl-eval/src/simulation/model.rs:3667/3693 and application.rs:611: context_only.
- FR-120 "Cumulative probabilistic work" and generator invariant; FR-203 Description, FR-203-AC-1, FR-203-AC-13; FR-1043-AC-5; TC-638 scope: examined for the trace and work-accounting checks.
- Gates run: `cargo test -p fsl-analyze --test it --no-run`; `cargo test -p fsl-analyze --test it exact_probabilistic_item::` (61 passed); the same `fraction_window` filter against origin/main's measure.rs (2 failed, as expected); `rustfmt --check` on the two touched Rust files. Clippy and the workspace suite were not run (filtered-test rule).
