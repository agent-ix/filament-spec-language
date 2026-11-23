---
id: SR-967175026302197768900474181941935084785
title: "Code review of QSL-2921 subsystem refutation"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3552 head eaccebc42040d1a21b51e308ac03ef17bf8120dc vs merge-base 9a2555a0; fsl-replay/src/probability_certificate/{document/refutation.rs,document/long_run.rs,witness/subsystem.rs,witness.rs,state_predicate.rs}, fsl-analyze/src/analyze/exact.rs, fsl-analyze/src/analyze/exact/{subsystem.rs,long_run.rs}, fsl-analyze/src/exact_probabilistic/{unbounded.rs,witness.rs,fair.rs}, fsl-analyze/tests/it/exact_probabilistic_item/{subsystem_refutation.rs,cyclic_state_refutation.rs}, fsl-analyze/tests/it/cyclic_reward_certificate.rs, tools/fsl-probability-check/tests/it/cyclic.rs"
review_set: subset
---

## Summary

Ticket: QSL-2921. Code review with the rust-review lane folded in, over the 20-file diff that adds FR-202's checked-subject subsystem refutation for long-run fractions, expected totals and cyclic unwindowed eventually/always/until.

Independence holds. `refutation::check` rebuilds the chain (workload or scheduler-induced), derives and classifies bottom components itself (Kosaraju over its own induced edges), requires value 1 at named union states and 0 at decided states outside the union, applies `x <= F(x)` or `F(y) <= y` with progress ranks, and compares strictly. Equality yields `ThresholdMissed`, which settles `inconclusive`. It binds the base objective by re-deriving it from the checked subject. The checker does not link `fsl-analyze`: `fsl-probability-check` uses `fsl-analyze` only as a dev-dependency. The long-run flip is sound because `long_run::check_graph` checks exact equalities, not one-sided bounds, and non-workload long-run claims stay refused.

Certificate soundness was traced by hand. A reachability lower bound needs both `x <= F(x)` and a positive path to the union for every positive state. The rank rule enforces the path, and every choice-less state in the eventual and state-predicate graphs is decided, so the unchecked terminal-stutter branch is reachable only for reward lower bounds, where the value is +inf by convention. An upper reward bound needs full coverage plus rank progress to the target, which gives properness. The infinite case `x = (θ/q0+1)·Pr(never)` is harmonic and passes the lower check. Missing, randomized, extra and unreachable scheduler entries are refused, and a workload carries no scheduler.

Reviewer runs (fresh): `cargo test -p fsl-analyze --test it --no-run`, then the name-filtered `exact_probabilistic_item::subsystem_refutation`, `exact_probabilistic_item::cyclic_state_refutation` and `zero_reward_still_requires_properness`: 13 passed.

## Verdict

**CONDITIONAL**: no soundness defect found. The findings are resource bounds in the new producer helpers, test-intent and coverage gaps, and minor duplication.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Producer fixed-point helpers `distances`, `reached` and `avoiding_policy` run O(n·(n+e)) loops with no cancel poll or meter charge; `distances` clones the whole vector every round | fsl-analyze/src/analyze/exact/subsystem.rs:319-364; fsl-analyze/src/exact_probabilistic/unbounded.rs:657-711 |
| FND-002 | low | `classified_union` is a third copy of iterative Kosaraju in fsl-replay (cyclic.rs:301, long_run.rs:1139) and leaves the reverse-edge and bottom-check loops uncharged, unlike long_run's per-edge `cx.work` | fsl-replay/src/probability_certificate/document/refutation.rs:700-822 |
| FND-003 | low | The producer ranks a positive state that has no distance as 0 (`unwrap_or(0)`) instead of declining; this emits a certificate that replay rejects RankBroken (inconclusive) rather than not_yet | fsl-analyze/src/analyze/exact/subsystem.rs:453 |
| FND-004 | low | `choice_indices` re-enumerates the whole model graph a second time, with separate storage accounting outside the document meter, after `subject()` already enumerated it | fsl-replay/src/probability_certificate/witness/subsystem.rs:141-210 |
| FND-005 | medium | TC-3160 and TC-3162 do not assert the AC clauses `decisive-counterexample` basis, no infinite ExactEntry or fabricated finite exact value (FR-202-AC-35, FR-203-AC-15), or AC-35's "proof-side certificate still uses the inclusive rule" | fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs:753-799; cyclic_state_refutation.rs:310-352 |
| FND-006 | low | The TC-3159 `equal` case sets the union state to 1/2 and expects InequalityFails (the union-value-1 rule), so AC-34's equality clause is not exercised there; only FSpec-TC-432's `quarter` covers eventually `>=` equality | fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs:585-591 |
| FND-007 | low | The TC-3164 `stay` case is rejected Malformed(values) because unreached `down` is named, not because `{ok}` accepts as AC-38 states; the component classification under `stay` is not exercised | fsl-analyze/tests/it/exact_probabilistic_item/cyclic_state_refutation.rs:504-520 |
| FND-008 | low | The added TC-3159/FR-202-AC-34 tag on fsl-probability-check `target_free_recurrence...` covers a random-line model at θ=3/5, not AC-34's example; the rewritten assertion checks only the analyze-side Refuted and drops the "no native acyclic witness construction" guarantee (no path-search count) | tools/fsl-probability-check/tests/it/cyclic.rs:158-252 |

