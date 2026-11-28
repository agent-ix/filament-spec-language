---
id: SR-96721
title: "QSL-3400/QSL-3401 gap analysis of the changed criteria (filament-spec-language PR 3979)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@0b9befd5ad62ef2bc971fee598d8c7d6a299ce6c; criteria FR-120-AC-1, FR-149-AC-8, FR-245-AC-7, FR-245-AC-8, FR-245-AC-12"
review_set: subset
date: 2026-11-25
---

## Summary

Ticket: QSL-3401 (pointer on QSL-3400). Scoped to the criteria the diff
touches, from `quire trace --id`. Repo-wide matrix not recomputed.

- FR-120-AC-1: new test `an_observed_model_step_reads_its_operation_string`
  (TC-471) reads the identity bytes back; writer covered by relation_certificate
  HP-1 and outcome/settle wire tests.
- FR-149-AC-8: new `an_undefined_step_verdict_writes_a_state_locus` validates
  against `StepVerdict`; existing TC-556 tests cover the other kinds.
- FR-245-AC-7/8/12: `implements` on `components` and `refine_edge`; TC-700
  `the_locked_undo_post_state_is_a_locked_entry_node` validates the written
  zone step against `CertificateRejected`; TC-1197 tests cover backward-edge.

Stubs or coverage inflation: none. Tests with production change reverted
were seen failing by the author (PR body).

Plan completion: not assessed

## Verdict

PASS: every touched criterion has a test that ran at head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- Criteria examined: 5, all with tests.
- Semantic review: not run.
- Plan completion: not assessed
