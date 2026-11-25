---
id: SR-2026112506280002
title: "Gap analysis — filament-spec-language QSL-3311 intake mapping"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; spec/, fsl-semantics/src/model/intake.rs, fsl-semantics/tests/it/model_intake.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Ran the repository-level matrix and coverage extraction for the QSL-3311 review. The change's three tests bind to `FR-056-AC-5`, and that criterion is tagged; the whole repository matrix still contains substantial pre-existing traceability and extraction gaps, so repository assurance does not pass.

## Verdict

**FAIL** — the computed repository matrix has 1,025 untagged criteria and 26 criteria tagged only by ignored tests; coverage also reports 6,579 unmatched test tags. These are repository-wide results and are not attributed to this diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed matrix contains 1,025 untagged criteria and 26 criteria whose only tagged tests are ignored, leaving live requirements without executable test evidence. | quire matrix: 1,025 untagged; 26 tagged-by-ignored-test |
| FND-002 | medium | Coverage extraction reports 6,579 untracked test symbols/tags, so those tags do not bind to declared criteria. | quire coverage: untracked_symbols=6579 |
| FND-003 | high | The whole-source reverse-gap inventory and source/test stub scan were not completed; only the changed intake path and its tests were inspected, so this is not a complete repository gap audit. | fsl-semantics/src/model/intake.rs; fsl-semantics/tests/it/model_intake.rs |

## Coverage

- Reconciliation: `quoin matrix --repo` refused with `CORE_REFUSED: request exceeds the accepted size`; used `quire matrix --scope ... --format json` (quire 0.52.0, engine 0.62.2), with no run evidence.
- Criteria: tagged 5,587; untagged 1,025; tagged-by-ignored-test 26; method-without-symbol 109; retired 15.
- `FR-056-AC-5` is tagged by 15 tests, including the three changed tests at `fsl-semantics/tests/it/model_intake.rs:205,235,253`; it has unbacked-claim diagnostics from existing tests but the changed tests themselves are bound. `FR-056-AC-42` is tagged by one test.
- Coverage extraction: untracked symbols 6,579; 24,747 diagnostics; 142 suspicions. These are repository-wide, not caused by this PR.
- Reverse-gap review was limited to the changed intake mapping and test helpers: one new mapping behavior has an owning requirement, and no new stubs or hollow tests were found there. A full manual inventory of all repository behaviors was not completed.
- Semantic review: skipped (not requested). Plan completion: not assessed.
- No applicable AssuranceProfile applies to the changed behavior.

## Round 1 verification

Recomputed the matrix and coverage on `d526596b8360bed7467ace6dded101a2139c3261`. `quire matrix --scope /home/peter/dev/worktrees/qsl-3311-intake-missing-name --format json` exited 0 and retained the same counts: tagged 5,587; untagged 1,025; tagged-by-ignored-test 26; method-without-symbol 109; retired 15. `quire coverage --scope /home/peter/dev/worktrees/qsl-3311-intake-missing-name --json` exited 0 with 6,579 untracked symbols, 24,747 diagnostics, and 142 suspicions. The three QSL-3311 tests remain among the 15 binders for `FR-056-AC-5`; `FR-056-AC-42` remains tagged.

Completed the reverse-gap and stub review for the applicable `fsl-semantics` source/test scope. The changed behavior is private intake mapping logic owned by `FR-056-AC-5`; the fix adds no public API or new externally visible surface. A source inventory found 2,990 public declaration lines in `fsl-semantics/src`; the 380 Rust files across that crate's source and tests were scanned for tiny files, placeholders, and ignored tests. The two tiny source files are a `JsonValue` type alias and a documentation-only file stating that its lowering nodes are not yet designed; neither is a hollow implementation behind this change's tagged criterion. Placeholder-token hits were doctest examples and test fixtures, and the ignored tests had explicit tracked reasons. The three changed tests assert the exact refusal payload and execute the production intake path. No diff-specific unowned behavior, source stub, or hollow test was found.

The full FSL-wide public-declaration ownership inventory remains broader than this PR's source scope and is explicitly owned by QSL-3110, whose Done-when requires that inventory and its reconciliation. QSL-3110 also explicitly owns the repository-wide untagged criteria, ignored-only criteria, and untracked tags found by the matrix/coverage run.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-3110 owns the repository-wide inventory and reconciliation of untagged criteria and ignored-only binders; these matrix results are pre-existing and not caused by QSL-3311. |
| FND-002 | deferred | QSL-3110 explicitly owns the repository-wide untracked trace-symbol inventory and requires each tag to be corrected, removed, or dispositioned. |
| FND-003 | deferred | QSL-3110 explicitly owns the remaining FSL-wide code-to-requirement inventory. The completed `fsl-semantics` scope scan found no gap introduced by this diff. |
