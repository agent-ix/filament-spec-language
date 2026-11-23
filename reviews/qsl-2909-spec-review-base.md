---
id: SR-967175026302197768900474181941935084767
title: "Base specification review — QSL-2909 FSL provider reasons"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-281, FR-286, FR-290, FR-621; spec/test-cases/TC-774"
review_set: subset
---

## Summary

Reviewed the changed requirement and test artifacts for verifiability, coverage and cross-reference consistency. One medium coverage gap remains: the newly specified precedence for simultaneous mapping defects is not exercised.

## Verdict

**CONDITIONAL** — FR-290 defines deterministic selection when multiple defects apply, but its acceptance criterion and test procedure only exercise each defect separately.

## Assurance Context

No applicable `AssuranceProfile` exists in this repository's specification scope.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-290 behavior | examined | `If multiple defects apply, the selected reason SHALL be the first in this order: unreadable package graph, unresolved node id, assignment to a non-parameter, missing claim-parameter assignment, then parameter value-kind mismatch.` |
| FR-290-AC-20 | examined | `A refuted external-provider verdict whose counterexample is unusable for one of the five reasons above yields exactly that closed reason in ProviderVerdict` |
| TC-774 step 15 | examined | `exercise each FR-290-AC-20 unusable-counterexample reason` |
| TC-774 expected result step 15 | examined | `each unusable mapping produces its matching closed reason` |
| FR-281-AC-24 | examined | `When an external provider reports a refuted verdict with each FR-290 unusable-counterexample reason in turn` |
| FR-621-AC-18 | examined | `A provider verdict carrying each FR-290 unusable-counterexample reason` |
| FR-286 analyze item members | examined | `optional item-level diagnostics` |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Deterministic precedence for simultaneous mapping defects has no acceptance assertion | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:519; spec/test-cases/TC-774-plugin-results-settle-as-typed-records-and-plugin-proofs-carry.md:38 |

The test procedure asserts each unusability cause in isolation. It does not provide two simultaneous defects or assert that the higher-priority reason wins, nor does it assert first-assignment or first-parameter selection for same-class defects. The ordering contract can therefore regress without failing this criterion. Add competing-defect and stable-order cases to FR-290-AC-20 and TC-774.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | Same-class assignment and missing-parameter tie-breaks cannot be verified from the closed reason output | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:522; spec/test-cases/TC-774-plugin-results-settle-as-typed-records-and-plugin-proofs-carry.md:38 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-290-AC-20 and TC-774 now specify and exercise pairwise precedence plus assignment and parameter order cases. |
| FND-002 | still-open | This same-class tie-break is not externally observable because the output contains only the reason category; reordering assignments cannot verify which assignment was selected. Remove the unobservable tie-break contract and its assertions, or expose a permitted distinguishing value. |
| FND-002 | fixed | FR-290 removes same-class selection requirements; TC-774 now verifies the observable cross-class precedence with reordered inputs. |
