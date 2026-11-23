---
id: SR-967175026302197768900474181941935084800
title: "Gap analysis of QSL-2924 cyclic until and always proof profiles"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3571 head bc5c36a2dc484c5ce19da77299aa9b7ed2d739ae; FR-201-AC-57 to FR-201-AC-65, FR-201-AC-75, FR-202-AC-40 against fsl-analyze/tests/it/{cyclic_state_certificate.rs,bounded_probabilistic_replay.rs}, fsl-eval/tests/it/qualified_bounded_monitor.rs, tools/fsl-probability-check/tests/it/{state_predicate.rs,bounded.rs}"
review_set: subset
---

## Summary

Ticket: QSL-2924. A planless gap analysis scoped to the criteria the PR implements. The matrix source is a grep index of `#[trace(...)]` tags at the PR head, a declared fallback: a full `quire matrix` run did not finish within the review on this loaded host. Every in-scope criterion has at least one tagged test, so nothing is untagged.

The semantic review (intent to test to code) ran inline, as the dispatching brief asked. The new production code has an owning requirement throughout (FR-201 cyclic profiles, FR-201-AC-57/AC-75), so there is no reverse gap. There are no stubs.

The gaps are in test fidelity. TC-3180 to TC-3185 prescribe specific fixtures (the s/t/u chain, the s/b/n/c/d chain, the a/b MDP, the wait/go and stay/leave end components) and exact per-row values. The fsl-analyze tests replace all of them with one four-phase model. That model checks target counts and the initial value at equality, but several AC clauses it never reaches.

The expected values the tests do assert are independent. I recomputed them by hand: DTMC until 3/5, always 2/5; MDP until minimum 1/5 and maximum 3/5, always minimum 2/5 and maximum 4/5. No test was run.

## Verdict

**CONDITIONAL**: no untagged criterion and no high finding. Four medium and two low test-fidelity gaps.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-61 end components created by a scheduler choice (wait/go, stay/leave) are never built; the only EC in the fixture is the single-action cycle {1,3}, so MEC pruning of a leaving action and the accepting EC outside the union with positive value are untested | fsl-analyze/tests/it/cyclic_state_certificate.rs:390-416 |
| FND-002 | medium | AC-58/59/60 prescribed fixtures and per-row values are not built: a decided state with a return edge absorbing, the no-row state reached only through the union, the single-policy value rejected by maximum Bellman, and the false-claim producer targets refused as mismatch are all unexercised | fsl-analyze/tests/it/cyclic_state_certificate.rs:229-323 |
| FND-003 | medium | AC-62: row and objective edits assert only non-acceptance, not the existing rule and locus; foreign identity/binding, model-only key, nonreduced rationals, noncanonical bytes and the unsupported claim-form list are untested for these profiles | fsl-analyze/tests/it/cyclic_state_certificate.rs:333-388; tools/fsl-probability-check/tests/it/state_predicate.rs:519-550 |
| FND-004 | medium | AC-63: storage, bytes and work exhaustion assert only !Accepted or is_err(), not the typed cause; there is no exhaustion for each stage; DTMC stages are unobservable (only pre-cancellation); the undefined-predicate case runs only the producer, not the checker | fsl-analyze/tests/it/cyclic_state_certificate.rs:470-629; fsl-replay/src/probability_certificate/document.rs:1249 |
| FND-005 | low | AC-57 tests use the Msg model instead of the engines Coin model and coin-tails.json named by AC-57 and TC-3195; the initial 1/2 rejection asserts the rule but not the initial-key locus | fsl-analyze/tests/it/bounded_probabilistic_replay.rs:959-1035; tools/fsl-probability-check/tests/it/bounded.rs:1339-1392 |
| FND-006 | low | FR-202-AC-40: Msg model instead of Coin; the step-1 counterexample offered unchanged for the at-most claim (not reusable, FR-202-AC-13) is not tested | fsl-analyze/tests/it/bounded_probabilistic_replay.rs:873-918 |

## Coverage

