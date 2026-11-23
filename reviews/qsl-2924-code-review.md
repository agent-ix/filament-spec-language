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

## New findings (disposition pass 1)

Re-reviewed at head c5ff46f6601c85d75886d5463db327b19acda12d (rebased on main 7ca2bd11ab). `git range-diff` shows the three original commits unchanged, so this pass reviews only the round commits 29fcdc5fb5, 2092bcce49 and c5ff46f660. No cargo command was run.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | A cyclic-dtmc-state-predicate/v1 document relabelled cyclic-mdp-state-predicate/v1 is accepted, and the new test pins that acceptance. FR-201's Rejections bullet says a changed profile SHALL refuse objective mismatch, and FR-201-AC-62 says changing the profile prevents acceptance. The reverse relabel (MDP to DTMC) gives Unsupported, not ObjectiveMismatch. The checker should derive the expected profile from the subject (DTMC when every live state has one distribution, MDP otherwise) and refuse a different one as ObjectiveMismatch, or the spec should be amended. | fsl-replay/src/probability_certificate/document.rs:553-560; fsl-analyze/tests/it/cyclic_state_certificate.rs:1502-1514 |
| FND-005 | medium | end_components charges count*(24+64) + edges*8 bytes up front and then the same amount again for every refinement work item. The count-sized vectors are freed after each iteration but the charge is cumulative, so the total grows with (number of SCC splits) * (number of states), and the first item is charged twice. A large subject with many components can exhaust StorageBytes although its peak memory fits. Charge the peak (once, or refund per item), or size the vectors to the set. | fsl-replay/src/probability_certificate/cyclic_state.rs:291-330 |
| FND-006 | low | The pin test for the restored replay mapping asserts `Err(ProbabilityWitnessError::Graph(_))` rather than `Graph(ExactProbabilityGraphError::ExpansionRefused)`. A Graph error raised earlier for another reason would also pass. It does tell apart the regression it guards (`Refused`). | fsl-analyze/tests/it/state_probabilistic_replay.rs:665-694 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | claim_acceptance now converts every accepted.certificate row with claim_value (cyclic_state.rs:522-525). The equality test asserts that the initial certificate row, accepted.value(), the proof-record basis value and exact_certificate's row are one number. |
| FND-002 | fixed | witness.rs:1055-1061 maps UndefinedPredicate back to Graph(ExpansionRefused) before document_error, so FR-202 replay settles as before. The path is reachable through replay_probabilistic_witness directly (the engine's product build reports undefined first). The pin test replays a defined-world counterexample over a world whose B is undefined at attempts = 1, which the rejecting path reaches. Weak assertion: FND-006. |
| FND-003 | still-open | Charges added (cyclic.rs:210-222, cyclic_state.rs:291-330), but the test asserts an absolute least ceiling `>= 790_000`. That number depends on size_of::<ModelSimState>(), size_of::<CheckerState>(), key byte lengths and every other storage charge in the pipeline. The gap to the uncharged baseline (779,240) is about 10.8 KB, and the baseline was not re-measured after the threshold was tightened. An unrelated accounting or layout change makes the test vacuous or red. A deterministic assertion is needed, for example an in-crate unit test that calls end_components / enumerate_graph_capturing with storage_bytes set to the amount computed from size_of (expect Limit StorageBytes at that amount minus 1, success at that amount). |

## New findings (disposition pass 2)

Re-reviewed at head 8f976fddaa9da8f11c9fb09d2535c0e32e1a71ab (rebased on main 2baf370e59). `git range-diff` shows the six earlier commits unchanged, so this pass reviews the round commit 8f976fddaa. The committed reviews/ files match my d1 files byte for byte. No cargo command was run. The Rust 1.98 toolchain's rustfmt was run on the changed files.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | end_components allocates count-sized vectors (member, adjacency, and the Tarjan number, low and on_stack arrays) for every refinement work item, but the meter is visited only once per member of the item's set. A subject whose classification graph splits into many components (for example a line of self-looping states: count singleton items) costs O(count^2) time while charging O(count) work units, so a configured work budget does not bound end-component decomposition (FR-201-AC-63). Size the per-item vectors to the set (a local index map), or meter count per item. | fsl-replay/src/probability_certificate/cyclic_state.rs:352,379,407 |
| FND-008 | low | The new retry guard in probability_proof_document is not rustfmt-formatted: rustfmt (toolchain 1.98) rewrites it, while mdp.rs on main and at the previous head is clean, so `cargo fmt --check` fails on fsl-replay. (The new lines at tools/fsl-probability-check/tests/it/bounded.rs:1370-1405 are not formatted either, but that file already differs from rustfmt output on main.) | fsl-replay/src/probability_certificate/document/mdp.rs:149 |
| FND-009 | low | The new end_component_peak doc comment was inserted directly under end_components' doc comment ("The maximal end components of the classification graph ..."). That text now attaches to end_component_peak, and end_components has no doc. | fsl-replay/src/probability_certificate/cyclic_state.rs:287-303 |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | The fragile absolute-threshold IT was removed. The in-crate unit test end_component_storage_is_charged_at_its_computed_peak_once runs end_components on a 64-state line with storage_bytes = end_component_peak(count, edges), computed from size_of: the exact amount succeeds and one byte less gives Limit StorageBytes with bound exact - 1. This is deterministic. The capture charge is pinned only by its formula (captured_cost), because enumerate_graph_capturing needs a ModelSystem. That is acceptable: the charge is a straight-line call, and the IT bisection asserts StorageBytes as the typed cause for every ceiling below the least accepted one, including in OriginalAdmission. |
| FND-004 | fixed | certificate_graph derives the profile from the original graph (`choosy` = some state with more than one choice) after classification, so an undefined predicate still reports first. A DTMC document on a subject with a choice and an MDP document on a one-distribution subject both refuse ObjectiveMismatch; a workload under the MDP profile refuses ObjectiveMismatch; the mutation test asserts error:ObjectiveMismatch in both directions. Producer: probability_proof_document retries the MDP profile on ObjectiveMismatch for until/always claims only. No DTMC-eligible subject can be issued the MDP profile, because the MDP certificate_graph refuses a choice-free subject with ObjectiveMismatch. That satisfies FR-201 Admission ("An eligible DTMC producer SHALL retain the DTMC profile"). |
| FND-005 | fixed | The peak is charged once, up front, and the per-item charge is gone. The 64-split unit test would fail at the exact amount if any per-item charge remained. |
| FND-006 | fixed | The pin test asserts Err(ProbabilityWitnessError::Graph(ExactProbabilityGraphError::ExpansionRefused)). |

## New findings (disposition pass 3)

Re-reviewed at head 1c5aa755680ace647188f84b49148cc35522818f (rebased on main 27044cf5e9). `git range-diff` shows the seven earlier commits unchanged, so this pass reviews the round commit 1c5aa75568. The committed reviews/ files match my d2 files byte for byte. No cargo command was run. The pinned 1.98 rustfmt (1.9.0-stable 48a229ceae) was run with --check on every .rs file in the PR diff, and on the main versions of the same files for comparison.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | end_component_work_is_linear_in_the_states_and_bounded_by_the_work_ceiling cannot fail on the code before this round. The removed count-sized vectors were never metered: the old code also called visit() once per set member, adjacency action and Tarjan step. Its least work ceiling was the same linear number, so `least < 16 * count` held before the fix too, and the comment "Count-sized vectors per item would cost count * count / 2 visits" is wrong. The fix is right by reading; the test does not show it. | fsl-replay/src/probability_certificate/cyclic_state.rs:659-691 |
| FND-011 | low | Lines added by the PR's first commits are not rustfmt-formatted under the pinned 1.98 toolchain, so `cargo fmt --all -- --check` (ci.yml:28, Makefile:60) reports them. The main versions of both files are clean. The spots: the `shape.ok_or(...)` chain at fsl-eval/src/automaton/bounded.rs:156 (should be joined onto one line), and the `fixture(...)` call and two `assert_eq!` lines at fsl-eval/tests/it/qualified_bounded_monitor.rs:371 and :440 (should be wrapped). | fsl-eval/src/automaton/bounded.rs:156; fsl-eval/tests/it/qualified_bounded_monitor.rs:371,440 |

## Dispositions (round 3)

| FND | outcome | reason |
| --- | --- | --- |
| FND-007 | fixed | Every per-item vector (member, adjacency, the Tarjan arrays via strongly_connected(&roots, &adjacent, set.len(), ...), roots, and mapping parts back) is now sized to the item's set, with membership by binary search on the sorted set. Per-item cost is proportional to |set| times (degree * log |set|), and visit() runs at least once per member in the prune pass, so the work meter bounds the stage up to the per-edge factor the earlier code also had. No O(count) allocation per item remains. The new test does not show this: FND-010. |
| FND-008 | fixed | document/mdp.rs is clean under the pinned 1.98 rustfmt, with the guard wrapped. The PR's own hunks in tools/fsl-probability-check/tests/it/bounded.rs (1370-1414) are clean. The remaining diffs at lines 84 and 95 are identical on origin/main and are not this PR's. Other files (fsl-eval): FND-011. |
| FND-009 | fixed | end_components' doc comment now sits on end_components, and end_component_peak keeps its own. |
