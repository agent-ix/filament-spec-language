---
id: SR-967175026302197768900474181941935084847
title: "Gap analysis — QSL-3102 repository assurance"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository-wide spec/, Rust source, and tests; planless"
review_set: subset
relationships:
  - {target: ix://agent-ix/filament-spec-language, type: references}
---

## Summary

Ran the planless traceability matrix and coverage scan for the repository. The matrix reports 1,076 untagged criteria and the coverage scan reports 6,521 untracked symbols; the full code-to-requirement ownership inventory could not be completed in this review pass.

## Verdict

**FAIL** — untagged criteria are high-severity gaps; the reverse-gap inventory is incomplete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed matrix reports 1,076 untagged criteria with no bound test | spec/ (quire matrix) |
| FND-002 | medium | Coverage reports 6,521 untracked trace symbols that name no declared criterion | source and tests (quire coverage) |
| FND-003 | high | The reverse-gap audit could not map all 5,932 public Rust declarations to owning requirements | Rust source tree |

## Coverage

- Reconciliation: quire matrix (quire 0.52.1) — no run evidence read. `quoin matrix --repo . --json` was attempted but failed with `CORE_REFUSED: request exceeds the accepted size`.
- Criteria: tagged 5,499 · untagged 1,076 · tagged-by-ignored-test 24 · method-without-symbol 109 · retired 15.
- Untracked tags: 6,521 symbols; the changed test paths contribute 18 pre-existing foreign or old IDs. Newly changed test bindings for FR-195-AC-5 and FR-199-AC-4/5 are present in the matrix.
- Reverse-gap / stubs: inventoried 5,932 public Rust declarations; no tiny source stubs were found in `fsl-*/src`, and the `TODO` / `unimplemented!` matches found there were in documentation examples. Ownership mapping across the full inventory remains incomplete.
- Semantic review: skipped (optional opt-in step).
- Plan completion: not assessed

## Dispositions

Checked at head a36a2293324f9f06ca46c92f9c31c696868a65c6 against a static origin/main checkout (c00fe79348, quire 0.52.1). PR-scope bindings: FR-195-AC-4, FR-195-AC-5, FR-199-AC-4 and FR-199-AC-5 all report `tagged`; this PR moves FR-199-AC-5 from untagged to tagged and makes no criterion untagged.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | Pre-existing baseline (origin/main 1,072 untagged, head 1,076; every difference comes from main commits after the merge base, none from this diff). Owned by QSL-3110 |
| FND-002 | deferred | Pre-existing (origin/main 6,522 untracked symbols, head 6,523; the one extra is from main commit 631b95ec3c, the 18 in changed files are identical on main). QSL-3110 scope widened by comment to own it |
| FND-003 | deferred | Pre-existing; the PR changes only tests, adding no public declaration. QSL-3110 scope widened by comment to own the reverse-gap inventory |
