---
id: SR-2026111414542500000007
title: "Integrity review of PR #2714 macOS RSS measurement text"
type: SpecReview
analysis: integrity
scope: 'agent-ix/filament-spec-language@f22fba6f9295f23872cc6262d41198392652f217; spec/assurance/MP-006-probe-check-wall-time-and-rss.md, spec/assurance/MP-011-model-check-probe-rates-and-rss.md, fsl-bench/BENCHMARKING.md'
review_set: subset
date: '2026-11-15'
---

## Summary

Reviewed the changed MeasurementPlan and benchmark descriptions for agreement on the macOS reader, units, comparison series and apparatus scope. The text consistently identifies Darwin `ru_maxrss` from `getrusage(RUSAGE_SELF)` as bytes converted to KiB, and separates results from the prior `vmmap` series. No applicable AssuranceProfile was found under `spec/`.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| MP-006 macOS sampling and population description | spec/assurance/MP-006-probe-check-wall-time-and-rss.md | examined |
| MP-011 macOS population and sampling description | spec/assurance/MP-011-model-check-probe-rates-and-rss.md | examined |
| benchmark README description of RSS and cross-series comparison | fsl-bench/BENCHMARKING.md | examined |

## Verdict

PASS for the changed measurement text. Both plans use KiB consistently, state that earlier macOS `vmmap` measurements form a separate series, and preserve the Linux `VmHWM` path. The updated apparatus description still names the RSS reader as protected apparatus.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found (placeholder) | - |
