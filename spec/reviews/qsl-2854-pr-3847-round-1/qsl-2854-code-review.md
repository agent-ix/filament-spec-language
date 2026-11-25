---
id: SR-967175026302197768900474181941935084844
title: "QSL-2854 Rust code review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; Cargo.lock; fsl-package/src/bare_term.rs; fsl-package/src/checked_v2.rs; fsl-package/src/emit/tests/composition_view.rs; fsl-replay/Cargo.toml; fsl-replay/src/hybrid_replay/class.rs; fsl-replay/src/hybrid_replay/tests.rs; fsl-replay/src/refinement/package.rs; spec/non-functional/NFR-141-measure-performance-program-paths.md; QSL-2854; PR #3847 at f278120ad92cee60199106c364e676eac93629c9"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/NFR-141", type: reviews}
---

## Summary

Reviewed the full PR diff, with the Rust lane applied to all changed Rust and Cargo files and the three new NFR-141 criteria checked against the implementation and tests. The main paths are behavior-preserving by inspection, but the new test-only pre-change oracles duplicate production behavior, and new trace attributes also include unminted parent requirement ids.

## Verdict

**FAIL** — the Agent IX code-review duplication gate treats the three copied prior implementations as high-severity findings; two new trace attributes also carry parent ids that are not valid trace targets.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `previous_projection` reimplements the declared-composite projection in a test as a copied pre-change oracle; the projection now also exists in `BarePackage::declared_composites`, creating a second implementation to keep in sync. | fsl-package/src/emit/tests/composition_view.rs:374-413; fsl-package/src/bare_term.rs:1838 |
| FND-002 | high | `prior_hidden_projection` copies the old nested membership scan into a test oracle beside the new `hidden_field_keys` implementation, creating two maintained statements of the same hidden-field rule. | fsl-replay/src/refinement/package.rs:46-54,563-581 |
| FND-003 | high | `prior_term` and `prior_polynomial_gap` duplicate the removed recursive class-walk implementation as test oracles; `prior_recursive_walk` also reproduces its recursive depth behavior. These copies create drift points in the path and class rules. | fsl-replay/src/hybrid_replay/class.rs:549-654,805-818 |
| FND-004 | medium | The AC-25 test trace attribute includes `FR-931` and `FR-322`, which are parent requirements with minted AC children, so Quire reports both as untracked trace targets. Keep the valid AC-25 trace and remove or replace the parent ids with exact obligations actually verified. | fsl-package/src/emit/tests/composition_view.rs:366 |
| FND-005 | medium | The AC-27 tests add `FR-1019` as a trace target although that requirement has minted AC children; Quire reports the parent id as untracked. | fsl-replay/src/hybrid_replay/class.rs:657,801 |

## Scope examined

- NFR-141-AC-25 (declared-composites graph work): examined. Its test binds the new AC and asserts one graph build plus exact output equality at 250, 500 and 1,000 declarations.
- NFR-141-AC-26 (refinement hidden-field membership): examined. Its test binds the new AC, counts `Ord::cmp` operations and compares ordered output with a prior scan.
- NFR-141-AC-27 (hybrid replay class walk): examined. Its tests bind the new AC, check visit/frame/path counters, compare branch/call/locus outcomes, and isolate a depth test.
- All nine changed paths listed in `scope` were inspected; no CI workflow changed. No applicable AssuranceProfile was found under `spec/`.
- `git diff --check origin/main...HEAD` passed. `cargo fmt --all -- --check` exited 1 on unrelated workspace files; no changed file appeared in the formatter diff. Cargo tests and Clippy were not run to avoid overlapping Cargo builds. `cargo deny check` exited 1 before analysis because Cargo could not write a downloaded crate into the read-only shared registry cache.
