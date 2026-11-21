---
id: SR-550
title: "gap-analysis of QSL-2324 FSL owner cohort"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; Cargo.lock, fsl-command/Cargo.toml, fsl-command/tests/it/main.rs, fsl-command/tests/it/temporal_owner_cohort.rs; QSL-2324"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-931", type: references }
---

## Summary

Ticket: QSL-2324; PR: filament-spec-language#3332. Frozen head fbdb4642ecdd161b346e085b0fca8ec54650229d; base e01f365deff3ec3a283d9c30256a618bb51de90c. Architecture explicitly limits completion to dependency compatibility. No production request or observation authority construction is required. No applicable AssuranceProfile; no spec/plan changes, so spec-review and its sub-analyses do not apply.

Bounded changed-surface reverse-gap/stub inspection found no additional gap. This is an incomplete repository assurance audit, not an engine-backed PASS.

## Verdict

**CONDITIONAL** — Bounded changed-surface reverse-gap/stub inspection found no additional gap. This is an incomplete repository assurance audit, not an engine-backed PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Retained metadata-final.json resolves one identity each for filament-mltl 94f3eb20, tl-mltl 06cda777, tl-syntax 9f7b1c86 and filament-observation 1e356f99. Scoped library/it Clippy and new-module rustfmt pass. main.rs has only the new registration; removing it matches the saved baseline byte-for-byte, including its existing formatting discrepancy. Diff check passes. Three focused runtime tests remain compiling in consumer-tests.log; no runtime pass is claimed. No duplicate Cargo, aggregate gate, workspace or whole-crate suite was run.

Reconciliation: unavailable. Prior FSL matrix/coverage commands hung; dispatch explicitly forbids repeating them. No matrix counts, stale-tag census or engine PASS asserted. Whole-tree reverse-gap audit is outside the bounded authorization. The dependency-only production change introduces no new callable behavior; the two test functions execute real owner paths and contain behavioral assertions, with no stubs, vendored implementation, ignored tests, weakened CI or coverage changes. The incorrect FR-931-AC-5 binding is owned by SR-549 FND-001.

Plan completion: not assessed

Semantic review: skipped (not requested). No production source was edited. Owner-side API context was read from the selected dependency checkouts. The exact pinned cohort remains intentional even though owner main has advanced; future observation/bridge adoption is another lane's combined change.

## Examined scope

```yaml
scope:
  - {"id":"FR-931-AC-5","path":"spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md","role":"examined","excerpt":"Only `fsl-command`'s manifest among the FSL workspace crates names `tl-syntax` or `tl-mltl` (ADR-011)."}
  - {"id":"ADR-011","path":"spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md","role":"context_only","excerpt":"FSL `fsl-command` → `tl-syntax`, `tl-mltl`, `filament-mltl`"}
  - {"id":"Cargo.lock","path":"Cargo.lock","role":"examined","excerpt":"name = \"filament-mltl\""}
  - {"id":"fsl-command/Cargo.toml","path":"fsl-command/Cargo.toml","role":"examined","excerpt":"filament-mltl = { git = \"https://github.com/agent-ix/filament-mltl\", rev = \"94f3eb20927d70bbdc8af72e7b0333485e9c583f\" }"}
  - {"id":"fsl-command/tests/it/main.rs","path":"fsl-command/tests/it/main.rs","role":"examined","excerpt":"mod temporal_owner_cohort;"}
  - {"id":"fsl-command/tests/it/temporal_owner_cohort.rs","path":"fsl-command/tests/it/temporal_owner_cohort.rs","role":"examined","excerpt":"#[trace(\"FR-931-AC-5\")]"}
```

