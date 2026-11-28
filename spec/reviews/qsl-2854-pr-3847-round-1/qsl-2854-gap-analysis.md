---
id: SR-967175026302197768900474181941935084845
title: "QSL-2854 repository gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; spec/non-functional/NFR-141-measure-performance-program-paths.md; fsl-package/src/bare_term.rs; fsl-package/src/checked_v2.rs; fsl-package/src/emit/tests/composition_view.rs; fsl-replay/src/hybrid_replay/class.rs; fsl-replay/src/hybrid_replay/tests.rs; fsl-replay/src/refinement/package.rs; QSL-2854; PR #3847 at f278120ad92cee60199106c364e676eac93629c9"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/NFR-141", type: references}
---

## Summary

The full static matrix was computed for the repository, with reverse-gap and stub inspection focused on this PR's changed production and test surface. The three new NFR-141 criteria are tagged, but the repository-wide matrix has 1,088 untagged criteria and 24 criteria supported only by ignored tests; the new tests also carry unminted parent trace ids.

## Verdict

**FAIL** — the whole-repository static matrix has untagged and ignored-only criteria; the changed ACs are tagged but have extra invalid parent tags.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-001-AC-1 remains an active untagged criterion even though its text says the source-content check was retired; the matrix cannot establish test evidence for this live obligation. | FR-001-AC-1 |
| FND-002 | high | FR-056-AC-60 is backed only by an ignored test, so no runnable test currently covers that criterion. | FR-056-AC-60 |
| FND-003 | medium | New tests add parent trace ids FR-322 and FR-931 although Quire only mints their child ACs; these ids are reported as untracked. | fsl-package/src/emit/tests/composition_view.rs:366 |
| FND-004 | medium | New tests add parent trace id FR-1019 although Quire only mints its child ACs; this id is reported as untracked. | fsl-replay/src/hybrid_replay/class.rs:657,801 |

## Coverage

- Reconciliation: `quoin matrix --repo . --json` refused with `CORE_REFUSED: request exceeds the accepted size`; the documented static fallback `quire matrix --scope . --format json` succeeded (Quire CLI 0.52.0, engine 0.62.2). `quire coverage --scope . --json` succeeded; the evidence store has no run evidence for this static matrix.
- Matrix status counts: tagged 5,488; untagged 1,088; tagged-by-ignored-test 24; method-without-symbol 109; retired 15. The three changed criteria NFR-141-AC-25, AC-26 and AC-27 each have at least one tagged test.
- Coverage output reported 6,519 untracked symbols repository-wide. In the changed test files, the actionable new untracked tags are the parent ids listed in FND-003; an existing module doc reference to FR-1019 is context only.
- Changed production behavior inventoried: one graph construction for declared composites, ordered hidden-field membership for refinement objects, and iterative hybrid class-gap walks. Each maps to the new NFR-141 ACs; no additional unowned behavior or source/test stubs were found in the changed surface.
- Plan completion: not assessed
- Semantic review: skipped

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `FR-001-AC-1` is now an active criterion backed by `TC-424` and the tagged source-identity test `admission_mints_the_caller_named_source_reference`. |
| FND-002 | fixed | `FR-056-AC-60` is backed by the runnable `a_float_field_model_explores_by_its_ieee_bits` test, including direct solved field writes and the signed-zero successor assertion. |
| FND-003 | fixed | The AC-25 test trace now names only `NFR-141-AC-25`; the unminted parent ids are removed. |
| FND-004 | fixed | Hybrid class test traces now name only `NFR-141-AC-27`; the unminted parent id is removed. |

Disposition verification at `85747e97a81f91d20d0dd02167746b3de015e80b` on Titan: `cargo test --locked -p fsl-foundation --lib admission_mints_the_caller_named_source_reference -- --test-threads=3` passed (1); `cargo test --locked -p fsl-eval --test it a_float_field_model_explores_by_its_ieee_bits -- --test-threads=3` passed (1); the declared-composites, hidden-field, and hybrid class tests passed (1, 1 and 3); `cargo check --locked --workspace --all-targets` passed with default features (not `--all-features`); and `cargo clippy --locked -p fsl-package --all-targets -- -D warnings` passed. The full repository matrix still has its previously reported unrelated baseline coverage gaps; the four findings in this PR's original review are resolved.
