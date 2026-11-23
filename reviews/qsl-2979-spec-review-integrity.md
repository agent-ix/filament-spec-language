---
id: SR-3603002
title: "Integrity review of the on-each reachability conditional profile (filament-spec-language PR 3603)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@32ca5b98116c0ca890097e2604f1e680ed721a1f; spec/functional/FR-201-check-a-probability-certificate.md; spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md; spec/test-cases/TC-3208-an-on-each-reachability-ratio-document-is-checked-per-instance.md"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-3208
    type: references
---

# Integrity review of the on-each reachability conditional profile (PR 3603)

## Summary

Ticket: QSL-2979. Structural checks: `quire validate --scope .` on the three
changed files exits 0. TC-3208 was free on origin/main, and TC-3207 exists.
The TC follows the repo's TC shape (Description with Scope, Test Procedure,
Expected Results, a `verifies` FR-201 relationship). Its trace tag
`#[trace("TC-3208", "FR-201-AC-76")]` follows the `tools/fsl-probability-check`
convention. The test file `tests/it/on_each.rs` is added by code PR #3599.
FR-201-AC-76 is unique and points at TC-3208. No spec file still spells the
FR-875 analyze cause `{"type": "no-instance"}`. FR-290-AC-9's
`{"type":"no-instance","depth":1}` belongs to the plugin reader's FSpec
FR-331 wire, which is correct there. Contradictions with FSpec FR-412, and
the AC-76 contradiction, are in SR-3603001 and are not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The coverage locus `Malformed` at `values` is not in the FR-201 rejection table. The table's Malformed `CertificateMember` list names `numerator` and `denominator` but not `values` for a ratio part. FR-412 makes a malformed member locus carry no `part`, so `values` cannot say which part is incomplete. Add the profile's coverage row to the table, or reject at `numerator` or `denominator` (the incomplete part's member), so the locus names the part. | spec/functional/FR-201-check-a-probability-certificate.md:885-892 |

## Verdict

Structurally clean apart from FND-001. The ids are free and unique, quire
validate is clean, the trace tag follows convention, and no stale
`type`-spelled analyze cause remains in spec/.
