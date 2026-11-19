---
id: SR-2026111612172400000053
title: "Gap analysis — verification plan S3 declaration checking"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository Test Matrix and verification-plan implementation; QSL-2508 / FR-753-AC-1 through FR-753-AC-5"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-753
    type: references
---

## Summary

Ran the computed Quoin matrix over the repository and traced the FR-753 criteria to tests and changed code, with focused attention on QSL-2508 / FR-753-AC-2. AC-2 is tagged by active tests and its focused test filter passed, but those tests do not discharge the required source-span and cycle-edge assertions; other FR-753 criteria remain ignored or untagged outside this S3 ticket.

## Verdict

**FAIL** — the full matrix has untagged criteria, including FR-753-AC-5; AC-2 also has a medium evidence gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-753-AC-2's active tests omit assertions for required refusal spans and cycle edges | tests/it/synthesis_verify_check.rs:123 |
| FND-002 | high | FR-753-AC-5 is untagged, so its plan execution behavior has no traced test | spec/functional/FR-753-build-check-emit-and-execute-a-verification-plan.md:134 |

FR-753-AC-5 has no binder in the computed matrix. The criterion specifies observable plan execution, engine negotiation, dispositions, blocking, and package identity refusal, but no test carrying its criterion id verifies those obligations. This is an uncovered acceptance criterion in the audited FR-753 scope.

## Coverage

- **Matrix:** `quoin matrix --repo /home/peter/dev/worktrees/qsl-2508 --json` (Quoin 0.36.7) completed. Across 5,825 criteria: 4,281 tagged, 1,145 untagged, 283 tagged only by ignored tests, 103 method-without-symbol, and 13 retired. All report `no run evidence` in the computed matrix; this static evidence field does not incorporate the focused command below.
- **FR-753:** AC-1 tagged-by-ignored-test; AC-2 tagged by four active test functions; AC-3 and AC-4 tagged-by-ignored-test; AC-5 untagged. AC-1/3/4/5 are adjacent follow-on scope, not changes required by QSL-2508.
- **AC-2 trace check:** its cycle, positive, duplicate/method, and reference/bound tests all trace the AC. Test bodies establish expected catalog codes and a successful positive case but do not compare source spans; the cycle test does not compare `DefinitionCycle.edges`. The stray edge assertion is inside a separately ignored synthesis test and is inconsistent with that fixture.
- **Focused execution:** `cargo test --locked -p filament-spec-language --test it synthesis_verify_check::` through `locked-build.sh`, with debuginfo/incremental disabled and a 16 GiB address-space cap — 4 passed, 0 failed, 3 ignored, 849 filtered out.
- **Reverse gaps and stubs:** inspected all changed source files and their S3 call sites; the new verification-plan logic is explicitly tied to FR-753/FSpec FR-192, and no TODO/FIXME/placeholder stub was found in the changed logic. No repository-wide source-symbol ownership inventory was generated beyond the computed matrix.
- **Semantic review:** skipped (optional; not requested).
- **Plan completion: not assessed.**
