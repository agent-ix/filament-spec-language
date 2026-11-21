---
id: SR-202611212158130340200000004
title: "QSL-2158 cumulative probability WorkScope gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2158; 20 changed fsl-eval and fsl-analyze source/test files"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-120", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-203", type: references}
---

## Summary

Ticket QSL-2158, PR filament-spec-language#3405. gap-analysis examined the complete 20-file implementation diff against its merged cumulative-budget and early-admission contracts. Frozen head `665c88c05719853bee284f63555ef01a6484f4ca`; accepted base `ad94cbf94a62ff902cb3529f97fbd2720eb59db9`. Native Codex reviewer; exact model ID unavailable (`native-codex/unavailable`). No external model, delegation, source edits or local Cargo. Quoin 0.36.8; quire 0.52.0 / engine 0.62.2; spec-artifacts-process@v0.36.0.

## Verdict

**FAIL** — 4 source/trace findings require same-PR remediation. **Current-head compile and runtime qualification: PENDING.** Lead reports the current focused eval command stopped with environmental os error 28 (No space left on device) building libfsl_package.rlib: zero runtime tests and no Rust diagnostic. The intended analyze command did not run. The older E0063 checkpoint is neither current-head evidence nor an unresolved current source finding. No inferred compile PASS, runtime PASS or mergeable claim is made. All Rust execution remains with the lead/Titan lane.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-203-AC-13 has no engine-bound test; the new native cumulative-work test is tagged to AC-7 instead of its cumulative settlement criterion. | spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md:280 |
| FND-002 | medium | untimed_exact_admits_edges_before_relation_and_target_construction claims settlement AC-7 but tests early construction admission owned by AC-14, without binding AC-14. | fsl-analyze/tests/it/exact_probabilistic.rs:2627 |
| FND-003 | medium | untimed_exact_counts_distinct_pending_keys_and_reuses_full_state_capacity claims settlement AC-7 but tests early construction admission owned by AC-14, without binding AC-14. | fsl-analyze/tests/it/exact_probabilistic.rs:2696 |
| FND-004 | medium | The precondition factory test claims FR-107-AC-1 but constructs none of its ParentOrder, NoCycle or VersionAdvances cases; its assertions concern clause-set short circuit and duplicate refusal. | fsl-eval/src/simulation/model/probabilistic/tests.rs:293-295 |

## Evidence

FR-203-AC-13 is present in the actual merged spec and in computed matrix output with status untagged and zero binders. The native cumulative test already asserts serialized work_units/B/accounting.work_units and separate item behavior but omits that criterion ID. The two untimed observer tests return direct transition/state construction limits, never the settlement/refusal-map cases AC-7 describes; bind their actual AC-14 behavior. The factory precondition fixture uses Msg/send_a clauses A/B, asserts canonical false short circuit and duplicate set refusal, and never creates the ParentOrder/NoCycle/VersionAdvances AC-1 corpus; use its clause-set owners instead. Correct syntactic extraction alone does not establish intent.

## Coverage

Plan completion: not assessed

Reconciliation: `quire matrix --scope /home/peter/dev/worktrees/qsl-2158-cumulative-work-scope --format json` and `quire coverage --scope /home/peter/dev/worktrees/qsl-2158-cumulative-work-scope --json`, both exit 0; no run evidence read. Raw outputs and the selected matrix are retained beside this artifact. The six owned cumulative/admission criteria (FR-120-AC-35/36, FR-191-AC-8, FR-203-AC-13/14, FR-255-AC-23) compute tagged 5, untagged 1; ignored-only/other statuses 0. This is a scoped PR audit, not a whole-repository PASS. Broad pre-existing symbol collisions and unrelated matrix gaps are outside this finding set; their diagnostics are retained, not treated as new failures. Coverage reports no unmatched/untracked tags or suspicious assertions for the inspected new symbols. Three semantically wrong bindings are recorded below despite syntactically successful extraction.

Reverse-gap inventory: six changed behavior groups: clause factory/ports; optional system scope/fresh meter; local-first candidate budget; native per-item activation/method meters; untimed product admission; digital product admission and test-support observer. Owners are FR-107, FR-120, FR-191, FR-203 and FR-255. No new endpoint, worker pool, wire knob, vendored owner arithmetic or duplicate consumption ledger found. Source and changed tests contain substantive implementations/assertions; no new source/test stub or coverage-threshold weakening found. No applicable AssuranceProfile was found. Optional independent semantic model analysis was not invoked; source/test intent was inspected natively as requested, without Jev or another model.

