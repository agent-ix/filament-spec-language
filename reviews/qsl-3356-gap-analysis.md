---
id: SR-2026112601100002
title: "Gap analysis — filament-spec-language PR #3997 observation refusal loci"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3997 head 48a91b77c05386368d69af981f31493ca6709f5d; criteria FR-663-AC-3, FR-663-AC-5, FR-663-AC-6, FR-056-AC-43, FR-106-AC-85, FR-106-AC-86"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-663", type: references }
---

## Summary

Ticket: QSL-3356 (partial, Refs). A gap audit scoped to the PR. Tags were found by grep and by reading each test's code path; the repository-wide matrix and reverse-gap inventory were not run, so this is not a full repository gap audit.

## Verdict

**PASS** for the partial scope. Items 1 to 6 of QSL-3356 are delivered on the admission record and in the replay locus. Each new or changed criterion has a tagged test that asserts literal pointers, members and causes. What stays open is stated in the PR and on the ticket: an `Invocation`/`PreCall` refusal reaches no outcome document, some dangling and 6.1/6.2 refusals have no `path`, and the unit-bound pointer stops at the field. Spec rows FR-106-AC-85 and FR-106-AC-86 match their tests (`native_snapshot_values.rs` three TC-465 tests; `machine_contexts.rs::a_specialized_machine_operation_refuses_beyond_its_unit_bound`), and `check_argument_bounds` carries `#[implements]` for both. The unit-bound gaps are recorded on QSL-3437 (SR-2026112601100001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
