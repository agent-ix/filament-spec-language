---
id: SR-2026111414542500000006
title: "Gap analysis of PR #2714 macOS RSS reader delta"
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/filament-spec-language@f22fba6f9295f23872cc6262d41198392652f217; delta d524008c0976dbeb8b08b775b4cf9bb86a34cad4..f22fba6f9295f23872cc6262d41198392652f217; fsl-bench/src/rss.rs and MP-006/MP-011 peak RSS units'
review_set: subset
date: '2026-11-15'
---

## Summary

The installed Quoin 0.29.4 has no `gap-analysis` command (`quoin gap-analysis --help` exited 2), so this is a manual acceptance-to-test check scoped to the RSS reader delta, not a full repository audit. QSL-1906-AC-2 is exercised on this Darwin host, but the test asserts only that the result is positive; it does not pin the KiB conversion required by MP-006 and MP-011. Plan completion: not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| QSL-1906-AC-2: Peak RSS prints a number on macOS and Linux | Linear issue QSL-1906 | examined |
| positive-value test for the peak RSS reader | fsl-bench/src/rss.rs:46-50 | examined |
| KiB unit for macOS peak RSS | spec/assurance/MP-006-probe-check-wall-time-and-rss.md:87-93; spec/assurance/MP-011-model-check-probe-rates-and-rss.md:17-20, 83-90 | examined |
| remaining QSL-1906 criteria and repository matrix | outside this delta review | context_only |

## Verdict

CONDITIONAL. The existing test runs through the new macOS path and confirms a numeric result. It would still pass if `ru_maxrss` bytes were returned as KiB without division, so it does not protect the measurement unit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The macOS RSS test checks only `kib > 0`, which also passes if raw bytes are returned. It does not catch a regression that removes the `/ 1024` conversion required by the measurement plans' KiB unit. | fsl-bench/src/rss.rs:34, 46-50; spec/assurance/MP-006-probe-check-wall-time-and-rss.md:90-93; spec/assurance/MP-011-model-check-probe-rates-and-rss.md:18 |

## Coverage

Manual delta check: QSL-1906-AC-2 maps to `peak_rss_is_a_number_on_linux_and_macos`; that test passed in the focused fsl-bench suite on Darwin. The test confirms availability but not the reported unit. No FR or acceptance criterion was added by this delta. The full matrix and reverse-gap audit were not run because the installed Quoin binary has no `gap-analysis` subcommand; no repository-wide PASS is claimed. Plan completion: not assessed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `bytes_to_kib` now owns the conversion and `darwin_ru_maxrss_bytes_convert_to_kib` asserts zero, sub-KiB, exact-KiB and multi-KiB cases. |
