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

## New findings (disposition pass 3, final)

Head 16961b0ca6dbaf413ef426d9f87b5416e1fe2659; `git diff ba197d79 16961b0c` touches FR-895 only (rules 28/29 wording). Titan, coder's
worktree at 16961b0c, clean: `--test it` 6-filter set 102 passed, 0 failed; `--lib a_refused_prefix` 2 passed. Probes ran as a scratch
test and were reverted (tree clean after). An unrelated process briefly injected `false &&` into `repeat_iteration` in that worktree
(07:13:56 to 07:14:43); the 102 run ended 07:10:29 and every probe below ran on clean source. Every probe: prefixes monotone for the
cancel verdicts, last prefix equals batch. Each false verdict below is `satisfied` with the same recording under `outstanding continue`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | high | Rule 28's rank is unsound when a cancelled branch skips an iteration, which is exactly what `cancel` allows. Single `repeat { parallel any cancel {L}{R}; Placed; Got; Decide }`, recording `L`, `Placed`, `Got`, `yes`, `L`, `R`, `Placed`, `Got`, `yes` (R cancelled in iteration 1): the iteration-2 `R` has rank 1, matches the first `Placed`, and is `violated`, `unexpected` (probe S). Present since round 1's `iteration_index` | fsl-analyze/src/protocol_analysis/recording.rs:4284-4311 |
| FND-011 | high | Rule 29's "most times any step before the inner `repeat`" over-counts when a preceding step is itself inside a `repeat` (or another inner `repeat`'s subtree), and that breaks the after-step's side. Outer body `P; repeat I1 { parallel any cancel {L}{R} }; repeat I2 { X }; Placed ...`, recording `P`, `L`, `R`, `L`, `R`, `X`, `Placed`, `Got`, `yes`, `P`, `L`, `R`, `X`, `Placed`, `Got`, `yes`: `X` of iteration 1 counts `L` twice, gets iteration 2, equals the iteration-2 `L` (`P` counted twice) and precedes it, so the iteration-2 `L` and `R` are `violated` (probe T) | fsl-analyze/src/protocol_analysis/recording.rs:4312-4342 |
| FND-012 | high | Rule 29 under-counts when the steps before the inner `repeat` lie in different `case`s across iterations, counting each case's steps separately. Outer body `choice Pick { case a {A} case b {B} }; repeat Inner { parallel any cancel {L}{R} }; Placed ...`, recording `a`, `A`, `L`, `R`, `Placed`, `Got`, `yes`, `b`, `B`, `L`, `R`, `Placed`, `Got`, `yes`: the iteration-2 `L` and `R` get iteration 1, match the first `Placed`, and are `violated` (probe C3) | fsl-analyze/src/protocol_analysis/recording.rs:4312-4342 |

Sound in the probes: three-deep nesting `repeat { P; repeat Mid { Q; repeat Inner { parallel }; M2 }; Placed ... }` legit (all satisfied,
probe D), the `R` after its own `M2` violated and nothing else (D2), a skipped `R` in a Mid iteration with the step before determined (D3);
AC-39's three recordings.

Out of this diff, seen in the probes (not counted against the PR): (1) with `event P; choice Pick {...}` before an inner `repeat` and
different cases per outer iteration, the iteration-1 `L` and `R` go `satisfied` then `violated`, `unexpected` when the iteration-2 case
arrives, under `continue` too: an enabledness verdict reversal across prefixes (`assert_every_verdict_only_advances` fails; probes C2,
C2cont, C3cont). (2) Nested `repeat`s with two outer iterations report the `repeat` nodes `violated`, `duplicated` (probes D, T).

Direction for FND-010 to 012: take a `repeat`'s iteration of a step only from a step that runs exactly once in every iteration (a step on
the body's spine: not under a `choice` case, a `parallel` branch or another `repeat`), counted causally at or before the step, and treat
every other case as undetermined. That subsumes rules 28 and 29 and keeps rule 30's "missed, never false" claim true.

## Dispositions (round 3)

| FND | outcome | reason |
| --- | --- | --- |
| FND-008 | fixed | every enclosing repeat whose body holds both the parallel and the step is matched (repeat_iteration); probe N shape all satisfied (AC-39); three-deep nesting probe D sound. Residual soundness gaps in repeat_iteration are FND-010 to FND-012 |
| FND-009 | fixed | the invariant is stated at the continue (package route refuses selection of a protocol with a parallel, rule 25); acceptable for a low |

## New findings (disposition pass 4, final)

Head e2c54e6b4c369708c08bc1da650bee5f25ec44b9; `git diff origin/main...head` holds no `dbg`, `eprintln!` or `println!`. Titan, coder's
worktree at e2c54e6b, clean: `--test it` 6-filter set 104 passed, 0 failed; `--lib a_refused_prefix` 2 passed. Probes ran as a scratch test,
each recording under `outstanding cancel` and `outstanding continue`, and were reverted (tree clean after). No legitimate recording gives a
cancel-only violation: two inner repeats with the first run twice (T), choice before the inner repeat with and without `P` (C3, C2),
`P`; choice; parallel in a single repeat with cases a then b (C5), a choice after the inner repeat (C6), only once-per-iteration steps after
the parallel (O), skipped branch (S), three-deep nesting legit, skipped inner branch, and Mid run zero times then twice (D, D3, D4). Each
violating shape gives exactly the late `R` `violated` and nothing else (Tv, C5v, C6v, D2). With no once-per-iteration step before the
parallel the late `R` is missed, not false (Ov), as rule 31 states. Cancel verdicts monotone; last prefix equals batch in every probe.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | low | `body.children.iter().scan(false, ..)` takes the children through `on_path` inclusive and silently takes them all if `on_path` is absent; `let end = body.children.iter().position(|child| *child == on_path.node)?;` then `body.children[..=end]` states the intent and fails closed | fsl-analyze/src/protocol_analysis/recording.rs:4288-4296 |

Out of this diff (QSL-3009): the choice-with-different-cases reversal also shows in a single-level `repeat` (C5) and with the choice after
the inner `repeat` (C6), on an obligation other than the cancel nodes; under `continue` too.

## Dispositions (round 4)

| FND | outcome | reason |
| --- | --- | --- |
| FND-010 | fixed | iteration from once-per-iteration main-sequence steps only; skipped branch no longer shifts it (AC-40; probe S) |
| FND-011 | fixed | steps in an inner repeat never count (rule 30; AC-41; probe T with I1 twice, Tv) |
| FND-012 | fixed | steps under a case never count; probes C2, C3, C5, C6 give no cancel-only violation; no test (SR-3608002 FND-004) |
