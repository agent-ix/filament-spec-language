---
id: SR-967175026302197768900474181941935084685
title: QSL-1978 positive policy code-review
type: SpecReview
analysis: code-review
scope: agent-ix/filament-spec-language; PR3408 QSL-1978; owned diff b68f04c445396b1e66fa71063337d001f3717dcd..df2e5e322f2bf44fe0a401d1ae69bafd95cfac13
review_set: subset
---

## Summary

Frozen candidate: `df2e5e322f2bf44fe0a401d1ae69bafd95cfac13`. Base: `b68f04c445396b1e66fa71063337d001f3717dcd`. Ticket: QSL-1978; public PR filament-spec-language#3408. Native Codex reviewer; exact native model ID unavailable. No Jev result is claimed.

No Cargo, runtime test, full gate, baseline, workspace matrix or repository-wide scan was run. Current-head fifteen-case runtime qualification and final all-target custody are pending with the dispatching parent; historical receipts are not current-head passes. The parent reports relay Redis os11 prevented dispatch, with no enqueue receipt. Focused test compilation reported by parent is compilation evidence only. The reviewed branch was never edited or committed by this reviewer.

No applicable AssuranceProfile was found under spec. Direct, bounded review applies. Selected lenses are base, EARS, failure-domain and scope-boundary. Integrity, dependency and object review are not selected because no structural sections, relationships or domain objects changed; evidence analysis is not selected because no Verification method changed; risk-complexity is not selected because this adds no external technology/contract dependency. Criterion-strength is not run: no Jev client/result is available, and no new AC is authored. Optional whole-repository semantic gap review is not claimed; scoped code/test intent is part of code-review. Whole-matrix extraction previously failed by size and the dispatch forbids repeating it; this limitation cannot establish an engine-backed traceability PASS. Plan completion: not assessed.

## Verdict

FAIL — static defects require repair; runtime verdict pending.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Positive-policy preparation still traverses, deep-clones and builds storage outside the original meter: encode pre-scans states, instances_with_meter calls unmetered overlaps/conformance and bound classification, and canonical_policy builds unmetered clause letters/reference bindings. | fsl-eval/src/simulation/model.rs:2363-2388 |
| FND-002 | high | Product-key ordering and certificate monitor equality inspect policy bytes through derived Ord/raw Vec equality without ModelCandidate byte charges or cancellation polling; the frontier/checker counters do not account for this representation work. | fsl-analyze/src/plan_search.rs:797-805,1339-1341,1363-1367 |
| FND-003 | medium | Canonical reader native allocation failure is mapped to Invalid by the catch-all, losing its Allocation requested-byte payload and misclassifying resource exhaustion as malformed policy state. | fsl-eval/src/automaton/policy.rs:541-546 |

## Coverage

The Rust lane read rust-style API, implementation, safety/concurrency, testing and boundary references, repository AGENTS/CLAUDE/CONTRIBUTING/lints, all 16 changed files, shared transition owners, source letter/reference-binding callers, replay and certificate reconstruction. No CI/config/dependency-pin gate changes, vendored fixtures or source/test stubs were introduced. Existing SafetyState::read and BoundedKind helpers remain the transition owner. Terminal closure is constant work, treats k=0/exhausted windows correctly and reads no atom after a decided/exhausted window. Strict re-encoding against each expected context rejects alternate syntax, extra members, foreign context and framing/trailing changes; no history/local monitor ID enters positive keys. Model-only key limit remains separate.

FND-001 evidence: StepPolicy::encode at policy.rs482 scans every state with states.iter().any before it constructs Budget or makes its first charge; thus even its direct validation pass escapes actual structural visit accounting. EffectiveId in the actual a58474c exact pin is an owned Vec<OriginalDeclarationKey>, so parameter.clone/member.clone allocate strings and vectors. model.rs2367 calls overlaps after one population work event; overlaps2445-2449 calls StateModel::type_identity (cloned effective identity) and TypeEnvironment::conforms with further clones and unmetered comparisons. The unbounded path calls population_node_key before any node storage admission and check_requires_bound builds its own rooted Vec/classifier storage. SourcePlan::canonical_policy then calls clause_letters at plan_source.rs737; letters.rs190-236 clones effective/declaration IDs, reconstructs reference bindings and collects overlapping declarations. ReferenceBindings::with_temporal_clause341-398 allocates literal traversal sets/stacks and clones the entire binding map. This recurs at start, successor and closure; a large admitted type identity/reference map does real byte work/storage outside the original result ledger. Move this path onto the same budget with actual pre-growth owner hooks or borrow already admitted immutable data, preserving order/vacuity. NodeKey::clone itself is shallow Arc and is not alleged to deep-copy.

