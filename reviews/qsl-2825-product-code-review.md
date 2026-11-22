---
id: SR-967175026302197768900474181941935084750
title: "QSL-2825 bounded protocol reachability code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3487; QSL-2825; protocol reachable search, caller budgets and cycle-key fixtures"
review_set: subset
date: 2026-11-22
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Frozen head: `4f0287b35a423aacbaa0a381cc59bc6fef7db3d7`. Ticket: QSL-2825. Read the existing repository instructions, Rust API/implementation/safety/testing/boundary conventions, changed code, all direct reachable callers, existing frontier Limit owner and the checker cycle-key seam. No applicable AssuranceProfile exists. No workflow, manifest, dependency or wire-format change occurs. No local build/test/repository executable or titan job ran in this review. `git diff --check` passed.

The supplied titan evidence at merged tree `fb7be2e72958ba8de9c8bf9e327cd66f4c721150` records all four exact-target compile-only commands, all 25 named runtime cases and the workspace compile gate passing. Six library search tests cover initial/duplicate keys, zero, exact closure, limit/error precedence, ordered workers and panic propagation; seven analyze cases cover the original concrete-channel cap, the five repaired footprint/commutation cases, and expansion refusals; ten evaluator consumers and two counter-enabled bench cases pass. The five repaired cases ran alone under hard watchdog/address-space/time limits; their peak RSS values were 180552, 187672, 98216, 99472 and 98640 KiB, all below 2 GB. The concrete cap-16 case used 91988 KiB. No module group or whole-binary acceptance is claimed: those runs remain prohibited by the lane policy. The excluded tc736 replay case is outside this change and was not run.

The new admission check follows full canonical-key lookup and precedes insertion, so duplicates consume no slot, initial and successor states share the budget, and exactly closed graphs succeed at their cap. The existing `frontier::Limit` is reused without an API default. Ordered per-frontier results let the admission phase select an earlier cap refusal ahead of a later expansion refusal independently of worker completion order. All expansion workers are joined; the pre-existing documented panic propagation is retained. No new unchecked input indexing, unsafe code, wire conversion, ambient dependency, copied implementation or stub was found.

The graph's admitted-state budget is not a strict byte/RSS cap: a level still materializes bounded expansion results before admission. This is the existing expansion architecture and does not contradict FR-126's distinct admitted-state contract; measured RSS is separate evidence for the five repaired fixtures. Every direct caller explicitly chooses FR-101's published state budget. The unrelated automaton and closure-certificate methods named reachable are not callers of this API.

The concrete-channel regression keeps uncanonicalized delivery indices and asserts enabled send, duplicate and lose steps before exact refusal at 16 states on workers 1 and 4. Its canonical counterpart tests exact closure at 5 and refusal at 4 on workers 1, 2 and 4. The five formerly runaway fixtures select the existing checker cycle policy, assert the measured graph sizes, and retain their original commutation/footprint assertions. There is no new identity abstraction or reconstructed world.

## Verdict

PASS for this scoped code/Rust review: no scoped findings. The repository-wide gap audit has separate inherited limitations; this conclusion does not certify whole-repository assurance or the prohibited module/binary memory runs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

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
