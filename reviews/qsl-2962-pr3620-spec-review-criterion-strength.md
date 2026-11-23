---
id: SR-3620003
title: "Criterion-strength review of FR-1015-AC-18 and FR-027-AC-21 (filament-spec-language PR 3620)"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language@736670854b8f6ecedc657733d1a1a9e02f782856; spec/functional/FR-027-export-compiled-native-package.md (call_site program rule, acceptance table, FR-027-AC-21); spec/functional/FR-1015-request-an-export-and-write-the-export-outcome.md (compile-only member bullet, FR-1015-AC-18); spec/test-cases/TC-3210 (step 5); spec/test-cases/TC-3211-an-export-request-refuses-a-compile-only-member.md"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: references
---

# Criterion-strength review of FR-1015-AC-18 and FR-027-AC-21

## Summary

Ticket: QSL-2962. No Jev client is on this host, so this was done by hand.
FR-1015-AC-18 is falsifiable. Accepting the member, refusing with another
code or exit, writing a file, or running the export each fail it. All three
members are covered. FR-027-AC-21 now has separate one-unit and two-unit
array cases, so either reading of "array" is decidable (its soundness is
SR-3620001 FND-001). Weakness kind: `partial` for AC-18.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-18 does not name its base request ("An export request that also carries ..."). The code reads and validates the `export` member (export.rs:218 `request::read`) and checks a transfer conflict (:203-212) before the compile-member check (:219-226). So a base request with an invalid `export` member, or a `refinement_transfer`, refuses for that reason first, and a test of it cannot tell a missing compile-member check from a working one. TC-3211 says "a valid export request", but the AC does not. Candidate: "FR-1015-AC-1's request (exporting `SlowEnough` over `Braking`), with top-level `call_site` added, refuses ...". | spec/functional/FR-1015-request-an-export-and-write-the-export-outcome.md:229 |

## Verdict

Partial. Strong enough once the base request is named.
