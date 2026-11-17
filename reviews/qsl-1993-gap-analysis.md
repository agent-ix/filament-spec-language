---
id: SR-2026111612172400000023
title: "gap-analysis — FSL diagnostics batch PR 2819"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2819; acceptance criteria for 24 QSL tickets"
review_set: subset
---

## Summary

Manually compared the PR diff, added tests, and ticket acceptance criteria because Quoin matrix loading failed. The implemented bounded syntax diagnostic path contains one concrete gap: its suggested setting is not accepted.

## Verdict

**CONDITIONAL** — one medium finding: the syntax diagnostic cut message recommends a nonexistent setting.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Diagnostic cut message recommends nonexistent `s1.diagnostics` setting | fsl-cst/src/parser.rs:164, fsl-foundation/src/setting.rs:121 |

## Coverage

Plan completion: not assessed. Manual acceptance-to-diff/test review covered the acceptance criteria for QSL-1985, QSL-1993, QSL-1994, QSL-1996, QSL-2005, QSL-2009, QSL-2010, QSL-2011, QSL-2014, QSL-2015, QSL-2016, QSL-2017, QSL-2018, QSL-2195–QSL-2203, QSL-2266 and QSL-2267. Quoin matrix unavailable: installed Quoin 0.35.0 refused to load the spec-objects-architecture 0.5.0 module because `traceability.required_relations[2].record_keys` is unknown. PR-stated partial scopes remain pending and were treated as pending, not completed: QSL-1993 loci, QSL-1994, QSL-2005, QSL-2009 enum classification, QSL-2014–2016 driver work, and QSL-2203 producers. Semantic review was not requested.

## Review limitations

Quire validation and aggregate gates were not run. Focused verification claims in the PR body were read but not rerun.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Both cut-message sites now say `i2.diagnostics`, but that setting belongs to the I2 v2 reader; `fsl_cst::Limits::set_bound` rejects it and exposes only S1 settings. The parser cutoff still cannot be raised. The focused cutoff test passed only because its expected string was changed to `i2.diagnostics`; it does not verify that the setting is accepted. |
| FND-001 | fixed | after_excerpt: `{omitted} additional syntax errors omitted after the fixed limit of {SYNTAX_DIAGNOSTIC_BOUND}`; `{omitted_count} additional syntax errors omitted after the fixed limit of {SYNTAX_DIAGNOSTIC_BOUND}` |
