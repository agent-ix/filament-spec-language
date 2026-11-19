---
id: SR-2026111612172400000010
title: "scoped gap analysis of PR #2794 (QSL-2323): hyper divisor refusals"
type: SpecReview
analysis: gap-analysis
date: 2026-11-16
scope: "agent-ix/filament-spec-language; QSL-2323; PR #2794 at a01bac80b3ef039afa72d1abe9a314dfa7c0b9a4; fsl-semantics/src/check/hyper_clause.rs and seven changed hyper test/support files"
review_set: subset
relationships:
  - { target: ix://agent-ix/filament-spec-language/FR-487, type: references }
  - { target: ix://agent-ix/filament-spec-language/FR-495, type: references }
---
# SR-2026111612172400000010: scoped gap analysis of PR #2794

## Summary

Ticket: QSL-2323. Scoped to the seven changed files and their declared-clause refusal behavior. The changed tests cover tuple and projected atoms, universal and existential match paths, witness ordering, and refusal before horizon or projection evaluation. The PR's parked replay and Kernel-body cases are excluded from this review scope and are not counted as covered. No Cargo commands were run.

The modified `hyper_lowering` test loses the runtime-undefined behavior it formerly checked while retaining that criterion's trace tag. The separate code review records the untested multiple-divisor span case.

## Verdict

**CONDITIONAL** for the requested diff scope. This does not assert repository-wide traceability, ticket completion, or aggregate-suite acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The changed S3-refusal test no longer discharges the FR-487-AC-3 runtime-undefined clause it still claims. | fsl-analyze/tests/it/hyper_lowering.rs:155-164 |

## Coverage

Compared `origin/main...HEAD` at `a01bac80b3ef039afa72d1abe9a314dfa7c0b9a4`. Production source is limited to the hyper refusal-to-cause mapper. Six test modules update declared-clause/VC expectations; the shared vault fixture and tuple fixture add S3-refusal helpers. No spec, plan, CI, dependency, ignore-marker, or gate changes are present. FR-487-AC-3 was inspected for test intent; FR-495-AC-3 was read as context for Linked nonzero refusal. The source behavior and all changed tests were checked at the changed-test level. The optional broader intent↔test↔code analysis was not run.

The 15 changed tests in the PR body are reported as focused S3 refusal coverage. The reported test commands and results are attributed to the PR author and were not rerun. Seven replay cases remain parked for QSL-2330, and Kernel function-body cases remain parked for QSL-2329. The unchanged empty-set case contains no divisor in its declared clause. None of these parked cases contributes to this review's coverage conclusion.

Matrix context: `quoin matrix --repo . --json` completed with the installed Quoin 0.29.4. Its repository-wide JSON is large and was not used to claim a complete repository audit or to infer that the PR's parked work passed. The scoped review identified the local test-to-criterion mismatch and untested multiple-divisor span selection described above.

Plan completion: not assessed.

## Examined units

| ID | Path | Role | Excerpt |
| --- | --- | --- | --- |
| FR-487-AC-3 | spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md | examined | Over the secure vault, the atom returns completed true/false readings and the unproved-divisor atom returns `Undefined`; zero budget is incomplete, invalid slot count faults without charge, and equal calls have equal evaluations and charge logs. |
| FR-495-AC-3 | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md | context_only | Linked source with a divisor that may be zero refuses `undefined_expression/unproved-nonzero` at the operator; a proved nonzero divisor checks. |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The refusal-only test now has the corrected trace tag `#[trace("FR-495")]`, rather than claiming FR-487-AC-3's runtime `Undefined` assertion. |