Spec-review and its sub-analyses: not applicable (no specification/plan diff). Rust lane folded into code-review. No broad test, clippy, Kani, benchmark or baseline run was requested or performed.

Changed files examined:

- `fsl-analyze/src/exact_probabilistic.rs`
- `fsl-analyze/src/exact_probabilistic/build.rs`
- `fsl-analyze/src/exact_probabilistic/digital.rs`
- `fsl-analyze/src/statistical/model.rs`
- `fsl-analyze/tests/it/digital_clocks.rs`
- `fsl-analyze/tests/it/exact_probabilistic.rs`
- `fsl-analyze/tests/it/probability_from_source.rs`
- `fsl-analyze/tests/it/statistical_model.rs`
- `fsl-eval/src/simulation/model.rs`
- `fsl-eval/src/simulation/model/application.rs`
- `fsl-eval/src/simulation/model/contracts.rs`
- `fsl-eval/src/simulation/model/frames.rs`
- `fsl-eval/src/simulation/model/frames/narrowing_tests.rs`
- `fsl-eval/src/simulation/model/input.rs`
- `fsl-eval/src/simulation/model/probabilistic.rs`
- `fsl-eval/src/simulation/model/probabilistic/tests.rs`
- `fsl-eval/src/simulation/model/walk.rs`
- `fsl-eval/src/value/expression/clause_set.rs`
- `fsl-eval/src/value/expression/mod.rs`
- `fsl-eval/src/value/mod.rs`

## Scope and trace records