- Matrix source: grep index of trace tags (declared fallback; `quire matrix` did not complete).
- In-scope criteria with a tagged test: 11 of 11 (FR-201-AC-57 to AC-65, FR-201-AC-75, FR-202-AC-40).
- Reverse gaps: none. Stubs: none. Semantic review: performed inline.
- Out of scope, pre-existing on main: fsl-probability-check `bounded_wrong_source_and_nonexact_rows_never_produce_an_accepted_document` is stale since 4ebe69fa62 (QSL-2850).
- Plan completion: not assessed

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | AC-62's no-proof list names clocks, but neither claim-form test builds a timed (clock) subject for these profiles. Fairness, reward, event-predicate, bounded, nested, multiple bindings and initials, and incomplete and unresolved workloads are all covered. | fsl-analyze/tests/it/cyclic_state_certificate.rs:1727-1891 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-3183 (i) wait/go and (ii) stay/leave are built with the spec's rows, ranks and rules (ThresholdMissed at s/ok, ObjectiveMismatch for empty targets, InequalityFails for ok = 0). A second test proves a wait EC is pruned (go keeps value 1/2) and that an accepting stay EC sits outside the union, under the MDP profile. |
| FND-002 | fixed | The AC-58, AC-59 and AC-60 chains are built with the exact targets, per-row values and ranks. Covered: t absorbs despite its return edge; no row for d; single-policy s = 1/4 rejected InequalityFails at s; the false >= 1/2 claim with harmonic targets {n} refused ObjectiveMismatch; ThresholdMissed at the stricter bounds; RankBroken with rank 2 or 0 at s. |
| FND-003 | fixed | The mutation table asserts exact outcomes for identity, model-only key, foreign binding, fairness, profile, event, nonreduced rational, noncanonical bytes, dropped, duplicate and reordered rows, and lower, upper and interval kinds, plus the CLI rule and locus. The new claim-form test is consistent with the AC-62 list: a bounded `always[0,3]` / `eventually[0,3]` under ev proves only under reachability/v1 (never these profiles); nested event-position is Unsupported; an unbounded nested form is refused at the front end (NotStatePredicate); multiple bindings and initials are Unsupported; incomplete and unresolved workloads are front-end refusals. A fully weighted workload chain proving under cyclic-dtmc-state-predicate/v1 is correct: the Admission bullet admits "a workload fully resolved by its authored semantics to one distribution per live state". Residual: clocks (FND-007); profile relabel (code-review FND-004). |
| FND-004 | fixed | The bisection test asserts, for every sampled ceiling below the least accepted value, the limit's own typed cause (bytes, storage, states, edges, scalar bits, work, integer bits). For MDP it asserts the stages each one stops in. The DTMC profile has no observer, but the typed cause is asserted at every sample. The undefined predicate now goes through check_probability_certificate_document and yields UndefinedPredicate for both profiles. |
| FND-005 | still-open | The fsl-analyze AC-57 tests now use a Coin fixture and assert the initial-key locus. The separate-process fsl-probability-check test (bounded.rs:1339-1392), which carries AC-57's "fsl-probability-check accepts" clause, still uses the Msg `link` source. The engines Coin model is not in this repo, so an in-repo Coin source like the new `coin()` fixture is the reachable equivalent. Low; the lead may accept it as is. |
| FND-006 | fixed | Coin fixture used. The FR-202-AC-13 clause is tested: the step-1 counterexample offered unchanged for AtMost is Refused. |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | The process-level AC-57 test now uses a Coin source (coin.native, coin.snapshot.json, coin.semantic-ir.json), with tails and heads set through the snapshot. The universe comes from the snapshot rather than the hard-coded link population. Committing the semantic-IR as a static generated document follows the existing practice: link.semantic-ir.json in the same directory is also committed with no generator or sync check (last touched in #2711). This is not a regression; a regenerate-and-compare check would be a repo-wide improvement, not this PR's. |
| FND-007 | fixed | a_timed_subject_issues_no_proof_in_the_cyclic_state_predicate_profiles builds a dense-clock subject for until and always and asserts exactly Err(Unsupported) from probability_proof_document. |
