---
id: SR-967191
title: "Gap analysis of QSL-3507 PR 4062"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 4062 at cd923f76fb65d5bcb13669be4f5968fb46eab096; fsl-analyze/src/protocol_analysis/conformance.rs, fsl-analyze/src/analyze/document/package_run.rs, fsl-eval/src/simulation/model.rs, fsl-analyze/src/protocol_analysis/finite_replay.rs, fsl-analyze/src/analyze.rs, fsl-analyze/tests/it/protocol_conformance.rs, fsl-analyze/tests/it/analyze_package_replay_conformance.rs, fsl-analyze/tests/it/model_instances.rs"
review_set: subset
---

## Summary

Ticket: QSL-3507. quire matrix on head, read against the criteria FR-896, FR-990 and FR-125 give this behaviour.

## Verdict

**FAIL** - untagged criteria and a hollow tag.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-896-AC-8 has no test | fsl-analyze/tests/it/protocol_conformance.rs:140-207 |
| FND-002 | high | FR-896-AC-9 has no test | fsl-analyze/tests/it/protocol_conformance.rs:140-207 |
| FND-003 | medium | FR-896-AC-7 is tagged but not asserted, and implements is put on a function that does not implement it | fsl-analyze/tests/it/protocol_conformance.rs:143-150 |
| FND-004 | low | FR-896-AC-10 is untagged | fsl-analyze/tests/it/protocol_conformance.rs:140-207 |

## Finding detail

**FND-001.** quire matrix reports FR-896-AC-8 untagged on head. It is the criterion for the bound-object half of the behaviour this PR changes; the code does not implement it (see code review FND-001).

**FND-002.** quire matrix reports FR-896-AC-9 untagged. A request whose subject.protocol binds another protocol is neither declined by the code nor tested.

**FND-003.** global_conformance_classifies_empty_over_as_no_instance is traced to FR-896-AC-7 (under TC-2800; the AC belongs to TC-2532) but uses neither the keyed order fixture, p/q, `check Ready`, the over=q counterexample nor closure.state, so the matrix counts AC-7 as covered. package_run.rs:2354 tags supplied_protocol `implements("FR-896-AC-7", ...)`; that function only resolves the item, and the package route does not check keys at all. The PR body lists no criteria.

**FND-004.** quire matrix reports FR-896-AC-10 untagged. The PR does not change the deadlock-item path; recorded because it is in the same requirement section.

## Coverage

Plan completion: not assessed

Semantic review: skipped. Matrix: FR-896-AC-8, AC-9, AC-10 untagged; FR-896-AC-7 tagged only by a test that does not assert it; FR-990-AC-11 and FR-125-AC-13 tagged. Reverse gap and stubs on the diff: package_global_behavior_item returns unsupported for every non-empty case (code review FND-004).
