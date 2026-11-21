---
id: SR-202611210022442313144108529
title: "integrity review of FR-1045 admission request member forms"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md with FR-100 as changed in the same PR; PR 3335 head 7a44bc4ef6081ca54b9c6a4f92cf28dc3c16721e"
review_set: subset
---

## Summary

Ticket: QSL-2740. Every cited id resolves to the intended requirement: FSL US-029, FR-038, FR-100, FR-100-AC-24, FR-106, FR-106-AC-74, FR-285, FR-944, FR-1039; FSpec FR-111, FR-181, FR-262, FR-287, FR-480; filament-observation FR-001, FR-014. The binding rulings from the brief all hold: full Producer 1.2 bundle, lowercase-hex identity, slots as a set, conflict over match, ResourceLimit as incomplete exit 22, an optional `reasons` array, incomplete-extent as a reason, and no correlation semantics in FSL. The ticket carries no arch-ruling comments, so the rulings were checked as stated in the brief.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-100 (same PR) writes `missing-valuation` `record` as lowercase hex; FR-1045 and the library keep it as exact text | spec/functional/FR-100-run-a-named-function-through-the-spine.md:411-412 |
| FND-002 | medium | FR-1045 says its member names follow FR-014 so a library record reads unchanged; the subject, authority and byte forms differ | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:102-106 |
| FND-003 | medium | A slot read from a result cannot be resubmitted unchanged; the closed slot form rejects its `authority` and `interface_version` | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:134-135 |
| FND-004 | low | "as added by FR-100-AC-24" is change-log wording | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:344 |

## Verdict

Fix round needed: one cross-requirement contradiction and two false interoperability claims.

## Scope and evidence

FSpec checked at filament-specification origin/main. Library checked at filament-observation baa30aa5.
