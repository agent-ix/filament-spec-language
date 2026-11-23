---
id: SR-3608001
title: "code-review of PR 3608 cancelled branches and predicate joins in replay enabledness"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/protocol_analysis/recording.rs, fsl-analyze/tests/it/protocol_replay_mode.rs at 115a613f78ca072982125b5cb8e38c801c453d70"
review_set: subset
---

## Summary

Ticket: QSL-2998. Rust review folded in. Titan focused runs: `--test it` filter 93 passed, 0 failed, 0 ignored; `--lib a_refused_prefix`
2 passed. Probes ran as a scratch test in the coder's titan worktree and were reverted (tree clean after).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Rule 20 equates "the other branches finished causally before the step" with "after the cancel", but the cancel happens at the join step, which ST-7 enables and does not force: under `join any ... outstanding cancel` the closed recording `L`, `R`, `Placed` (Buyer order; `R` steps after `L` finished but before the join fires) gives `R` `violated`, `unexpected` (probe P1; same with `predicate(left)`). A legitimate interleaving is refuted. Only a step that needs the join (a step after the `parallel`) causally before the branch step proves the cancel came first | fsl-analyze/src/protocol_analysis/recording.rs:4168-4195 |
| FND-002 | medium | Predicate tables are bounded by silently truncating: `bits = joined.len().min(12)` while `join_reached` builds its mask over every joined label, so 13 or more labels give a mask past the table (or the wrong arity to the function) and the join is `Unevaluable` forever with no refusal; at 64 or more labels `usize::from(finished) << bit` overflows (panic in debug). Up to 4096 evaluations per predicate join, each on a fresh `recording_meter()`, are spent before any limit is charged, and the tables are rebuilt for every prefix of an incremental assessment | fsl-analyze/src/protocol_analysis/recording.rs:4146-4156 |
| FND-003 | medium | On the package route `BarePackage::evaluate_join_predicate` returns `SOURCE_FORM`, so every predicate join is `Unevaluable` and its step stays `pending` whatever the closure: a closed, conforming recording can never settle `proved` and reports `inconclusive`/`open-history`, a cause that names the recording, not the engine's missing capability. No test covers this path | fsl-analyze/src/protocol_analysis/recording.rs:4202-4240 |

## Verdict

Not mergeable (FND-001). The predicate evaluation follows FR-218: labels sorted, Boolean `true` for a completed branch, and cancelled
branches never read; the policies `all`, `any` and `quorum(n)` are counted over branches finished causally before the step, which is
stable across prefixes, and the tests assert prefix sequences with `assert_every_verdict_only_advances` and last prefix equal to the
item. Nested parallels work through the ancestry windows. The cancel rule is wrong for steps taken after a branch finished but before
the join step (FND-001), and the predicate table has no honest bound or package-route outcome (FND-002, FND-003).

## New findings (disposition pass 1)