```yaml
clean: false
scope:
- id: FR-120-AC-35
  path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md
  role: examined
  excerpt: 'In one EN-4 or EN-5 model-subject run with local `max_candidates` above each expansion''s charge, two
    expansions that each fit `accounting.work_units = B` alone but whose combined candidate and clause work exceeds
    B stop at the first unpaid cumulative charge with `EvaluationMeter { counter: work_units }`; starting the second
    expansion does not reset the allowance. The same holds for two clause evaluations that fit separately but exceed
    B together, and for an expansion followed by a claim predicate. A separate run starts at zero; other accounting
    counters retain their existing scopes.'
- id: FR-120-AC-36
  path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md
  role: examined
  excerpt: 'For an argument vector, candidate and evaluated result pair, a metered fixture observes exactly one
    additional cumulative work unit apiece, beyond its existing clause work; an operation with no result adds no
    pair charge. A local `max_candidates` denial remains `MaxCandidates` even when the same candidate unit would
    also exceed cumulative work; a cumulative denial with local capacity remaining names `EvaluationMeter { counter:
    work_units }`. Cancellation during candidate generation returns the existing cancelled stop before another candidate
    is generated, and no stopped expansion supplies a co'
- id: FR-191-AC-8
  path: spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md
  role: examined
  excerpt: 'A two-test model-subject run with cumulative candidate and clause bound B, sufficient for each test
    alone but insufficient for both in ascending test order, spends the remainder on the later test and stops at
    the same first unpaid charge with workers 1, 2, 4 and 64. Reversing completion order of speculative samples
    changes no stop, counts, provenance, estimates or witnesses. Instrumented admitted candidate and clause work
    never exceeds B across all workers, including discarded samples; no later sample consumes an earlier sample''s
    entitlement. If the first test alone exhausts B, the later test '
- id: FR-203-AC-13
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: In a model-subject build, two expansions, two claim reads, and an expansion followed by a claim read
    each fit the caller's `accounting.work_units = B` individually but exceed B cumulatively. Each run stops at
    its first unpaid cumulative charge and settles `Incomplete(LimitReached{work_units, B, accounting.work_units})`
    with no truth. A separate run has a fresh allowance. A local candidate denial continues to settle `Incomplete(LimitReached{max_candidates,
    C, expansion.max_candidates})` at its configured C; cancellation retains `Incomplete(Cancelled{source})`.
- id: FR-203-AC-14
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: 'On a nonterminal product node with zero remaining transition capacity, an instrumented successor provider
    observes no successor-construction call before `MaxTransitions` incomplete settlement. With one edge slot remaining
    and multiple possible edges, only that slot is admitted before the next planned edge is denied; the full planned-edge
    vector is never materialized. At a full state ceiling an existing target key reuses its slot, while the first
    new key stops `MaxStates` before its planned target representation is constructed or retained, after only the
    work needed to establish the key. Stops '
- id: FR-255-AC-23
  path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md
  role: examined
  excerpt: Native analyze probability items with omitted candidate and accounting settings use local `expansion.max_candidates`
    1000000 and cumulative `accounting.work_units` 1000000 for either engine. Supplying a different accounting bound
    B preserves B. The local ceiling stops as `max_candidates` with `expansion.max_candidates`; the cumulative bound
    stops as `work_units` with `accounting.work_units`. Neither default is `u64::MAX`, and starting an expansion
    or clause does not renew cumulative work.
- id: FR-203-AC-7
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: The build-result map is exhaustive over `ProductRefusal` and `ClauseOutcome` with no `_` arm. A build
    whose expansion stops at `max_candidates` 2 settles `incomplete`, `Incomplete(LimitReached{max_candidates, 2,
    expansion.max_candidates})`. A build whose `ProductRefusal`, asserted before settling, is `Expansion(stop)`
    naming `EvaluationMeter` (a subject whose successor expansion charges the meter before the claim reads any predicate,
    under an accounting bound `B` of `work_units` its first expansion exceeds) settles `Incomplete(LimitReached{work_units,
    B, accounting.work_units})`, naming the ex
- id: FR-107-AC-1
  path: spec/functional/FR-107-evaluate-state-clauses-at-s6a.md
  role: examined
  excerpt: 'Over admitted observations of the FR-108 corpus: `ParentOrder` is `Completed(true)` for healthy-parent
    and absent-parent and `Completed(false)` for violating-parent; `NoCycle` is `Completed(false)` for cycle and
    self-loop and `Completed(true)` for healthy-parent; `VersionAdvances` is `Completed(true)` for unchanged-version
    and advanced-version and `Completed(false)` for changed-version.'
- id: FR-107-AC-14
  path: spec/functional/FR-107-evaluate-state-clauses-at-s6a.md
  role: examined
  excerpt: With `UpdateNotMax` added, the call from 3 to 4 gives the one result `UpdateAllowed` `Completed(false)`,
    and no result for `UpdateNotMax` or `UpdateRaises`; with the invariant `NoCycle` over its own observation added
    instead, the results are `NoCycle` `Completed(true)` then `UpdateAllowed` `Completed(false)`, and still none
    for `UpdateRaises`.
- id: FR-107-AC-16
  path: spec/functional/FR-107-evaluate-state-clauses-at-s6a.md
  role: examined
  excerpt: A set naming `UpdateAllowed` twice is refused `InputRefusal::DuplicateClause`; a set naming the function
    `sameIdentity` is refused `InputRefusal::UnknownClause`; a set pairing `UpdateAllowed`'s name with observations
    admitted for `UpdateRaises` is refused `InputRefusal::ObservationsMismatch`; none returns a result.
- id: FR-120-AC-29
  path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md
  role: examined
  excerpt: 'Over a model whose object type `Cell` (population `cells`, universe `{k1}`) declares `a: Int[0, 99]`
    and `b: Int[0, 99]` and whose one operation is `swap()` with frame `modifies a, b` and the clauses `post A {
    self.a = pre(self.b) }` and `post B { self.b = pre(self.a) }`, from the state with `a` 3 and `b` 5, the expansion
    has one successor, with `a` 5 and `b` 3. The `max_candidates` budget is one count for a whole expansion, over
    every operation and object, so each case below has one operation. The charge of an expansion is observed as
    the smallest `max_candidates` under which it completes: it'
- id: FR-120-AC-30
  path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md
  role: examined
  excerpt: 'Over AC-29''s `Cell` with one operation `step()` (frame `modifies a, b`) and the one clause `post A
    { self.a = pre(self.a) + 1 }`, from the state with `a` 3, the application has 100 candidates, one per value
    of `b`, in ascending `b` candidate order, and 100 successors, each with `a` 4. Its charge is 101: one argument
    vector and 100 candidates. The same model with the reference clause `post A { (self.a = pre(self.a) + 1) = true
    }`, a comparison of the conjunction with `true` that is neither an equation nor a bound and so solves no field,
    so both fields are enumerated, charges 10,001 and has the '
- id: FR-196-AC-3
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: context_only
  excerpt: §15.4's `Deliver` over every scheduler is an MDP with two actions at each live state, whose draws carry
    `9/10` and `1/10` (`send_a`) and `4/5` and `1/5` (`send_b`); under `Even` it is a DTMC with FR-187-AC-2's probabilities.
- id: FR-196-AC-4
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: context_only
  excerpt: FR-187-AC-3's `Health` variant with two post-states per draw builds, over every scheduler, an intermediate
    state per draw with two probability-1 actions, and the monitor does not advance on it; under a workload the
    build stops with `Unsupported(NotMarkov)`; the variant with no post-state stops with `Unsupported(NotMarkov)`
    under both. Two builds of one request are equal.
- id: FR-204-AC-3
  path: spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md
  role: context_only
  excerpt: '`Retx` with `delay Msg::send ~ discrete { 1 ms: 1, 2 ms: 1 };`, under a workload `One` with weight 1
    on `send`, is a DTMC over digital timed states; `probability >= 0.99 [eventually[0 ms, 4 ms] holds(m.delivered)]`
    settles `proved`, `ExactValue{159129/160000}`: after every reset `send` waits 1 or 2 ms with probability 1/2
    each, both inside its window `[1, 2]`, loses with `1/10`, and attempts at time stamps up to 4 ms inclusive count.
    The same claim `under every scheduler` is decided with the distribution applied (FR-204-AC-15); with `delay
    Msg::send ~ exponential(1 per ms);` under `One` and ex'
- id: FR-192-AC-1
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: context_only
  excerpt: 'Each outcome row maps exactly as the table states: an Accepted, a Rejected and an Undecided (`IndifferenceRegion`)
    `Completed` outcome; a `Stopped` naming `max_samples`, settling `Incomplete(LimitReached{limit, value, setting})`
    with setting `statistical.max_samples`; an `Unsupported` outcome with cause `NotMarkov`; a `NoTest` refusal
    settling `inconclusive`, `no-initial-state`; a `ConfidenceBeyondPrecision` refusal settling `unsupported`, `confidence-beyond-precision`;
    an `Internal` refusal settling `failed`; a `Stopped` by an expansion naming `EvaluationMeter` with exhausted
    counter `work_un'