## Coverage

- Checked: independence (a), spec clauses and scheduler rules (b), certificate soundness (c), tests and traces (d), rust-review (f): no unsafe; no unwrap or panic in non-test code; checked integer conversions; BTreeMap and sorted scheduler for determinism; rational inputs bounded by `scalar_bits`; visits charged through `reward::visit`.
- Changed assertions: `zero_reward_still_requires_properness...` (unsupported to refuted) follows FR-201-AC-36 (improper means infinite) plus FR-202-AC-35. The `cyclic::target_free_recurrence...` change (unsupported to Refuted) follows FR-202-AC-34 and AC-21. Neither change weakens the test, apart from FND-008.
- Pre-existing failure: `cyclic_mdp::strict_rows_boundaries_and_quantitative_source_scope_are_closed` panics in the test helper's compile `unwrap` (tests/it/bounded.rs:79), because the front end refuses thresholds `0`/`1` as QualitativeThreshold (fsl-semantics/src/check/probabilistic.rs:909-927). That happens before any code this PR touches, and neither the front end nor that test differs between the merge-base, the PR head and origin/main. It is pre-existing. Confirmed by reasoning; it was not re-run on main.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The new limit/cancellation test's pre-cancelled call is caught by `replay_probabilistic_witness_impl`'s first `poll_cancel` before the subsystem entry is dispatched, so the entry's own cancellation (final acceptance poll, mid-check) is not exercised; no work/storage exhaustion case; and it is tagged TC-3159/FR-202-AC-34, whose text says nothing about limits or cancellation | fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs:933-982; fsl-replay/src/probability_certificate/witness.rs:572 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `reached`, `distances` (now one predecessor BFS) and `avoidance` charge `step(budget)` per state and edge, which polls cancellation and charges the candidate meter. `avoiding_policy` is now two linear worklists charged through `tick`. Pass 1 gives the same results; pass 2 keeps the BFS levels but may break ties between choices differently (still deterministic and still valid). |
| FND-002 | fixed | One shared `bottom_components` in probability_certificate.rs, used by long_run.rs and refutation.rs, charges once per stack step, assigned node, predecessor edge and successor edge. long_run keeps exactly its old charge points and its Order-only phase observation. |
| FND-003 | fixed | `avoidance` returns `Ok(None)` when a positive reached state has no distance to the union. |
| FND-004 | fixed | `choice_indices` is removed. `refutation::scheduled_choices` walks only the chain the rows induce, under the document meter and storage. It still refuses non-identity, unsorted, missing, extra and unreached-state rows. |
| FND-005 | fixed | TC-3160 asserts the decisive-counterexample basis and truth false, no exact certificate or document, no wire `value`, and the inclusive proof at `>= 1`. TC-3162 asserts the decisive basis, keeps the counterexample, and adds an acyclic control where `searches > 0`. |
| FND-006 | fixed | The case is renamed `halved`. An AC-34 equality case (FR-412's chain at `>= 1/2`) now rejects ThresholdMissed at the initial key. |
| FND-007 | fixed | A certificate naming only `ok` under `stay` rejects RankBroken at `ok`. |
| FND-008 | fixed | The test is retagged FR-202-AC-21 and asserts `path_searches == 0`. `test-support` is enabled only on the fsl-analyze dev-dependency: resolver 2, no Cargo.lock change, not a production dependency. |