Head 8d9aeec6e325c6d7ae74064d426eb0d9f859ff3e (code equal to d09580b4 in the coder's titan worktree). Titan: `--test it` filter
99 passed, 0 failed, 0 ignored; `--lib a_refused_prefix` 2 passed. Probes (scratch test, reverted): P1 `L`, `R`, `Placed` under
cancel gives `R` satisfied; `L`, `Placed`, `R` gives `R` violated; probe R below.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | The cancel rule is not occurrence-matched in a `repeat`: with a `join any ... outstanding cancel` parallel inside the loop body, the legitimate recording of two iterations (`L`, `R`, `Placed`, `Got`, `yes`, then `L`, `R`, `Placed`, `Got`, `yes`) gives the second iteration's `L` and `R` `violated`, `unexpected`, because the first iteration's `Placed` (a step after the `parallel`) causally precedes them (probe R). Only a post-join step of the same iteration proves the cancel | fsl-analyze/src/protocol_analysis/recording.rs:4249-4295 |
| FND-005 | medium | Rule 25's refusal (a predicate that does not evaluate to a Boolean) is unreachable for an admitted function: FR-218 and FSpec FR-146 make it total and pure, and `recording_meter()` sets every limit to `u64::MAX`, so the only way in is an evaluator `InternalFault`, which this maps to `unsupported_construct`/`not-yet-implemented` instead of a runtime-invariant refusal. Delete rule 25 and route an internal fault to the existing internal-fault refusal | fsl-analyze/src/protocol_analysis/recording.rs:4340-4366 |
| FND-006 | low | The label bound refusal uses `unsupported_construct`/`not-yet-implemented`, which reads as an unbuilt feature; a fixed engine bound has the `stage_limit_exceeded` family (`node-count-exceeded`, `edge-count-exceeded`) or at least a cause naming the bound. The `predicate-join-label-bound` fact disambiguates, so the wire is usable | fsl-analyze/src/protocol_analysis/recording.rs:4322-4330 |
| FND-007 | low | The package-route arm of `predicate_join_tables` is unreachable today: the package route refuses any protocol with a `parallel` at selection, and AC-36's test exercises that selection refusal, not the arm. Keep it only with a comment saying it guards the day the package route admits `parallel`, or delete it | fsl-analyze/src/protocol_analysis/recording.rs:4332-4339 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_cancel needs a post-parallel step or finish causally before the branch step; probe P1 L, R, Placed gives R satisfied (AC-32); repeat case is FND-004 |
| FND-002 | fixed | at most 10 labels, refused before any evaluation; mask takes at most 10 bits; tables built once per read and shared by every prefix |
| FND-003 | fixed | package route refuses unsupported_construct/not-yet-implemented (AC-36), never leaves steps pending |

## New findings (disposition pass 2)

Head c513f5ba2b32c6b8ad02a722b6c79589323b4a91 (code equal to 79f1ea77 in the coder's titan worktree). Titan: `--test it` filter
101 passed, 0 failed, 0 ignored; `--lib a_refused_prefix` 2 passed. Probe N (scratch test, reverted, tree clean after).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | Nested repeats violate rule 26 ("the same iteration of every `repeat` the `parallel` runs in"): only the innermost `repeat` is matched by iteration, and steps after the inner `repeat` in the outer body go to the any-iteration set. With `repeat Outer { repeat Inner { parallel ... join any outstanding cancel } exhausted {}; event Q }`, the legitimate recording `L`, `R`, `L`, `R`, `Q`, `L`, `R`, `L`, `R`, `Q` gives every `L` and `R` of the second outer iteration `violated`, `unexpected`, because the first outer iteration's `Q` precedes them (probe N; prefixes monotone, last equals batch) | fsl-analyze/src/protocol_analysis/recording.rs:4335-4360 |
| FND-009 | low | `predicate_join_tables` now `continue`s on the package route, relying on the package route having no `parallel`; if it ever admits one, `join_reached` finds no table and answers `false` for every subset, so the join is silently never reached. A `debug_assert!` or the selection refusal's invariant stated at the call keeps that loud | fsl-analyze/src/protocol_analysis/recording.rs:4441-4443 |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | a step after the parallel inside the repeat body counts only in the same iteration (iteration_index, carried through each case); AC-37 and AC-38 tests; probe R shape gives every L and R satisfied. Nested repeats are FND-008 |
| FND-005 | fixed | rule 25 deleted; an evaluator fault refuses runtime_invariant/established-invariant-broken (FailedWithDetail naming the join), the existing internal-fault family |
| FND-006 | fixed | label bound refused stage_limit_exceeded/work-budget-exceeded with fact predicate-join-label-bound, checked before any evaluation |
| FND-007 | fixed | package-route arm deleted; rule 25 states the selection refusal that makes it unreachable; residual is FND-009 |

## Dispositions (fix round 3)

| Finding | Disposition |
| --- | --- |
| Probe N (nested repeats) | Fixed. `after_cancel` matches a step after the `parallel` per repeat: for every `repeat` whose body holds both the `parallel` and the step, both iterations must be determined and equal (`repeat_iteration`). With no repeat between, the iteration is the causal rank of the node taken through the cases; with an inner repeat between, it is the most times a step before the inner repeat in the body was observed before the step; where none exists the iteration is not determined and the step is not counted (a missed violation, never a false one), so Probe N stays satisfied. Test `a_cancel_in_nested_repeats_cancels_per_iteration_of_each` (FR-895-AC-39): with a step `P`, the two-iteration recording is satisfied in batch and every prefix and the last `R` after its own iteration's `Placed` is violated; without `P`, Probe N is satisfied; the outer body holds a `choice` (`Decide`) whose case is one of the matched steps. |
| Package skip | The skip in `predicate_join_tables` states its invariant (the package route refuses the selection of a protocol with a `parallel`, rule 25). |
| PR body | Refreshed to rules 19 to 32. |
