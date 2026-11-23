---
id: SR-3620002
title: "Integrity review of PR 3620 (filament-spec-language)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@736670854b8f6ecedc657733d1a1a9e02f782856; spec/functional/FR-027-export-compiled-native-package.md (call_site program rule, acceptance table, FR-027-AC-21); spec/functional/FR-1015-request-an-export-and-write-the-export-outcome.md (compile-only member bullet, FR-1015-AC-18); spec/test-cases/TC-3210 (step 5); spec/test-cases/TC-3211-an-export-request-refuses-a-compile-only-member.md"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: references
---

# Integrity review of PR 3620

## Summary

Ticket: QSL-2962. The new ids FR-1015-AC-18 and TC-3211 are unique at HEAD.
TC-3211 traces `verifies` FR-1015, and its scope is exactly AC-18. Its test
file, fsl-command/tests/it/cli_contract/export.rs, is named. The FR-1015
bullet cites FR-027, FR-665 and FR-810, the owners of the three members. The
FR-027 sentence, AC-21 and TC-3210 step 5 restate one rule consistently. The
table split loses no content.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The id TC-3211 is reused. On origin/main, reviews/qsl-2962-spec-review-base.md:20 has `relationships` target `ix://agent-ix/filament-spec-language/TC-3211`, and its text (lines 101, 127) and criterion-strength.md:43, 66 describe TC-3211 as the deleted missing-definition test case. After this PR that link resolves to the unrelated export test case, so the history reads wrong. TC-3212 is free on fresh origin/main. Candidate: renumber the new test case to TC-3212, or accept the reuse and note in TC-3211 that the id formerly named the deleted FR-286-AC-41 test case. | spec/test-cases/TC-3211-an-export-request-refuses-a-compile-only-member.md:2 |

## Verdict

Minor. One low bookkeeping finding.
