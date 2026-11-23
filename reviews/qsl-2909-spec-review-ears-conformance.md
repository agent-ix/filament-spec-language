---
id: SR-967175026302197768900474181935084768
title: "EARS conformance review — QSL-2909 FSL provider reasons"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; FR-281 and FR-290 changed behavior and acceptance criteria"
review_set: subset
---

## Summary

Reviewed the new normative statements and acceptance criteria in FR-281 and FR-290 for obligation strength, subject, condition and singularity. The conditional behavior and its outcomes are stated clearly.

## Verdict

**PASS** — no new EARS conformance defect found.

## Assurance Context

No applicable `AssuranceProfile` exists in this repository's specification scope.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-290 provider reason behavior | examined | `For a refuted external-provider verdict, ProviderVerdict SHALL carry either mapped assignments ... or one typed unusable-counterexample reason` |
| FR-290 FSL settlement behavior | examined | `When FSL receives an unusable-counterexample reason, it SHALL settle the item inconclusive with cause replay-parity` |
| FR-290-AC-20 | examined | A refuted unusable external-provider counterexample produces the typed reason and item diagnostic. |
| FR-281-AC-24 | examined | The same engine result is settled without replay or counterexample/replay members. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
