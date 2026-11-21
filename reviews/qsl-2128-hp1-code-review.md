---
id: SR-551
title: "QSL-2128 HP-1 certificate code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; all 25 changed production/test/spec files in PR #3334"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-163
    type: references
---

## Summary

Full-review head: 76993592e438ff03b2a2af1dfdae5c50bbf966b6; current narrow-integration head: cab7971ef42919e2ca56456eae5a993fbcd8c286. Ticket: QSL-2128; PR #3334. Fresh main collision check: 660a550b40b068df5337155009e009299654f3b1 has no SR-551 through SR-556. No applicable AssuranceProfile exists under spec/. Review selection: base, integrity, failure-domain and EARS for the final amendment; other specification lenses do not match the bounded change.

No new defect found in this method's bounded inspection.

## Verdict

**PASS** — No introduced defect found in the full review or narrow integration. Current cab7971 library check and both recommendation smokes pass; prior 76993592 HP-1 evidence remains explicitly qualified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Rust lane folded into code-review. Inspected all 25 changed files against frozen base 06ba58566456f14c5cdb3a53c8b97f7aaf9fb6e9, including callers and test assertions. No workflow, dependency, accounting, catalog or wire-cause change. No introduced duplication, stubs, unsafe code, suppression or proof flag. ModelRelationTransition now has one shared owner; producer, core checker and refutation replay all use ModelSystem::relation_successors. The independent BFS deliberately lives in fsl-replay and consumes existing lower-level model/evaluator primitives, without depending on fsl-analyze or its retained graph.

AllResults changes only the bounded walk selection; all ordinary constructors retain FirstResult. Each result remains candidate-budget charged, cancellation is polled by the budget/evaluator, and a later denial propagates through frames::apply and expand before any successor set is returned. ModelSimState::reached preserves control and restored provenance in both policies. The finite preexisting result-order limit remains intact. Exact edge comparison precedes tuple acceptance, preserves typed results and post-states, and the tuple charge stops before body evaluation. CertificateUnchecked remains the absent/unperformed result, actual rejection remains CertificateRejected, and only Accepted reaches Certified.

Test assertions were checked for real API execution, exact values, reachable negative paths and registration in tests/it/main.rs. Coalesced-result refutation is exercised in producer, independent checker and replay. Temporal certificate wrappers preserve their prior serialization and outcome distinctions. Reached-limit settlement additionally retains its typed bound.

## Qualification limits

Trace bindings checked by manual source inspection (not an engine matrix):

```yaml
bindings: [{"test_id":"hp1_det_certificate_is_deterministic_and_independently_certified","ac_id":"FR-163-AC-10","trace":"correct"},{"test_id":"hp1_state_mismatches_name_the_actual_missing_or_extra_key","ac_id":"FR-163-AC-10","trace":"correct"},{"test_id":"hp1_closed_unreachable_superset_is_rejected_at_its_least_extra_state","ac_id":"FR-163-AC-10","trace":"correct"},{"test_id":"hp1_unsorted_and_duplicate_states_fail_before_recomputation","ac_id":"FR-163-AC-10","trace":"correct"},{"test_id":"hp1_checker_owns_its_tuple_allowance_and_names_first_unpaid_tuple","ac_id":"FR-163-AC-13","trace":"correct"},{"test_id":"hp1_aliases_follow_declarations_and_edge_sequences_are_exact_inputs","ac_id":"FR-163-AC-11","trace":"correct"},{"test_id":"hp1_false_and_undefined_rejections_keep_complete_first_tuples","ac_id":"FR-163-AC-6","trace":"correct"},{"test_id":"hp1_false_and_undefined_rejections_keep_complete_first_tuples","ac_id":"FR-163-AC-12","trace":"correct"},{"test_id":"hp1_coalesced_results_and_typed_null_remain_distinct","ac_id":"FR-163-AC-12","trace":"correct"},{"test_id":"hp1_result_expansion_keeps_all_results_and_ordinary_successors_stay_unchanged","ac_id":"FR-163-AC-12","trace":"correct"},{"test_id":"hp1_checker_state_transition_and_cancellation_stops_never_certify","ac_id":"FR-182-AC-17","trace":"correct"},{"test_id":"hyper_step_relation_through_analyze","ac_id":"FR-182-AC-17","trace":"correct"}]
```

Current narrow-integration frozen head: cab7971ef42919e2ca56456eae5a993fbcd8c286, based on main 9943d97241ad3faa4c83fd42feea5b899a96cdc1. The full bounded review remains the original inspection at 76993592e438ff03b2a2af1dfdae5c50bbf966b6; this follow-up examines only integration with main's #3340 recommendation changes. Own HP-1 algorithm, contract and test files have no delta from that reviewed head. The worktree is clean; git diff --check passes.

