---
id: SR-555
title: "QSL-2128 HP-1 certificate spec-review-failure-domain"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/filament-spec-language; spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md; final HP-1 exact-set amendment"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-163
    type: references
---

## Summary

Full-review head: 76993592e438ff03b2a2af1dfdae5c50bbf966b6; current narrow-integration head: cab7971ef42919e2ca56456eae5a993fbcd8c286. Ticket: QSL-2128; PR #3334. Fresh main collision check: 660a550b40b068df5337155009e009299654f3b1 has no SR-551 through SR-556. No applicable AssuranceProfile exists under spec/. Review selection: base, integrity, failure-domain and EARS for the final amendment; other specification lenses do not match the bounded change.

No new defect found in this method's bounded inspection.

## Verdict

**PASS** — The final amendment has no identified defect in this selected lens. Runtime/integration readiness is separate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Examined trust boundary and topology: supplied keys cannot add unreachable evidence; arbitrary foreign/noncanonical octets are detected by exact set difference without decoding them. Cycles terminate through reached-key membership; reconstruction has existing state/transition/candidate/evaluation limits and cancellation, with no acceptance of partial expansion. Alias identity stays per declaring source unit and state-key equality does not collapse alias ownership. InitialMissing precedence is stated across aliases before other set differences. Body purity and existing evaluator semantics are reused, not redefined. Unreachable disconnected closed components are a specified rejection. No unstated failure behavior was found in the amendment.

## Qualification limits

Trace bindings checked by manual source inspection (not an engine matrix):

```yaml
bindings: [{"test_id":"hp1_det_certificate_is_deterministic_and_independently_certified","ac_id":"FR-163-AC-10","trace":"correct"},{"test_id":"hp1_state_mismatches_name_the_actual_missing_or_extra_key","ac_id":"FR-163-AC-10","trace":"correct"},{"test_id":"hp1_closed_unreachable_superset_is_rejected_at_its_least_extra_state","ac_id":"FR-163-AC-10","trace":"correct"},{"test_id":"hp1_unsorted_and_duplicate_states_fail_before_recomputation","ac_id":"FR-163-AC-10","trace":"correct"}]
```

Current narrow-integration frozen head: cab7971ef42919e2ca56456eae5a993fbcd8c286, based on main 9943d97241ad3faa4c83fd42feea5b899a96cdc1. The full bounded review remains the original inspection at 76993592e438ff03b2a2af1dfdae5c50bbf966b6; this follow-up examines only integration with main's #3340 recommendation changes. Own HP-1 algorithm, contract and test files have no delta from that reviewed head. The worktree is clean; git diff --check passes.

The added Outputs.recommendations field is None in the normal expansion path. Consequently the recommendation-only enforcement filter and precondition-accepted push/continue branch are inert for ordinary FirstResult and HP-1 AllResults. recommend_operation uses walk_of's default FirstResult, and the memoized Walk initializer explicitly retains FirstResult. Both policy arms still construct successors through ModelSimState::reached, preserving control/restored provenance. No introduced defect was found in this narrow inspection; no #3336 cherry-pick, HP-1 logic, specification, test or resource-accounting change occurs.

Actual prior-head receipts 33–54 qualify the library check and 32 runtime passes, zero failures or ignored tests, at 76993592. They are retained as prior-head evidence, not relabelled as current-head runtime tests. Current-head receipt 55-recommendation-integration-check.log passes the library check in 27.20 seconds, including typed initializer coverage. Current-head receipt 56 verifies a_recommendation_ignores_selected_postconditions: one pass, 188 filtered; receipt 57 verifies a_false_precondition_disables_the_recommendation: one pass, 188 filtered. Both have zero failures and zero ignored tests. These are actual inspected results at the unchanged cab7971 head. The narrow integration is ready for focused merge after final artifact custody and ordinary CLA/current-overlap checks. This reviewer ran no Cargo or duplicate gates. No whole QSL-2128 completion is claimed.

The whole-repository matrix/coverage/advisor audit remains unavailable under the authorized bounded dispatch. This is an explicit focused-push assurance limit, not an introduced source finding, and prevents an engine-backed or full-repository assurance PASS; SR-552 remains CONDITIONAL solely on that limit. Strict clippy's documented unchanged baseline remains disclosed, with no suppression or unrelated cleanup. QSL-2128 remains open for external FR-940-AC-18 third-pin and driver acceptance.

## Examined scope

```yaml
scope: [
  {
    "id": "FR-163-AC-10",
    "path": "spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md",
    "role": "examined",
    "excerpt": "The HP-1 `Det` certificate of FR-179-AC-1 produced twice has byte-equal RFC 8785 serialization and one subject closure for its repeated alias, with ascending state keys and no automaton key; the independent checker accepts it after evaluating 64 tuples. Removing its initial key rejects `InitialMissing` at that missing key; removing a noninitial reachable key rejects `SuccessorMissing` at that missing key. Adding an unreachable, foreign or noncanonical key rejects `Malformed` at that key. Initial membership failures precede other set differences; subsequent simultaneous missing and extra keys s"
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
  }
]
```

