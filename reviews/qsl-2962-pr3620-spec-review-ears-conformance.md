---
id: SR-3620004
title: "EARS review of the FR-1015 compile-only member bullet (filament-spec-language PR 3620)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language@736670854b8f6ecedc657733d1a1a9e02f782856; spec/functional/FR-027-export-compiled-native-package.md (call_site program rule, acceptance table, FR-027-AC-21); spec/functional/FR-1015-request-an-export-and-write-the-export-outcome.md (compile-only member bullet, FR-1015-AC-18); spec/test-cases/TC-3210 (step 5); spec/test-cases/TC-3211-an-export-request-refuses-a-compile-only-member.md"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: references
---

# EARS review of the FR-1015 compile-only member bullet

## Summary

Ticket: QSL-2962. The changed FR-027 sentence keeps its If-then form (it
adds only a parenthetical). The new FR-1015 bullet is passive, and
`quire validate` flags it: "line 108: [quality:agentless-passive] `shall be
refused` names no agent".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-1015 bullet "An export request that carries a compile-only top-level member ... SHALL be refused `invalid-request` naming that member, exit `20`, and writes no file" is passive, names no agent, and mixes a SHALL with an indicative "writes". Candidate: "If an export request carries a compile-only top-level member, `output`, `handoff` or `call_site` (FR-027, FR-665, FR-810), then FSL SHALL refuse it `invalid-request` naming that member, exit `20`, before it runs the export or writes any file; the export names its file in `export.output`." | spec/functional/FR-1015-request-an-export-and-write-the-export-outcome.md:108 |

## Verdict

Minor wording.