The added Outputs.recommendations field is None in the normal expansion path. Consequently the recommendation-only enforcement filter and precondition-accepted push/continue branch are inert for ordinary FirstResult and HP-1 AllResults. recommend_operation uses walk_of's default FirstResult, and the memoized Walk initializer explicitly retains FirstResult. Both policy arms still construct successors through ModelSimState::reached, preserving control/restored provenance. No introduced defect was found in this narrow inspection; no #3336 cherry-pick, HP-1 logic, specification, test or resource-accounting change occurs.

Actual prior-head receipts 33–54 qualify the library check and 32 runtime passes, zero failures or ignored tests, at 76993592. They are retained as prior-head evidence, not relabelled as current-head runtime tests. Current-head receipt 55-recommendation-integration-check.log passes the library check in 27.20 seconds, including typed initializer coverage. Current-head receipt 56 verifies a_recommendation_ignores_selected_postconditions: one pass, 188 filtered; receipt 57 verifies a_false_precondition_disables_the_recommendation: one pass, 188 filtered. Both have zero failures and zero ignored tests. These are actual inspected results at the unchanged cab7971 head. The narrow integration is ready for focused merge after final artifact custody and ordinary CLA/current-overlap checks. This reviewer ran no Cargo or duplicate gates. No whole QSL-2128 completion is claimed.

The whole-repository matrix/coverage/advisor audit remains unavailable under the authorized bounded dispatch. This is an explicit focused-push assurance limit, not an introduced source finding, and prevents an engine-backed or full-repository assurance PASS; SR-552 remains CONDITIONAL solely on that limit. Strict clippy's documented unchanged baseline remains disclosed, with no suppression or unrelated cleanup. QSL-2128 remains open for external FR-940-AC-18 third-pin and driver acceptance.

## Examined scope

