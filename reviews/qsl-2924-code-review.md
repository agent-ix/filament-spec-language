---
id: SR-967175026302197768900474181941935084799
title: "Code review of QSL-2924 cyclic until and always proof profiles"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3571 head bc5c36a2dc484c5ce19da77299aa9b7ed2d739ae; fsl-replay/src/probability_certificate/{cyclic_state.rs,cyclic.rs,document.rs,document/mdp.rs,document/writer.rs,state_predicate.rs}, fsl-eval/src/automaton/{bounded.rs,qualified_bounded.rs}"
review_set: subset
---

## Summary

Ticket: QSL-2924. Code review with the rust-review lane folded in, against spec PR 3549 (FR-201-AC-57, FR-201-AC-75, FR-202-AC-40) and spec PR 3551 (FR-201 "Closed cyclic until and always document profiles", FR-201-AC-58 to AC-65). Diff: 15 files, +1858/-44, merges cleanly on main d52879e6. No cargo command was run. Every conclusion comes from reading the code at the PR head and origin/main.

Checker independence: verified. `cyclic_state::certificate_graph` rebuilds the original graph with `enumerate_graph_capturing` (target None, stop_at_target false, full keys), checks positive edges and unit mass for each choice, and refuses DTMC states with more than one choice. It reads predicates through `state_predicate::outcome` with the claim's own occurrence and binding. It derives maximal end components itself (prune-and-split over Tarjan) and builds the union and certificate graph from those. No producer target, component or classification enters. Both checkers compare `targets`, `event` and `optimization` with the derived values before any row check (document.rs:1882-1889, mdp.rs:1090-1096). The threshold is inclusive `v <= 1 - θ` / `v <= θ`, through `violated_bound` and `check_threshold`. Thresholds 0 and 1, fairness, clocks, non-infinite-trace profiles and MDP workloads are refused Unsupported. The DTMC and MDP union rule (`undecided_accepts == violating`) matches FR-202's classification: an undecided component rejects until and accepts always.

Coder decisions:

- (a) Claim-oriented acceptance value and side. The spec states only the threshold, not the reported value. Reporting the claim's quantity matches the acyclic state-predicate profile, which reports the direct event value. Acceptable, but see FND-001.
- (b) An MDP `workload` document is now read and then refused ObjectiveMismatch, because the derived objective is `maximum` (AC-60). A workload claim is still refused Unsupported by `certificate_graph`. Verified.
- (c) The typed UndefinedPredicate result: fsl-analyze's `document_failure` maps both the old `Graph(ExpansionRefused)` and `UndefinedPredicate` to `broken()`, so producer and checker engine results are unchanged. A third consumer the coder did not name changes, though: FND-002.
- (d) Fair cyclic until/always and FR-1042 `base` are not specified by PR 3551. FR-201 says "these proof profiles do not define ... fair until or always certificates" and "this profile does not establish FR-1042 base admission". AC-65's max_rational_bits settlement is specified and implemented. No specified feature was dropped.
- (e) `bounded_wrong_source_and_nonexact_rows_never_produce_an_accepted_document` fails on main without this PR. The claim is `eventually[0,2]`, and `qualified_formula_shape` returns `bounded_formula_shape` unchanged for any root other than Holds. None of the fsl-replay changes touch the Eventually or Formula paths. Main commit 4ebe69fa62 (QSL-2850, #3517) moved bounded max_rational_bits exits from `not_yet()` to `Incomplete(LimitReached{ExactProbMaxRationalBits})` and updated only the fsl-analyze tests. The test was last edited in #3474, before that commit. Settled by reading and history, not run. This is not a finding against this PR.

## Verdict

**CONDITIONAL**: no correctness defect found in the new checker. Three low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | claim_acceptance rewrites value and side but leaves accepted.certificate as violated-quantity rows; the proof record pairs a 1-v basis value with rows whose initial value is v | fsl-replay/src/probability_certificate/cyclic_state.rs:492-508; fsl-replay/src/proof_result.rs:1791 |
| FND-002 | low | Typed UndefinedPredicate also reaches FR-202 counterexample replay (witness.rs:985) and maps to ProbabilityWitnessError::Refused, so settlement moves from not_yet to ReplayRefused; unpinned | fsl-replay/src/probability_certificate/state_predicate.rs:159-161; fsl-replay/src/probability_certificate/witness.rs:985,1076 |
| FND-003 | low | CapturedStates keeps a clone of every reached ModelSimState, and end_components allocates count-sized vectors for each refinement, none charged to storage_bytes | fsl-replay/src/probability_certificate/cyclic.rs:205-211; fsl-replay/src/probability_certificate/cyclic_state.rs:278-356 |
