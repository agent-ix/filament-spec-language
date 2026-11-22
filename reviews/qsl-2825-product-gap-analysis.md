---
id: SR-967175026302197768900474181941935084751
title: "QSL-2825 bounded protocol reachability gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3487; QSL-2825; protocol reachable search, caller budgets and cycle-key fixtures"
review_set: subset
date: 2026-11-22
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Frozen head: `4f0287b35a423aacbaa0a381cc59bc6fef7db3d7`. Ticket: QSL-2825. Read the existing repository instructions, Rust API/implementation/safety/testing/boundary conventions, changed code, all direct reachable callers, existing frontier Limit owner and the checker cycle-key seam. No applicable AssuranceProfile exists. No workflow, manifest, dependency or wire-format change occurs. No local build/test/repository executable or titan job ran in this review. `git diff --check` passed.

The supplied titan evidence at merged tree `fb7be2e72958ba8de9c8bf9e327cd66f4c721150` records all four exact-target compile-only commands, all 25 named runtime cases and the workspace compile gate passing. Six library search tests cover initial/duplicate keys, zero, exact closure, limit/error precedence, ordered workers and panic propagation; seven analyze cases cover the original concrete-channel cap, the five repaired footprint/commutation cases, and expansion refusals; ten evaluator consumers and two counter-enabled bench cases pass. The five repaired cases ran alone under hard watchdog/address-space/time limits; their peak RSS values were 180552, 187672, 98216, 99472 and 98640 KiB, all below 2 GB. The concrete cap-16 case used 91988 KiB. No module group or whole-binary acceptance is claimed: those runs remain prohibited by the lane policy. The excluded tc736 replay case is outside this change and was not run.

A planless repository matrix and coverage audit accompanied the changed behavior's reverse trace and stub inspection. The new direct search obligations bind or explicitly use Inspection. No new scoped gap or hollow assertion was found. Global extraction/coverage debt and the incomplete whole-repository reverse census prevent a repository-wide assurance PASS.

## Verdict

FAIL for repository-wide assurance because inherited high gaps remain. The reviewed QSL-2825 change is source-clear and its four new obligations are accounted for; the baseline finding does not direct this PR to repair unrelated criteria.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Repository assurance remains incomplete: the static matrix reports 1081 untagged and 68 ignored-only criteria with extraction diagnostics, and a whole-repository reverse inventory is not established. This is inherited baseline debt rather than a new scoped source defect; repository-wide PASS cannot be certified. | spec/spec.md:1 |

## Coverage

Reconciliation: static `quire matrix --scope /home/peter/dev/worktrees/qsl-2825-protocol-memory --format json` and `quire coverage --scope /home/peter/dev/worktrees/qsl-2825-protocol-memory --json`, quire 0.52.0 engine 0.62.2, both exit 0; no run evidence read by these commands. Native `quoin matrix --repo /home/peter/dev/worktrees/qsl-2825-protocol-memory --json`, /home/peter/.npm-global/bin/quoin 0.36.8, exits 1 with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`, reproducing the previously reported limitation. The documented static fallback succeeds; no native/runtime-backed matrix PASS is claimed.

Engine statuses: tagged 5023; untagged 1081; tagged-by-ignored-test 68; method-without-symbol 108; retired 15. Coverage reports 6211 untracked symbols and 7083 unmatched tags. Diagnostics: 6 section-matches-nothing, 1 archetype-matches-nothing, 22868 trace-id-outside-marker, 610 untracked-id-has-minted-children, 22 uncatalogued-verification-method and 1 catch-all-universal. Rust binding census: 14452 candidates, 13877 tagged, 13279 bound. These are engine totals, not a claim that every reported item was independently proven a new defect.

FR-126-AC-21 is method-without-symbol (Inspection); direct caller/API inspection fulfills this review step. AC-22 binds the initial/successor budget test. AC-23 binds the exact-cap test, reordered-worker refusal test and concrete/canonical integration regression. AC-24 binds the concrete unknown-delivery integration regression. These binders are actual production-path assertions, not mentions.

Plan completion: not assessed

Reverse inspection followed the public reachable API/refusal, canonical-key admission, frontier result ordering, worker join lifecycle, fixture key policy, measured counts and explicit caller budgets (eight behavior groups across ten changed files). All eight groups map to FR-126/101/205. No new unowned behavior, source stub, test stub, ignored assertion, copied implementation or coverage inflation was found in this inspected surface. A completed whole-repository behavior inventory and per-diagnostic adjudication were not established. Optional separate semantic gap expansion was not requested; requirement/code/test agreement was checked by the mandatory Rust review. Spec-review is inapplicable because the prerequisite requirement PR is already merged and this diff changes no spec or plan.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | Inherited repository-wide assurance backlog outside the assigned QSL-2825 product fix; the scoped direct search obligations have active binders or declared Inspection and passing named evidence. |

## Examined scope

- FR-126-AC-21 — spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; role: examined; excerpt: `ProtocolSystem::reachable(worker_limit, max_states)` requires an explicit `usize` state budget at every caller and reuses `simulation::frontier::Limit`; there is no overload or default that omits the budget.
- FR-126-AC-22 — spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; role: examined; excerpt: In the direct reachable-state search, a nonempty initial set with `max_states` 0 returns `ReachRefusal::Limit(Limit::States(0))`, and two distinct initial keys with budget 1 return the corresponding limit refusal before the second key is admitted. Repeated initial or successor keys consume one slot per distinct full canonical key. No limit refusal returns a partial successful graph.
- FR-126-AC-23 — spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; role: examined; excerpt: A finite direct protocol search with `n > 0` distinct reachable canonical states completes with exactly `n` states at budget `n`; at budget `n - 1` it returns `ReachRefusal::Limit(Limit::States(n - 1))`. With worker limits 1, 2 and 4 the limit refusal and its value are identical, including when later breadth-first expansions finish first.
- FR-126-AC-24 — spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; role: examined; excerpt: A concrete protocol system with an unordered Boolean channel declared `delivery unknown capacity 2 overflow block`, a send on that channel, and enabled duplicate and lose steps, using its concrete delivery indices without cycle-key canonicalization, returns `ReachRefusal::Limit(Limit::States(16))` from the direct reachable-state search with `max_states` 16 and worker limits 1 and 4. The run returns no partial successful graph; the test remains bounded even though repeated duplicate/lose steps can produce indefinitely fresh concrete keys.
- FR-101 — spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-fspec-sampler.md; role: context_only; excerpt: `max_states` and `max_transitions` are caller-raisable resource budgets
- FR-205 — spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md; role: context_only; excerpt: FSL SHALL build a temporal claim's **protocol subject**, a model subject
- spec/spec.md — spec/spec.md; role: context_only; excerpt: name: filament-spec-language

## Changed files examined

- `fsl-analyze/tests/it/fill.rs`
- `fsl-analyze/tests/it/protocol_footprints.rs`
- `fsl-analyze/tests/it/release_acquire.rs`
- `fsl-analyze/tests/it/weak_memory.rs`
- `fsl-bench/src/mc.rs`
- `fsl-eval/src/simulation/protocol/reach.rs`
- `fsl-eval/tests/it/protocol_duplicate_effect.rs`
- `fsl-eval/tests/it/protocol_observe_only.rs`
- `fsl-eval/tests/it/protocol_over.rs`
- `fsl-eval/tests/it/protocol_population.rs`