```yaml
scope: [
  {
    "id": "FR-163-AC-1",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "Accepted vectors: §8.1's secure `NonInterference` certificate (8 product states, every component `MissingAcceptance` or `Trivial`) is `Accepted`; the copy-swap certificate (6 states) is `Accepted`; §8.2's secure `Opaque` certificate is `Accepted`; FR-179-AC-1's `Det` certificate is `Accepted`; FR-181-AC-1's HP-5 certificate is `Accepted`."
  },
  {
    "id": "FR-163-AC-2",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "Rejected vectors over the AC-1 `NonInterference` certificate: one member removed is `Rejected` with rule `SuccessorMissing` naming the member whose successor is missing; its keys unsorted is `Rejected` with rule `Malformed` before any recomputation; an added member whose `l` components differ, in a component marked `MissingAcceptance`, is `Rejected` with rule `WitnessFails`; one member listed in two components is `Rejected` with rule `NotPartition`; two components joined by an edge listed in the reverse order is `Rejected` with rule `BackwardEdge`; the initial product state removed is `Rejecte"
  },
  {
    "id": "FR-163-AC-4",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "The AC-1 `NonInterference` certificate checked with `max_states` 1 returns `Stopped` with `{MaxStates, 1}` naming the limit and its value; checking it twice gives equal results."
  },
  {
    "id": "FR-163-AC-5",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "HP-2 and HP-3 certificates with absent `components`, including one with an empty closure, reject `Malformed` at `CertificateMember(\"components\")` before any successor recomputation, without selecting a state key."
  },
  {
    "id": "FR-163-AC-6",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "An HP-1 certificate for FR-179-AC-2's nondeterministic `Det` subject rejects `ViolatingMember` at the first complete labelled edge tuple in canonical order; equal pre-state keys and arguments with different post-states remain distinct. FR-179-AC-4's undefined body rejects `UndefinedMember` at its complete edge tuple."
  },
  {
    "id": "FR-163-AC-7",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "For ADR-023 HC-2 with zero universal variables, a supplied key with an empty universal tuple and its actual witness set retains both on serialization and reading; the empty witness set remains distinct from an absent witness set. No component is added to satisfy a nonempty-array constraint."
  },
  {
    "id": "FR-163-AC-9",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "Production order. §8.1's secure `NonInterference` proof run twice writes byte-equal RFC 8785 certificates, whose `closure` keys are ascending, whose components are in FR-126's component order over the self-composed product's canonical breadth-first indices, and whose components each list their states ascending in the closure's key order. The same certificate with two components that no edge joins listed in the other order is still `Accepted`."
  },
  {
    "id": "FR-163-AC-10",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "The HP-1 `Det` certificate of FR-179-AC-1 produced twice has byte-equal RFC 8785 serialization and one subject closure for its repeated alias, with ascending state keys and no automaton key; the independent checker accepts it after evaluating 64 tuples. Removing its initial key rejects `InitialMissing` at that missing key; removing a noninitial reachable key rejects `SuccessorMissing` at that missing key. Adding an unreachable, foreign or noncanonical key rejects `Malformed` at that key. Initial membership failures precede other set differences; subsequent simultaneous missing and extra keys s"
  },
  {
    "id": "FR-163-AC-11",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "Over two secure-vault model aliases declared `V` then `N` and execution bindings `(y: N::Vault::step, x: V::Vault::step, z: V::Vault::step)`, a relation with body `true` has exactly the subject-closure alias sequence `[V, N]`, with execution entries `[x, z]` and `[y]` respectively, retaining both alias entries even for equal state keys. Each execution contains its complete typed applicable edges and the certificate is accepted. Omitting `N`, duplicating `V`, adding a third alias, reversing the alias entries or removing or altering a supplied execution edge refuses the certificate request with "
  },
  {
    "id": "FR-163-AC-12",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "FR-163-AC-6's false and undefined tuple rejections serialize the complete labelled edges in execution-binding order. Edges with equal pre-state and arguments but unequal post-states remain distinguishable; edges with equal pre-state, transition and post-state but unequal typed results remain distinguishable. A no-result edge serializes `result: null`, while an edge returning a typed null retains that typed value. Neither tuple acquires an automaton key."
  },
  {
    "id": "FR-163-AC-13",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "Checking FR-179-AC-1's certificate independently with `max_relation_tuples` 63 stops before evaluating tuple 64, naming `MaxRelationTuples`, value 63, count 64 and the complete labelled tuple 64 as its first unpaid tuple; settlement is incomplete with setting `model_check.max_relation_tuples`, no counterexample and the unchanged limit-cause shape, never `CertificateUnchecked` or proved. With 64 it accepts, including when the engine already evaluated its 64 tuples under the same request setting. A second independent check with 64 also accepts."
  },
  {
    "id": "FR-179-AC-1",
    "path": "spec/functional/FR-179-check-a-step-relation-over-reachable-transitions.md",
    "role": "examined",
    "excerpt": "FR-171-AC-3's `Det` over ADR-023 §8's secure vault returns `Holds{Exhaustive}` after evaluating every pair of the 8 reachable `step` edges (64 tuples)."
  },
  {
    "id": "FR-179-AC-2",
    "path": "spec/functional/FR-179-check-a-step-relation-over-reachable-transitions.md",
    "role": "examined",
    "excerpt": "`Det` over a vault variant whose `step` postcondition is `self.l = i or self.l = 1 - i` (a nondeterministic post-state) returns `Violated` with a `StepTuple` counterexample: two executions of `step` with equal pre-states and inputs and unequal post-states, each with a path from an initial state to its pre-state."
  },
  {
    "id": "FR-179-AC-3",
    "path": "spec/functional/FR-179-check-a-step-relation-over-reachable-transitions.md",
    "role": "examined",
    "excerpt": "AC-1's request with `max_relation_tuples` 10 returns `Stopped(ResourceExhausted, MaxRelationTuples)` after 10 tuples, naming the limit and its value; with `max_depth` 1 it enumerates only the 4 transitions from the two initial states (16 tuples) and returns `BoundReached{depth: 1}`. Running AC-2's request twice gives byte-equal counterexamples."
  },
  {
    "id": "FR-179-AC-4",
    "path": "spec/functional/FR-179-check-a-step-relation-over-reachable-transitions.md",
    "role": "examined",
    "excerpt": "Over a `Cell` model (field `d: Int[0, 1]`, operation `set(k: Int[0, 1])` with postcondition `self.d = k`, initial `d = 0`), `relation R using v over (x: C::Cell::set, y: C::Cell::set) { 1 / x.k >= y.k }` evaluates every tuple with `x.k = 1` `true` and every tuple with `x.k = 0` undefined, and returns `Violated` with an `Undefined` counterexample on the first tuple in canonical order, which has `x.k = 0`, carrying `UndefinedEvaluation{where: that tuple, cause: division-by-zero}`; it never returns `Holds`."
  },
  {
    "id": "FR-182-AC-17",
    "path": "spec/functional/FR-182-settle-a-hyper-verdict.md",
    "role": "examined",
    "excerpt": "FR-179-AC-1's HP-1 `Det` result, after all 64 tuples evaluate true and every subject's reachable graph closes, with no certificate supplied or with a certificate on which no core check ran, settles `Incomplete(CertificateUnchecked)`, category incomplete, with cause `{\"type\": \"bounded-search-incomplete\", \"reason\": \"certificate-unchecked\"}`, no `task_search` evidence and no frontier; it never settles `Proved`, `Certified` or `Uncertified`. The same result with FR-163's HP-1 certificate accepted settles V-1 `proved`, `Exhaustive`, `Certified`; with the certificate rejected it settles V-6 `Certifi"
  },
  {
    "id": "FR-184-AC-3",
    "path": "spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md",
    "role": "examined",
    "excerpt": "FR-179-AC-1's `Det` with `max_relation_tuples` 63 stops `Stopped(ResourceExhausted, MaxRelationTuples)` with value 63 and count 64; with 64 it completes `Holds`."
  },
  {
    "id": "FR-184-AC-4",
    "path": "spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md",
    "role": "examined",
    "excerpt": "FR-179-AC-1's engine run with `max_relation_tuples` 64 evaluates its 64 tuples and the resulting certificate's independent check under the same setting evaluates its own 64 tuples and accepts. Rechecking it starts from zero and accepts again. Checking that certificate under 63 stops with value 63 and count 64 before the 64th body evaluation and identifies the complete labelled tuple 64 as the first unpaid tuple; the record names `model_check.max_relation_tuples`, retains the existing limit-cause shape and carries no counterexample."
  },
  {
    "id": "FR-163/HP-1-reconstruction/1",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "- **HP-1 reconstruction.** The HP-1 certificate checker SHALL read the\n  checked relation and independently reconstruct each alias's complete\n  reachable state set by canonical breadth-first expansion from its initial\n  states through its `ModelSystem`, using no retained engine graph. Each\n  supplied state set SHALL equal that independently reachable set. The\n  producer emits the reachable closure; a closed unreachable superset adds\n  no proof strength, and exact equality lets the checker use the existing\n  model admission and successor primitives without a state-key decoder."
  },
  {
    "id": "FR-163/HP-1-reconstruction/2",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "When reconstruction completes with missing initial states, the checker SHALL reject\n  `InitialMissing`, first by alias declaration order and then by ascending\n  state-key octets."
  },
  {
    "id": "FR-163/HP-1-reconstruction/3",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "When all initial membership checks pass, the checker SHALL compare each alias's supplied and reachable sets in alias declaration\n  order, selecting the least key in their symmetric difference by state-key\n  octets."
  },
  {
    "id": "FR-163/HP-1-reconstruction/4",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "When the selected key is missing from the supplied set, the checker SHALL reject `SuccessorMissing` at that missing key."
  },
  {
    "id": "FR-163/HP-1-reconstruction/5",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "When the selected key is extra in the supplied set, including a noncanonical or foreign key, the checker SHALL reject `Malformed` at that supplied key. This ordering also\n  applies when missing and extra keys occur together. Every admitted member\n  is therefore independently reached and expanded. The checker SHALL recompute every edge\n  independently of the certificate's edge sequences. After closure is\n  checked, the checker SHALL compare each supplied execution's complete\n  typed edge sequence with the recomputed applicable edges; a missing,\n  extra, reordered or altered edge is an input mis"
  },
  {
    "id": "FR-163/HP-1-reconstruction/6",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "Edges with equal pre-state keys remain distinct when their transition,\n  post-state or result differs. A tuple rejection SHALL serialize as FSpec\n  FR-331's existing `HyperTransitionTuple`: exactly `transitions`, one edge\n  per execution binding, each with `pre_state`, `transition`, `post_state`\n  and `result`. The result uses the existing typed canonical value; JSON\n  `null` denotes an operation with no result, separately from a typed null\n  result. A state-member rejection SHALL use the existing hyper state\n  locus with `states` containing the actual offending state key and no\n  automaton ke"
  },
  {
    "id": "fsl-eval/src/simulation/model/frames.rs:integration",
    "path": "fsl-eval/src/simulation/model/frames.rs",
    "role": "examined",
    "excerpt": ".filter(|_| recommendations.is_none())"
  },
  {
    "id": "fsl-eval/src/simulation/model/walk.rs:integration",
    "path": "fsl-eval/src/simulation/model/walk.rs",
    "role": "examined",
    "excerpt": "recommendations: None,"
  },
  {
    "id": "fsl-eval/src/simulation/model/application.rs:integration",
    "path": "fsl-eval/src/simulation/model/application.rs",
    "role": "examined",
    "excerpt": "let walk = walk_of(self, observed, None)?;"
  },
  {
    "id": "fsl-eval/src/simulation/model/frames.rs:result-provenance",
    "path": "fsl-eval/src/simulation/model/frames.rs",
    "role": "examined",
    "excerpt": "state: self.walk.state.reached(\n                            candidate,\n                            handler_anchor(&self.application.text),\n                            effect,\n                        ),"
  }
]
```

