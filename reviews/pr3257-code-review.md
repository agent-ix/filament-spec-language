---
id: SR-20261121043348
title: "code-review and rust-review of PR 3257 pin-check full-sha rev pins"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; xtask/src/pin_check.rs, xtask/tests/consumer_facades.rs; PR 3257 head 704c4431b951df98a17a0e0160f90a99a8497574"
review_set: subset
---

## Summary

No ticket (arch ruling 2026-11-20: rev pins are the rule until FSL is complete). The rule change is small and correct: a git dependency on an agent-ix repository passes with a tag or a 40-character ASCII-hex rev, and fails on a branch, on no selector, or on an abbreviated or non-hex rev. `is_full_sha` matches Cargo's own hex test (`is_ascii_hexdigit`, either case). Manifest input cannot panic: every lookup is an `Option`, a non-string `tag`/`rev` reads as absent, and I/O and parse errors exit 2, so exit 1 means a finding. No pin, tag or `Cargo.lock` change. Every agent-ix git dependency in the workspace is a 40-hex rev, so the workspace now passes by construction. Two gaps: no test covers the no-selector path, and a manifest with both `tag` and `rev` now passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers an agent-ix git dependency with no selector. Deleting the `tag.is_none() && rev.is_none()` clause leaves every test green, while a bare git dependency then passes and follows the default branch. | xtask/src/pin_check.rs:169-173 |
| FND-002 | low | The new test's three negatives assert only exit 1, never the selector message or the dependency name, so a wrong classification still passes. | xtask/tests/consumer_facades.rs:160-166 |
| FND-003 | low | A dependency with both `tag` and a full `rev` now passes. Before this PR it was flagged `uses rev`. Cargo refuses that manifest ("only one of branch, tag or rev"), so pin-check now accepts a selector set Cargo calls invalid. Neither the code nor the spec states whether this is deliberate. | xtask/src/pin_check.rs:169 |

## Scope and evidence

Read the full diff `origin/main...704c4431b9`, all of `xtask/src/pin_check.rs`, the pin-check tests, `xtask/src/main.rs` exit handling and `xtask/src/error.rs::exit_code` (Io maps to 2). Grepped the workspace manifests: all 11 agent-ix git dependencies are 40-hex revs. Rust lane, loaded per rust-style: no `unwrap`/`expect` in production code, no indexing, no new `pub` API, no unrelated logic change. The reflow of `a_pending_field_is_rejected_by_the_closed_binding_schema` (consumer_facades.rs:213-216) is a rustfmt-only line wrap with no behaviour change. There is no "one rev per crate across the workspace" rule in pin_check.rs or in FR-760, so concern (3) has nothing to check. The first-party selector (`agent-ix/` or `agent-ix:` in the git URL) is unchanged and both tests exercise it. Not run on purpose: cargo, per the brief; titan runs the two focused commands.

## Verdict

Mergeable once FND-001 is fixed: add a no-selector case and assert `missing tag or full-sha rev`. FND-002 is a cheap addition to the same test. FND-003 needs a one-line decision: flag `tag` plus `rev` as an invalid combination, or accept it because Cargo itself refuses the manifest.