Existing owner seams: exact EffectiveId::try_clone_with exposes actual pre-reservation payload requests and owner_chain borrows declarations. Local StateModel::type_identity/type_key currently force clones. External semantic-value31d807c TypeEnvironment::conforms consumes owned IDs; conforms_ref and Ancestry::is_ancestor are private and expose no work/reservation callback. Its pending-assembly budget method uses a different stage budget and is not the runtime policy ledger. No complete public metered conformance seam was found; an external contract change requires the owner if borrowing/model-owned reuse cannot remove the operation. Do not duplicate conformance logic or guess prepaid clone sizes.

FND-002 evidence: NodeKey derived Ord compares ordered monitor vectors inside BTreeMap::get/insert. Certificate::check compares raw monitor vectors at certificate.rs368 and573. Copies are now metered but comparisons are not. A diamond joining equal model states with long equal policy contexts repeats policy-byte comparisons while only the ordinary planner frontier/checker count changes. Route policy equality/order through the original session meter before each actual inspected byte, retaining existing planner/checker count semantics.

FND-003 evidence: pinned quire-canonical2204c5c ReadError explicitly has Allocation { requested }. The catch-all drops it; use the existing typed allocation/encoding category while retaining the requested bytes. This finding is static branch tracing, not an injected allocation experiment.

## Review data

