---
id: SR-967175026302197768900474181941935084690
title: QSL-1978 positive policy spec-review-scope-boundary
type: SpecReview
analysis: scope-boundary
scope: agent-ix/filament-spec-language; PR3408 QSL-1978; owned diff b68f04c445396b1e66fa71063337d001f3717dcd..df2e5e322f2bf44fe0a401d1ae69bafd95cfac13
review_set: subset
---

## Summary

Frozen candidate: `df2e5e322f2bf44fe0a401d1ae69bafd95cfac13`. Base: `b68f04c445396b1e66fa71063337d001f3717dcd`. Ticket: QSL-1978; public PR filament-spec-language#3408. Native Codex reviewer; exact native model ID unavailable. No Jev result is claimed.

No Cargo, runtime test, full gate, baseline, workspace matrix or repository-wide scan was run. Current-head fifteen-case runtime qualification and final all-target custody are pending with the dispatching parent; historical receipts are not current-head passes. The parent reports relay Redis os11 prevented dispatch, with no enqueue receipt. Focused test compilation reported by parent is compilation evidence only. The reviewed branch was never edited or committed by this reviewer.

No applicable AssuranceProfile was found under spec. Direct, bounded review applies. Selected lenses are base, EARS, failure-domain and scope-boundary. Integrity, dependency and object review are not selected because no structural sections, relationships or domain objects changed; evidence analysis is not selected because no Verification method changed; risk-complexity is not selected because this adds no external technology/contract dependency. Criterion-strength is not run: no Jev client/result is available, and no new AC is authored. Optional whole-repository semantic gap review is not claimed; scoped code/test intent is part of code-review. Whole-matrix extraction previously failed by size and the dispatch forbids repeating it; this limitation cannot establish an engine-backed traceability PASS. Plan completion: not assessed.

## Verdict

PASS — bounded method only; limitations below apply.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

FR-1011 accounting is cross-cutting runtime resource admission owned by FSL policy construction/instance collection and established evaluation sessions, consuming filament-exact Meter/Cancel and quire-canonical reservation callbacks. The shared owner ModelCandidate clarification is explicitly approved and landed as docs-only exact PR192; the existing pin/API is unchanged. FR-1012 policy selection/transition/closure is FSL domain logic consuming shared canonical safety/bounded identity and native source atom evaluation. Canonical equality is verified by authored conformance tests; actual runtime is pending. Producer capacity hooks are relied upon as their published contract, not independently requalified by this review. No new external system or setting is introduced. General liveness/captures remain required later and are not allocated away. The existing external metered-conformance gap is a repair boundary documented in code-review rather than invented local semantics.

## Review data

```yaml
clean: true
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
findings: []
bindings: []
```
