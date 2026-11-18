---
id: SR-2026111612172400000039
title: "Code and Rust review of the nested option emission probe (filament-spec-language PR 2919)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@29bbeeef9cd4286a43895d614e2303b23a255c3e; git diff origin/main...HEAD (PR 2919, branch qsl-2257-option-nesting-probe); fsl-package/src/emit/tests.rs, fsl-package/src/emit/tests/option_nesting.rs"
review_set: subset
date: 2026-11-18
---

## Summary

Ticket: QSL-2257. PR: filament-spec-language#2919, head 29bbeeef9. Method:
`/code-review` with the `/rust-review` lane, over `git diff origin/main...HEAD`
(2 files, +61). Reviewer-only; no edits to the branch. `quoin:gap-analysis` was
not run: the diff changes no production code. The test-to-AC binding check was
done by hand in this review instead (FND-001).

The diff adds two fsl-package emit tests. They emit `Option<Option<Boolean>>`
and `Option<Option<Int[0, 9]>>` function parameters through `emit_checked`,
assert that nothing is omitted, assert that the pinned contract-ir v0.1.4
reader returns `Verified`, and walk parameter, outer option, inner option and
element nodes.

## Verdict

**Changes requested (one medium trace finding). Mergeable once FND-001 is
fixed with a one-line retag.**

- The assertions are real. If the nesting were flattened, the inner node would
  be the element and `assert_eq!(inner["semantic_form"], "option")` would fail.
  If emission refused or omitted the type, `emit_checked(..).expect` or the
  `omitted == []` assertion would fail. If the reader refused the package, the
  `let Read::Verified` else-panic would fire. A missing reference target makes
  `with_id` panic, because it looks up a `Null` id.
- Not a duplicate. No other fsl-package emit test covers `Option<Option<_>>`.
  Nested-option checking depth (FR-092-AC-7) is tested in fsl-semantics, which
  does not emit or read back. No existing test was changed or weakened. The
  only edit to an existing file is the `mod option_nesting;` line, in
  alphabetical order.
- The pinned reader is the one in use: `filament-contract-model v0.1.2` from
  `filament-contract-ir?tag=v0.1.4`.

## Gates

| Gate | Result |
| --- | --- |
| `cargo test -p fsl-package --lib option_nesting` | 2 passed |
| `cargo fmt -p fsl-package --check` | pass, no diffs |
| `cargo clippy -p fsl-package --all-targets -- -D warnings` | pass |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Both tests are tagged `#[trace("FR-093-AC-48")]`, but FR-093-AC-48 is about a present-option value (`Some(e)`) being lowered under the carrier contract, the emitted package id equalling the id the reader computes, and `Some(e)` type mismatches being refused. The tests build no `Some` value, check no package id and test no refusal, so they cannot fail on anything AC-48 states. The tag adds coverage to AC-48 that it does not have. The doc comment says "no AC names nested options", which is also wrong. FR-092-AC-1 owns the asserted behaviour: its Type nodes table row `Option<T>` maps to `composite_type`/`option` with a reference to `T`, and it lists `Option<Int[0,9]>`. FR-092-AC-7 names nested `Option`s, but only for check depth. Retag to FR-092-AC-1 (TC-413) and fix the comment. No new spec AC is needed, so no FR-093 addition belongs in this PR. | fsl-package/src/emit/tests/option_nesting.rs:38-40, 50-52 |
| FND-002 | low | The outer and inner option nodes are checked only for `semantic_form == "option"`, not for the rest of the FR-092 `Option<T>` row: `node_tag` `composite_type` and `semantic_type` the node itself. Under the FR-092-AC-1 tag that FND-001 asks for, the chain is only partly asserted. | fsl-package/src/emit/tests/option_nesting.rs:30-33 |
| FND-003 | low | `nested_option_chain` returns the wire and clones the outer and inner nodes, but both callers throw them away (`let (_, [_, _, element])`). The returned wire and the first two chain entries are dead values. | fsl-package/src/emit/tests/option_nesting.rs:18, 35-36, 43, 54 |
