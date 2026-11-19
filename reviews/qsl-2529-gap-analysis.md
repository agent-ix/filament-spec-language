---
id: SR-2026111919340478491681418
title: "gap-analysis of QSL-2529 exact replay of non-deterministic reset fairness"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2529; PR3119 head 28af05612194b1fcca6fb378909e51a3933a6cfb; FR-450 :149-157, :204-238; FR-750-AC-12; TC-3037; fsl-replay/src/hybrid_replay/**; fsl-analyze/tests/it/hybrid_requests.rs"
review_set: subset
---

## Summary

Ticket: QSL-2529 (slice of QSL-2234), PR filament-spec-language#3119. Diff-scoped manual AC-to-test and requirement-to-code check of the non-deterministic reset fairness half of FR-450 "Exact replay" and FR-750-AC-12 (b). Plan completion: not assessed. No whole-repository matrix was computed.

## Verdict

CONDITIONAL. FR-750-AC-12 (b)'s three Shuttle fairness variants are exercised on both paths: `shuttle_fairness_over_a_nondeterministic_reset_is_decided_exactly` (fsl-replay, exact `Reproduced`/`NotReproduced { Claim }` outcomes, hand-derivable) and `exact_replay_decides_nondeterministic_reset_fairness_claims` (fsl-analyze, settlements). Pulse's weak/strong test discriminates "never enabled" from "enabled at one irrational instant". The activation half (FR-750-AC-12 (a), FR-450 :204-223) stays `#[ignore]` with an honest reason citing QSL-2529. The `CoupledReset` deviation from FR-450 :224-234/:235-238 is tracked by QSL-2639. FR-450 :224-234 has no FR-450 AC of its own; FR-750-AC-12 (b) is its only binding (see FND-003 of the code-review SR).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The fsl-analyze unfair branch asserts only `inconclusive` and cause `replay-parity`, which a `NotExact` replay also produces; it does not check the disagreement is `hybrid-trace`/`not-reproduced`, so a regression to `NotExact` passes | fsl-analyze/tests/it/hybrid_requests.rs:1190-1195 |
