---
id: SR-967185
title: "code-review of PR 4032 fair checker refined end component tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/exact_probabilistic.rs, fsl-analyze/tests/it/prob_world.rs; PR 4032 head 1471fdd8c6c39264510e34f1e1c50dbd1cd5dc90"
review_set: subset
---

## Summary

Ticket: QSL-3297. Code review with the rust-review lane folded in, of a
test-only PR: a fair-MDP checker test on a fair end component refined out of
an SCC holding a removed state, an FR-1041-AC-6 test, and the
`fair_mec_refinement` fixture. Checker code read: fsl-replay
`document/fairness.rs` (refinement loop, `maximal_mecs`, `fair_sccs`).
Producer read: fsl-analyze `exact_probabilistic/fair.rs`, `unbounded.rs`,
`fair_interval.rs`. Runs (CARGO_TARGET_DIR scratch, `--locked -j 4`): both
added tests pass at the head; two mutants of the checker's strong-fairness
arm were run against them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Refinement test passes when the strong rule keeps the whole SCC `{s,u}` as fair (no refinement): `U={s,u}` also forces 0 at `u` and `s`, and `u` is the first failing key, so the test cannot tell `{u}` from `{s,u}`; only the "strong rejects the component" mutant is killed | fsl-analyze/tests/it/exact_probabilistic.rs:4509-4648 |
| FND-002 | medium | FR-1041 "Bounds and results" requires each refinement round charged to work and run under cancellation; checker refinement loop, `maximal_mecs`, `fair_sccs` and producer `fair_components` / `maximal_end_components_from` neither poll cancel nor charge work; no ticket | fsl-replay/src/probability_certificate/document/fairness.rs:703-886 |
| FND-003 | low | FR-1041-AC-6 tag covers only part of the AC (pre-cancel, limits, unused constraint); "cancellation during refinement" and limits "during refinement or progress checking" are not pinned and cannot be until FND-002 is fixed; the test doc does not say so, so the matrix reads AC-6 as covered | fsl-analyze/tests/it/exact_probabilistic.rs:4650-4655 |
| FND-004 | low | PR names a suspected defect in `fair_interval.rs` (fair components computed on the whole product before targets are dropped, against FR-1041 "Check" U on non-target states) with no ticket, failing test or ruling | fsl-analyze/src/exact_probabilistic/fair_interval.rs:27-39 |
| FND-005 | low | PR body says the zone_certificate 64 vs 65 red has no ticket; it is QSL-3380 (Done). The merge base 7228c02895 is behind main f57863933a, so the merge check should be rerun on current main | PR 4032 body |

## Verdict

Not mergeable until FND-001 is fixed; FND-002 needs a ticket (or a fix in
this PR) before the AC-6 tag stands.

Hand values checked: at `s` actions `a_wait` (to `s`), `z_go` (1/2 `g`, 1/2
`u`), `drift` (to `u`); at `u` only `loop` and `return`; `g` terminal.
SCC `{s,u}`; strong `z_go` is enabled at `s` and leaves the candidate, so `s`
is removed and `{u}` with `loop` is a fair end component. `v(g)=1`, `v(u)=0`,
`v(s)=min(v(s), 1/2, 0)=0`. The all-ones vector satisfies every Bellman
equation, so only `U` rejects it. Values in the test are correct and written
by hand. No tautology: each assertion names a rule and a locus.

Mutants (fairness.rs strong arm): M1 "reject the candidate" (U empty) makes
the first test fail (all-ones accepted), as the coder reported. M2 "ignore
the constraint" (whole SCC fair) leaves the first test passing; it is killed
only by the second test's `Used` claim. A drift-less variant (s: `a_wait`,
`z_go`; u: `loop`, `return`) has fair value `v(s)=1/2`, `v(u)=0`: the
accepted document `(1/2, 0, 1)` kills M2 (rejects it at `s`) and all-ones
kills M1.

