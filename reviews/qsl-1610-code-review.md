---
id: SR-2026112001423700000001
title: "Code review — QSL-1610 temporal strict reader"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-replay/src/spine/clause/tests/temporal/fspec_kinds.rs, fsl-replay/src/spine/clause/tests/temporal/wire.rs, fsl-replay/src/spine/native_result/temporal.rs; QSL-1610"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-267", type: references }
---

## Summary

Reviewed the three changed Rust files at PR #3096 head `a9a98b7e9aaf802152ec9514454e82c5ec7ef284` against current `origin/main`, FR-267 and the ticket's FR-352 ruling. The temporal reader behavior and acceptance coverage look correct; focused tests pass, with one low formatting finding.

## Verdict

**CONDITIONAL** — the changed Rust files fail focused `rustfmt --check`; behavioral tests pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new `condition-refused` error arm is not rustfmt-clean: the return expression at line 1858 needs rustfmt formatting. The touched `fspec_kinds.rs` import block also needs sorting after the new import. | fsl-replay/src/spine/clause/tests/temporal/fspec_kinds.rs:7-13; fsl-replay/src/spine/native_result/temporal.rs:1858 |

## Scope

- FSL FR-267-AC-20: `Each of the six rows pending-window, pending-next-event, indeterminate, activation-incomplete, progress-limit-reached and progress-contradicted, and the pending row, is written with exactly the members, category, basis and outcome of its FSpec FR-352 row (FSpec FR-352-AC-16).` — examined.
- FSpec FR-352-AC-10: `Each of the sixteen disposition.temporal kinds is written at stage evaluate of a temporal selection with exactly its row's members, category, truth, basis and outcome, no top-level witness and no refusal, and reads back to an equal disposition.` — examined.
- FSpec FR-352-AC-11: `A pending disposition reads with category pending, no truth and basis unsettled.` — examined.
- FSpec FR-352-AC-16: `Each of these temporal dispositions reads at stage evaluate with exactly its row's members, category, truth (absent), basis and outcome, and reads back equal` — examined.
- FSpec FR-352-AC-17: `Independently, each refuses with a located wire diagnostic naming the member: a temporal whose kind is outside the sixteen (pending-forever, held); category pending on an incomplete row and category indeterminate on a violated row; category inconclusive on a pending row` — examined.
- FSpec FR-352-AC-22: `A window-limit-reached disposition is written at stage evaluate as {"kind": "window-limit-reached", "limit": "max_window_values", "value": "2", "setting": "monitor.max_window_values"}, its value a decimal string` — examined.

## Verification

- `cargo test -p fsl-replay --lib temporal_rows_restore_verdicts_and_refuse_mismatched_members` — passed.
- `cargo test -p fsl-replay --lib limit_values_read_decimal_strings_and_refuse_other_encodings` — passed.
- `cargo test -p fsl-replay --lib a_condition_refusal_reads_its_position_and_catalog_outcome` — passed.
- `cargo test -p fsl-replay --lib an_unregistered_temporal_kind_refuses` — passed.
- `rustfmt --check --edition 2024` on the three changed Rust files — failed on formatting differences listed in FND-001.
- `cargo fmt --check` — failed on broad existing workspace formatting differences outside this PR; no files changed.
- `cargo clippy -p fsl-replay --lib --tests -- -D warnings` — failed on 569 `result_large_err` lints in unchanged `fsl-semantics`; no diagnostic was in changed files.
- No aggregate gate run.