- id: FR-120
  path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md
  role: examined
  excerpt: 'Before performing a candidate unit or clause work charge that would

    exceed the cumulative allowance, the simulator SHALL stop with the

    existing `EvaluationMeter` limit naming counter `work_units`; settlement

    SHALL name `work_units`, the configured bound and

    `accounting.work_units` under

    [FR-192](FR-192-settle-a-statistical-result-on-the-measured-axis.md) or

    [FR-203](FR-203-bound-an-exact-run-and-settle-its-verdict.md).'
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-203-AC-13
  related_ids:
  - FR-120-AC-35
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  lines: '280'
  excerpt: In a model-subject build, two expansions, two claim reads, and an expansion followed by a claim read
    each fit the caller's `accounting.work_units = B` individually but exceed B cumulatively. Each run stops at
    its first unpaid cumulative charge and settles `Incomplete(LimitReached{work_units, B, accounting.work_units})`
    with no truth. A separate run has a fresh allowance. A local candidate denial continues to settle `Incomplete(LimitReached{max_candidates,
    C, expansion.max_candidates})` at its configured C; cancellation retains `Incomplete(Cancelled{source})`.
  finding: FR-203-AC-13 has no engine-bound test; the new native cumulative-work test is tagged to AC-7 instead
    of its cumulative settlement criterion.
- fnd: FND-002
  severity: medium
  confidence: high
  method: gap-analysis
  check: trace
  artifact_id: FR-203-AC-7
  related_ids:
  - FR-203-AC-14
  path: fsl-analyze/tests/it/exact_probabilistic.rs
  lines: '2627'
  excerpt: The build-result map is exhaustive over `ProductRefusal` and `ClauseOutcome` with no `_` arm. A build
    whose expansion stops at `max_candidates` 2 settles `incomplete`, `Incomplete(LimitReached{max_candidates, 2,
    expansion.max_candidates})`. A build whose `ProductRefusal`, asserted before settling, is `Expansion(stop)`
    naming `EvaluationMeter` (a subject whose successor expansion charges the meter before the claim reads any predicate,
    under an accounting bound `B` of `work_units` its first expansion exceeds) settles `Incomplete(LimitReached{work_units,
    B, accounting.work_units})`, naming the ex
  finding: untimed_exact_admits_edges_before_relation_and_target_construction claims settlement AC-7 but tests early
    construction admission owned by AC-14, without binding AC-14.
  test_id: untimed_exact_admits_edges_before_relation_and_target_construction
  claimed_ac: FR-203-AC-7
  correct_ac: FR-203-AC-14
