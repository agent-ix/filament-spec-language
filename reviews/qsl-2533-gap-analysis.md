---
id: SR-2026111911313600000004
title: "Gap analysis — FSL hybrid wire read-back and finite end replay"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/hybrid_requests.rs, fsl-replay/src/proof_result/plugin_read/trace.rs, fsl-replay/src/hybrid_replay/exact.rs, fsl-replay/src/hybrid_replay/static_replay.rs, fsl-replay/src/hybrid_replay/tests.rs, fsl-replay/src/result/wire.rs; FR-750-AC-9, FR-750-AC-14, FR-193-AC-23, FR-450-AC-10, TC-3037"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/spec", type: references}
---

## Summary

A repository matrix and manual changed-criteria-to-test review covered undefined counterexample read-back, irrational intervals, and finite-end replay. The applicable Quoin gap-analysis command is unavailable in installed Quoin 0.36.7; the computed matrix shows all four in-scope criteria tagged and with no run evidence, while the AC-14 test leaves a parameter-index case uncovered and the implementation decodes it incorrectly.

## Verdict

**CONDITIONAL** — AC-14's test passes only for the parameter-zero call shape and does not expose the valid-root-call/nonzero-parameter read-back defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-14's test does not distinguish call path from argument index | FR-750-AC-14, fsl-replay/src/hybrid_replay/tests.rs::settlement::the_four_undefined_causes_round_trip_through_wire_read_back |

## Coverage

- Reconciliation: `quoin matrix --repo . --json` (quoin 0.36.7; quire matrix included) and `quire coverage --scope . --json`.
- In-scope static statuses: FR-750-AC-9, FR-750-AC-14, FR-193-AC-23 and FR-450-AC-10 are tagged; the quoin evidence axis reports `no run evidence` for each.
- Matrix-wide context: 4,323 tagged, 262 tagged only by ignored tests, 1,174 untagged and 104 method-without-symbol criteria; no run evidence across 5,876 criteria. These are repository baseline counts and were not attributed to this PR.
- Plan completion: not assessed
- Semantic review: skipped (not opted in).
- Manual check: the AC-9 analyze test derives an irrational undefined instant from exact replay, writes and reads its wire counterexample, and asserts a reproduced result; the AC-14 unit test serializes and reads the four causes, but parameter case uses only parameter 0 and a call at path `[0]`. The endpoint test covers exact replay, static check 6, and end-plus-loop refusal directly, with an activated analyze test for end read-back and outcome settlement.
- Quoin gap-analysis unavailable: `quoin gap-analysis --help` exited 2 with `command gap-analysis not found`; command inventory contains no `gap-analysis`. Manual AC-to-test review and the available matrix were used. The full matrix reports existing untagged criteria outside this ticket scope.

## Rationale

| Finding | Requirement and evidence |
| --- | --- |
| FND-001 | FR-750-AC-14 requires read-back of the exact function parameter for `outside-parameter-range`. The tagged test's parameter-0 call at structural path `[0]` makes the current implementation's incorrect `locus.term.term.last()` derivation pass. It does not cover a root call (empty path) or a nonzero parameter; the reader refuses the former and can mislabel the latter. |