```yaml
clean: false
scope:
- id: FR-1011-policy-accounting
  path: spec/functional/FR-1011-meter-the-plan-search-under-named-settings.md
  role: examined
  excerpt: 'For canonical source-policy state construction and admission, `ModelCandidate`

    (`model.candidate`) SHALL charge actual structural-node and byte visits to

    `accounting.work_units` and full new buffer-capacity bytes, before every

    growth, to `accounting.result_units`; writer/reader scratch, output copies and

    retained monitor vectors share the original evaluation ledger and cancellation

    handle through search and the existing independent replay/checker sessions,

    without changing model-only `plan_search.max_state_key_bytes` or the existing

    automaton identity-byte bound.'
- id: FR-1012-staged-boundary
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: examined
  excerpt: 'The first positive-length source-policy increment covers capture-free,

    origin-activated infinite-trace `always holds(A)`, `eventually[0,k] holds(A)`

    and `always[0,k] holds(A)` through their canonical shared monitor states and

    terminal-stutter closure. Other checked positive-length shapes retain an

    explicit unsupported result until implemented. Captures, general liveness

    and FR-1012-AC-1 remain required subsequent increments; this staged boundary

    does not change their acceptance obligations.'
- id: FR-1011-AC-1
  path: spec/functional/FR-1011-meter-the-plan-search-under-named-settings.md
  role: context_only
  excerpt: 'A plan request whose `limits` (FSpec FR-805) is `{"max_work": 1}` stops the plan search `incomplete`
    naming `plan_search.max_work`, exit `22`; `{"max_states": 1}` names `plan_search.max_states`.'
- id: FR-1011-AC-2
  path: spec/functional/FR-1011-meter-the-plan-search-under-named-settings.md
  role: examined
  excerpt: A low `accounting.work_units` names `accounting.work_units`, not a `plan_search` setting.
- id: FR-1011-AC-3
  path: spec/functional/FR-1011-meter-the-plan-search-under-named-settings.md
  role: context_only
  excerpt: 'A plan request whose `limits` is `{"max_steps": 1}` refuses `invalid_runtime_input`/`unknown-member`
    at `/request/synthesis/limits/max_steps`, and `{"max_work": 0}` refuses `invalid-value` at `/request/synthesis/limits/max_work`,
    each exit 20 with no search; a request with no `limits` reports every `plan_search.*` limit at its
    published default in the outcome''s `limits`.'
- id: FR-1012-AC-1
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: examined
  excerpt: The spine plan under the policy `eventually holds(c.versionNumber = 3)` is evaluated, not refused,
    and every reported plan visits version 3.
- id: FR-1012-AC-2
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: examined
  excerpt: Two runs of one plan request give byte-identical outcome documents.
- id: FR-1012-AC-3
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: 'A decimal per-step cost objective is written exactly in FSpec FR-181''s typed `decimal` form
    (`{"type": "decimal", "coefficient": "30", "scale": "2"}` for `0.30`) and certified with a `potential`
    certificate.'
- id: FR-1012-AC-4
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: 'Over FSpec''s published `timed-tick` fixture the plan''s `elapsed` is `{"type": "rational",
    "numerator": "30", "denominator": "1", "unit": "min"}` with every `delay` of numerator 0 (FSpec FR-808-AC-1);
    over `timed-go` with guard `x >= 2` the result is `optimal` with one step of delay `{"type": "rational",
    "numerator": "2", "denominator": "1", "unit": "s"}` and an accepted `closure` certificate (FR-808-AC-2),
    and with guard `x > 2` `feasible` with `bound` `{side: lower, value: 2/1, attained: false}` and delay
    `{"type": "rational", "numerator": "5", "denominator": "2", "unit": "s"}`, byte-identica'
- id: FR-1012-AC-5
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: Over FSpec's published `probabilistic-try` fixture the goal `Reach.final.done` gives `optimal`
    `retry` (FSpec FR-809-AC-1), and `maximize Reach.probability` over lengths 1 to 2 using only `try`
    gives `optimal` `try, try` with `objective.value` the Rational `3/4` (FR-809-AC-2).
- id: FR-1012-AC-6
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: 'Over FSpec''s published `counter` fixture, `limits` `{"max_states": 2}` gives `incomplete`
    with `explored.states` 2 and `explored.work` 6 and plan `null`, and `{"max_work": 1}` gives `explored.states`
    1 and `explored.work` 1 (FSpec FR-801-AC-8), each byte-identical across two runs.'
- id: FR-1012-AC-7
  path: spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md
  role: context_only
  excerpt: Over a model whose invariant is undetermined at every state from some value up, a goal reachable
    only through such a state gives `incomplete` with cause `invariant-undetermined` naming the invariant,
    plan `null` and no certificate, never `infeasible`; a goal reached before the first such state gives
    `incomplete` carrying that plan, never `optimal`; and a plan substituted through such a state is not
    reported.
- id: fsl-analyze/src/plan_search.rs
  path: fsl-analyze/src/plan_search.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-analyze/src/plan_search/certificate.rs
  path: fsl-analyze/src/plan_search/certificate.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-analyze/src/plan_source.rs
  path: fsl-analyze/src/plan_source.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-analyze/tests/it/plan_search.rs
  path: fsl-analyze/tests/it/plan_search.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-command/src/command/plan_synthesis.rs
  path: fsl-command/src/command/plan_synthesis.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-eval/src/automaton/bounded.rs
  path: fsl-eval/src/automaton/bounded.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-eval/src/automaton/key.rs
  path: fsl-eval/src/automaton/key.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-eval/src/automaton/mod.rs
  path: fsl-eval/src/automaton/mod.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-eval/src/automaton/policy.rs
  path: fsl-eval/src/automaton/policy.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-eval/src/lib.rs
  path: fsl-eval/src/lib.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-eval/src/policy_budget.rs
  path: fsl-eval/src/policy_budget.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-eval/src/simulation/model.rs
  path: fsl-eval/src/simulation/model.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: fsl-eval/tests/it/automaton_key_bounded.rs
  path: fsl-eval/tests/it/automaton_key_bounded.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
- id: tests/it/plan_export_surface.rs
  path: tests/it/plan_export_surface.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later'
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-1011-policy-accounting
  related_ids:
  - FR-1011
  - FR-1012
  path: fsl-eval/src/simulation/model.rs
  lines: 2363-2388
  excerpt: 'For canonical source-policy state construction and admission, `ModelCandidate`

    (`model.candidate`) SHALL charge actual structural-node and byte visits to

    `accounting.work_units` and full new buffer-capacity bytes, before every

    growth, to `accounting.result_units`; writer/reader scratch, output copies and

    retained monitor vectors share the original evaluation ledger and cancellation

    handle through search and the existing independent replay/checker sessions,

    without changing model-only `plan_search.max_state_key_bytes` or the existing

    automaton identity-byte bound.'
  finding: 'Positive-policy preparation still traverses, deep-clones and builds storage outside the original
    meter: encode pre-scans states, instances_with_meter calls unmetered overlaps/conformance and bound
    classification, and canonical_policy builds unmetered clause letters/reference bindings.'
- fnd: FND-002
  severity: high
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-1011-policy-accounting
  related_ids:
  - FR-1011
  - FR-1012
  path: fsl-analyze/src/plan_search.rs
  lines: 797-805,1339-1341,1363-1367
  excerpt: 'For canonical source-policy state construction and admission, `ModelCandidate`

    (`model.candidate`) SHALL charge actual structural-node and byte visits to

    `accounting.work_units` and full new buffer-capacity bytes, before every

    growth, to `accounting.result_units`; writer/reader scratch, output copies and

    retained monitor vectors share the original evaluation ledger and cancellation

    handle through search and the existing independent replay/checker sessions,

    without changing model-only `plan_search.max_state_key_bytes` or the existing

    automaton identity-byte bound.'
  finding: Product-key ordering and certificate monitor equality inspect policy bytes through derived
    Ord/raw Vec equality without ModelCandidate byte charges or cancellation polling; the frontier/checker
    counters do not account for this representation work.
- fnd: FND-003
  severity: medium
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: fsl-eval/src/automaton/policy.rs
  related_ids:
  - FR-1011
  path: fsl-eval/src/automaton/policy.rs
  lines: 541-546
  excerpt: "            let document = document.map_err(|error| match error {\n                quire_canonical::ReadError::Limit(limit)\
    \ => {\n                    PolicyStateError::IdentityLimit { value: limit.bound }\n             \
    \   }\n                _ => PolicyStateError::Invalid,\n            })?;"
  finding: Canonical reader native allocation failure is mapped to Invalid by the catch-all, losing its
    Allocation requested-byte payload and misclassifying resource exhaustion as malformed policy state.
bindings: []
```
