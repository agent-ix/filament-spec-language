---
id: SR-967175026302197768900474181941935084742
title: "QSL-2825 reachable-state budget ears-conformance review"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; PR #3486; QSL-2825; FR-126 direct protocol reachability and AC-21 through AC-24"
review_set: subset
date: 2026-11-22
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-126", type: references}
---

## Summary

Reviewed frozen head `c76ea63648f5762c3d07bc741830b3b5ef80484e` against the accepted explicit state-budget decision. The selected review set is base plus EARS, integrity and failure-domain, as applicable to the scoped requirement statement and new criteria. No applicable AssuranceProfile was found in the repository; no enforced review run is required by a discovered profile.

## Verdict

PASS for the changed specification. No scoped findings. Product implementation and titan runtime acceptance remain subsequent work; this review does not certify either.

The added requirement uses the ubiquitous EARS form: named actor ProtocolSystem::reachable followed by one SHALL and concrete admission/refusal responses. The clauses are facets of one budget obligation, separated into observable ACs rather than independent unrelated features. Zero-budget and frontier-completion sentences define exact conditions without vague handling verbs. The edited FR-126 validates grammar-clean with zero engine warnings; inherited warnings elsewhere do not concern this diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verification

Installed Quoin 0.36.8 supplied the SpecReview skeleton/schema (spec-artifacts-process v0.36.0). Whole-spec Quire validation exited 0: 2896/2962 documents grammar-clean, 148 inherited grammar warnings; edited FR-126 alone is 1/1 grammar-clean with zero warnings. `git diff --check` passed. No build, test, repository executable or titan job was run. Static Quire matrix extraction exited 0; AC-21 is method-without-symbol (Inspection), and AC-22 through AC-24 are untagged, matching the explicit spec-first implementation sequence. These runtime gaps must close in the product PR; no repository-wide coverage PASS is claimed. Criterion-strength/Jev analysis was not selected. New AC test coverage is intentionally pending the authorized spec-first product PR.

## Examined scope

- FR-126 — spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; role: examined; excerpt: `ProtocolSystem::reachable` SHALL require a caller-supplied `max_states` budget, admit each distinct canonical state at most once including initial states, and before admitting a state beyond that budget return `ReachRefusal::Limit(Limit::States(max_states))` with no partial successful graph, independently of worker count.
- FR-126-AC-21 — spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; role: examined; excerpt: `ProtocolSystem::reachable(worker_limit, max_states)` requires an explicit `usize` state budget at every caller and reuses `simulation::frontier::Limit`; there is no overload or default that omits the budget.
- FR-126-AC-22 — spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; role: examined; excerpt: In the direct reachable-state search, a nonempty initial set with `max_states` 0 returns `ReachRefusal::Limit(Limit::States(0))`, and two distinct initial keys with budget 1 return the corresponding limit refusal before the second key is admitted. Repeated initial or successor keys consume one slot per distinct full canonical key. No limit refusal returns a partial successful graph.
- FR-126-AC-23 — spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; role: examined; excerpt: A finite direct protocol search with `n > 0` distinct reachable canonical states completes with exactly `n` states at budget `n`; at budget `n - 1` it returns `ReachRefusal::Limit(Limit::States(n - 1))`. With worker limits 1, 2 and 4 the limit refusal and its value are identical, including when later breadth-first expansions finish first.
- FR-126-AC-24 — spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; role: examined; excerpt: A concrete protocol system with an unordered Boolean channel declared `delivery unknown capacity 2 overflow block`, a send on that channel, and enabled duplicate and lose steps, using its concrete delivery indices without cycle-key canonicalization, returns `ReachRefusal::Limit(Limit::States(16))` from the direct reachable-state search with `max_states` 16 and worker limits 1 and 4. The run returns no partial successful graph; the test remains bounded even though repeated duplicate/lose steps can produce indefinitely fresh concrete keys.
- FR-126-AC-8 — spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; role: context_only; excerpt: `ModelCheckLimits::default()` has `limits` equal to FR-101's `Limits::default()` (`max_states` 10,000,000, `max_transitions` 100,000,000) and `max_automaton_states` 1,048,576, with no depth member; a request with `max_depth: None` sets no horizon, `max_live_instances: None`, so FR-209's published default applies, and `max_store_buffer` and `max_messages` `None`, so FR-225's published defaults apply. AC-1's requests with the default limits return the AC-1 outcomes. A run stopped by `max_states` 2 with the other members at their defaults returns `Stopped{ResourceExhausted, {MaxStates, 2}}`, nami
- US-015 — spec/usecase/US-015-check-a-temporal-property-over-every-behaviour-of-a-model.md; role: context_only; excerpt: **So that** I get `proved` when no behaviour violates it, a counterexample I can replay when one does, and a stated reason when the check cannot decide, and so that a state where the model gets stuck by mistake is reported to me.
