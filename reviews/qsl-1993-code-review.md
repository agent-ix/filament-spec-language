---
id: SR-2026111612172400000022
title: "code-review — FSL diagnostics batch PR 2819"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2819; acceptance criteria for 24 QSL tickets"
review_set: subset
---

## Summary

Reviewed the PR’s Rust implementation and the acceptance criteria for the linked FSL diagnostics batch. Found one concrete user-facing defect in the bounded syntax diagnostic message.

## Verdict

**CONDITIONAL** — one medium finding: the syntax diagnostic cut message recommends a nonexistent setting.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Diagnostic cut message recommends nonexistent `s1.diagnostics` setting | fsl-cst/src/parser.rs:164, fsl-foundation/src/setting.rs:121 |

## Review limitations

Quire validation and aggregate gates were not run. Focused verification claims in the PR body were read but not rerun.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Both cut-message sites now say `i2.diagnostics`, but that setting belongs to the I2 v2 reader; `fsl_cst::Limits::set_bound` rejects it and exposes only S1 settings. The parser cutoff still cannot be raised. The focused cutoff test passed only because its expected string was changed to `i2.diagnostics`; it does not verify that the setting is accepted. |
| FND-001 | fixed | after_excerpt: `{omitted} additional syntax errors omitted after the fixed limit of {SYNTAX_DIAGNOSTIC_BOUND}`; `{omitted_count} additional syntax errors omitted after the fixed limit of {SYNTAX_DIAGNOSTIC_BOUND}` |