- fnd: FND-003
  severity: medium
  confidence: high
  method: gap-analysis
  check: trace
  artifact_id: FR-203-AC-7
  related_ids:
  - FR-203-AC-14
  path: fsl-analyze/tests/it/exact_probabilistic.rs
  lines: '2696'
  excerpt: The build-result map is exhaustive over `ProductRefusal` and `ClauseOutcome` with no `_` arm. A build
    whose expansion stops at `max_candidates` 2 settles `incomplete`, `Incomplete(LimitReached{max_candidates, 2,
    expansion.max_candidates})`. A build whose `ProductRefusal`, asserted before settling, is `Expansion(stop)`
    naming `EvaluationMeter` (a subject whose successor expansion charges the meter before the claim reads any predicate,
    under an accounting bound `B` of `work_units` its first expansion exceeds) settles `Incomplete(LimitReached{work_units,
    B, accounting.work_units})`, naming the ex
  finding: untimed_exact_counts_distinct_pending_keys_and_reuses_full_state_capacity claims settlement AC-7 but
    tests early construction admission owned by AC-14, without binding AC-14.
  test_id: untimed_exact_counts_distinct_pending_keys_and_reuses_full_state_capacity
  claimed_ac: FR-203-AC-7
  correct_ac: FR-203-AC-14
- fnd: FND-004
  severity: medium
  confidence: high
  method: gap-analysis
  check: trace
  artifact_id: FR-107-AC-1
  related_ids:
  - FR-107-AC-14
  - FR-107-AC-16
  path: fsl-eval/src/simulation/model/probabilistic/tests.rs
  lines: 293-295
  excerpt: 'Over admitted observations of the FR-108 corpus: `ParentOrder` is `Completed(true)` for healthy-parent
    and absent-parent and `Completed(false)` for violating-parent; `NoCycle` is `Completed(false)` for cycle and
    self-loop and `Completed(true)` for healthy-parent; `VersionAdvances` is `Completed(true)` for unchanged-version
    and advanced-version and `Completed(false)` for changed-version.'
  finding: The precondition factory test claims FR-107-AC-1 but constructs none of its ParentOrder, NoCycle or VersionAdvances
    cases; its assertions concern clause-set short circuit and duplicate refusal.
  test_id: probability_clause_factory_preserves_precondition_short_circuit_and_whole_set_refusal
  claimed_ac: FR-107-AC-1
  correct_ac: FR-107-AC-14
bindings:
- test_id: probability_work_scope_accumulates_real_expansions_on_source_and_package_routes
  ac_id: FR-120-AC-35
  trace: correct
- test_id: probability_clause_factory_shares_work_but_keeps_other_counters_fresh
  ac_id: FR-120-AC-35
  trace: correct
- test_id: probability_candidate_local_denial_precedes_shared_work_and_commits_only_admissions
  ac_id: FR-120-AC-36
  trace: correct
- test_id: probability_candidate_work_counts_argument_candidate_and_only_real_result_pairs
  ac_id: FR-120-AC-36
  trace: correct
- test_id: probability_candidate_then_claim_predicate_and_reward_use_the_same_allowance
  ac_id: FR-120-AC-35
  trace: correct
- test_id: serial_binding_tests_share_the_probability_work_allowance
  ac_id: FR-191-AC-8
  trace: correct
- test_id: native_probability_items_share_work_across_expansions_and_start_fresh_per_item
  ac_id: FR-120-AC-35
  trace: correct
- test_id: native_probability_items_preserve_caller_accounting_limits
  ac_id: FR-255-AC-23
  trace: correct
- test_id: direct_swap_reads_both_pre_values_and_observes_the_candidate_budget_boundary
  ac_id: FR-120-AC-29
  trace: correct
- test_id: guarded_partial_assignment_preserves_unsolved_values_effects_and_charges
  ac_id: FR-120-AC-30
  trace: correct
- test_id: digital_zero_edges_denies_before_successor_construction
  ac_id: FR-203-AC-14
  trace: correct
- test_id: digital_one_edge_denies_before_the_next_target_is_materialized
  ac_id: FR-203-AC-14
  trace: correct
- test_id: digital_full_state_capacity_reuses_existing_keys_before_denying_new_keys
  ac_id: FR-203-AC-14
  trace: correct
- test_id: digital_pending_duplicate_keys_share_one_state_slot
  ac_id: FR-203-AC-14
  trace: correct
- test_id: digital_pending_distinct_keys_deny_before_the_second_new_target
  ac_id: FR-203-AC-14
  trace: correct
- test_id: digital_race_merges_repeated_outcomes_at_full_edge_and_state_capacity
  ac_id: FR-203-AC-14
  trace: correct
- test_id: digital_boundary_equal_capacity_completes
  ac_id: FR-203-AC-14
  trace: correct
- test_id: untimed_exact_admits_edges_before_relation_and_target_construction
  ac_id: FR-203-AC-7
  trace: wrong
- test_id: untimed_exact_counts_distinct_pending_keys_and_reuses_full_state_capacity
  ac_id: FR-203-AC-7
  trace: wrong
- test_id: probability_clause_factory_preserves_precondition_short_circuit_and_whole_set_refusal
  ac_id: FR-107-AC-1
  trace: wrong
```