Second test: never-enabled `never` (pre `g and f`) is retained (2 fairness
records) and values equal the hand set `{0, 1/2, 1}` for both claims;
pre-cancel, states 1, edges 1, integer bits 0, work budgets sampled below
the bisected minimum (value equals budget) and cancel at sampled charges all
settle `Incomplete` with no document. `Cancel::observing` and
`count_charges` count the same polls, so the sampled points span the run.

Paired code (QSL-2846 rule 1): producer side named and filed as QSL-3487;
its hand-valued test belongs in that fix. Spec search (rule 3): no FR-200,
FR-201, FR-1041 or FR-1042 AC states the refinement-out-of-an-SCC case;
FR-200 "Fair end components" and FR-1041 "Check" state the rule, so the
TC-only tags on the first test are right.

Merge-check reds: the 13 named test failures fail with the same panic
lines and values with the test diff removed (only thread ids differ); the
rover test is the 60 s wall-clock assertion and passed alone. No new failure.
Lint reds map to QSL-3363, QSL-3309, QSL-3385, QSL-3465.

## New findings (disposition pass 1)

Head checked: 970559b18c42df77f3103ffab1cd5845bcf4aacf. Production change
reviewed: `fair_components_within` (fair.rs) and
`decide_fair_reachability_threshold` (fair_interval.rs).

Hand values, derived here. Unit-test product (0 to 1 or 2, half each; 1 to
target 3; 3 to 1; 2 loops): the only end component on `{0,1,2}` is `{2}`, so
`U={2}` and the fair minimum is `[1/2, 1, 0, 1]`; components over the whole
product add `{1,3}`, giving `[0,0,0,1]` once 3 is dropped. Returning-target
model: no action of `s` stays off the target, so `U` is empty and both values
are 1. Refinement model: `U={u}`, `v=(s 0, u 0, g 1)` with `drift`; without it
`s` keeps `1/2`. The extraction leaves the exact route's computation
unchanged; both routes now read FR-200's `U` from one function. No caller of
the interval function exists in the repository outside its unit tests.

Exact route against interval route on `FreeDelays` rows, probed on hand-built
products (temporary lib tests, removed): one row with a cycle through the
target, both give `[1/2, 1, 0]`; two rows, exact gives `[1/4, 1, 0]` and the
interval route `Err(InvalidProbability)`; one row with `fair weak go`, the
interval route gives 1 (correct) and the exact route 0 (wrong); with
`fair strong go` both give 1. Both findings below predate this PR. FR-204
admits no fairness set to the digital MDP, and the digital builder writes an
empty set, so a checked source reaches neither.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | Public `decide_fair_reachability` returns a wrong minimum on a `FreeDelays` product with a weak constraint: the row expansion adds a dispatch state with no action, so the constraint is not enabled at every state of the component and the component is kept. One row, `stay` loops, `go` reaches the target, `fair weak go`: exact 0, hand value 1 (interval route 1) | fsl-analyze/src/exact_probabilistic/fair.rs:195-218 |
| FND-007 | low | Public `decide_fair_reachability_threshold` builds its graph on the unexpanded product, so a `FreeDelays` state with two or more rows is `Err(InvalidProbability)` where the exact route gives `[1/4, 1, 0]`; its doc says components are derived as the exact route derives them | fsl-analyze/src/exact_probabilistic/fair_interval.rs:34-37 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Drift-less variant accepts `(1/2, 0, 1)` on both routes; with the checker's strong arm made to ignore the constraint (temporary, reverted) the test fails there |
| FND-002 | deferred | QSL-3488, by decision of the limits lane owner |
| FND-003 | fixed | The test doc says no stop is pinned to a refinement round and names QSL-3488 |
| FND-004 | fixed | Interval route takes components on non-target states through `fair_components_within`; with the old derivation restored (temporary, reverted) the new unit test fails |
| FND-005 | fixed | PR body names QSL-3380 as Done and the merge check ran on main 44c333a924; main has since moved to 0f6c371f7e with no file shared with this PR |
