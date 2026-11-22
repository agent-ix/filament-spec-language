---
id: SR-967175026302197768900474181941935084761
title: "QSL-2811 root evaluator dependency base review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR #3490; QSL-2811; FR-100-AC-7 and ADR-011 X-8 root evaluator dependency tables"
review_set: subset
date: 2026-11-22
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-100", type: references}
  - {target: "ix://agent-ix/filament-spec-language/ADR-011", type: references}
---

## Summary

Reviewed frozen head `fedda1012fee7d417d1bf07766640f9d41316712`. Scope is the two authorized dependency-table clarifications; the selected set is base plus EARS, integrity and failure-domain. No active AssuranceProfile document was found by its declared type, so no discovered profile requires an enforced review run.

## Verdict

PASS — no scoped findings. Both artifact IDs and criterion numbering remain unchanged and unique; the existing relative ADR section link resolves to 7.1 Target crate graph. FR-100 retains its US-014 and ADR-011 relationships. The changed dependency obligation is concrete and inspectable. Static trace extraction resolves FR-100-AC-7 with six structural verification claims; this is binding evidence, not test execution. Existing TC-390 asserts the relevant normal/dev/build boundary. No new criterion permutations, data inputs, outcomes, error paths, temporal states or wire contracts are introduced. The current root normal dependency remains a known product defect pending the separate authorized implementation.

This review certifies specification readiness only. The root normal dependency is not fixed by this PR, and no implementation or repository-wide coverage PASS is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verification

Installed Quoin 0.36.8 supplied the SpecReview skeleton and schema from spec-artifacts-process v0.36.0. Quire 0.52.0 (engine 0.62.2) validated the two edited documents: 2/2 grammar-clean, zero warnings. Whole-spec validation exited 0 with 2896/2962 documents grammar-clean and 148 inherited grammar findings; these do not establish a repository-wide clean assurance result. Static trace extraction for FR-100-AC-7 exited 0 and resolved six structural verification claims; they are not runtime evidence. Static Quire matrix extraction also exited 0 and reports the criterion tagged. Global matrix counts are method-without-symbol 108, retired 15, tagged 5020, tagged-by-ignored-test 68, untagged 1084; inherited gaps remain outside this two-clause review. The changed file list and `git diff --check` were inspected. No builds, tests, repository executables or titan jobs ran or were queued. Rust and production gap-analysis are inapplicable to this Markdown-only diff; no domain objects or relationship edges changed, so separate object/dependency analyses are not selected. Criterion-strength/Jev analysis was not selected. Existing unrelated pins and dependency architecture are outside the two changed clauses.

## Examined scope

- FR-100-AC-7 — spec/functional/FR-100-run-a-named-function-through-the-spine.md; role: examined; excerpt: `fsl_replay::spine::run` called directly over each AC-1, AC-4, AC-5 and AC-6 input returns the same outcome category, value, code, reason, counter, and parameter name or position `command`'s result document or refusal envelope carries for it. The root crate names `fsl-eval` in no normal or build dependency table (TC-390); a dev-dependency for tests is allowed ([ADR-011 §7.1](../decisions/ADR-011-stage-dag-and-dependency-architecture.md#71-target-crate-graph)).
- ADR-011/X-8 — spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; role: examined; excerpt: The root crate names `fsl-eval` in no normal or build dependency table, which TC-390 asserts; a dev-dependency for tests is allowed (§7.1).
- ADR-011/7.1 — spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; role: context_only; excerpt: A dependency needed only by tests is a dev dependency.
- TC-390 — tests/it/family_outcome_layering.rs; role: context_only; excerpt: the root crate's tests drive the temporal evidence call through a dev dependency
