---
id: SR-967175026302197768900474181941935084849
title: "Gap analysis of FSL PR #3824: AC-22 fair cyclic interval replay"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@e8f9db656427ce1e8a2dab5eab45ba46869d0c8e; PR #3824 against origin/main; FR-1042-AC-22, TC-3212 step 7 and tagging rule; tests tagged FR-1042-AC-22"
review_set: subset
---
# Gap analysis of FSL PR #3824

## Summary

Ticket: QSL-3132. The review pass at head `5c305b824d` (Linear comment f67ac4f5) recorded two findings. This file was rebuilt from that comment for disposition pass 1 because the original scratch copy was not available. The `## Findings` rows are that comment's findings unchanged. Plan completion: not assessed.

## Verdict

Disposition pass 1 at `e8f9db656427ce1e8a2dab5eab45ba46869d0c8e`: FND-001 and FND-002 are fixed. FR-1042-AC-22 is still not fully evidenced. Two of its tagged tests from main fail at this head (code-review SR FND-003). The analyzer never emits the `cyclic-mdp-fair-eventual/v1` profile that the AC names, and no test asserts that profile (code-review SR FND-004). New low trace finding FND-003. Not mergeable at this head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-3212's AC-22 mutation negative is not sent through the independent checker process. | tools/fsl-probability-check/tests/it/fair_cyclic.rs:13-33 |
| FND-002 | low | New AC-22 evidence is not tagged to TC-3212 as the test case requires. | tools/fsl-probability-check/tests/it/fair_cyclic.rs:13-15 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | `fair_one_shot_go_has_exactly_one_half_reachability` is tagged `FR-1042-AC-22` but asserts none of its clauses: no low rational budget, no `ValueBounds`, no documents, no checker. It calls the exact `decide_fair_reachability` on a separate `one_shot` model with default limits. That is FR-200 fair reachability evidence. Retag it to the FR-200 criterion it asserts, and drop FR-1042-AC-22. | fsl-analyze/tests/it/exact_probabilistic.rs:2626-2652 |

## Dispositions

Round 1 at `e8f9db656427ce1e8a2dab5eab45ba46869d0c8e`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 09f1b3c72a adds the fresh-process mutation check: each document's undecided value is replaced, and `fsl-probability-check` exits 1 with `inequality-fails` (lower) or `rank-broken` (upper) at that product key. |
| FND-002 | fixed | 09f1b3c72a adds `TC-3212` to the CLI test and to both TC-3212 step-7 analyzer tests. |
