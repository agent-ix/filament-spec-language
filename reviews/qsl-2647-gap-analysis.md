---
id: SR-202611200234061
title: "gap-analysis of PR3129 temporal and fairness export lowering against FSpec FR-813"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FSpec FR-813-AC-1..AC-6 (filament-specification origin/main) against tests/it/export_temporal_lowering.rs and fsl-command/src/command/export/lower/temporal.rs; PR3129 head f977c3a4d876162f4099033d659b2e278357b7e3"
review_set: subset
---

## Summary

Ticket: QSL-2647. Manual criterion-to-test check: the FR-813 criteria live in the filament-specification repository, so `quoin matrix` over this repository does not list them. Plan completion: not assessed. Slice 2 parts (AC-4 protocol half) and the fix-round parts owed by the arch rulings (AC-6 nondeterministic half, AC-1 quantified form) are out of scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-813-AC-3 second half (two inserted stuttering steps keep the verdict) has no test, and the tagged golden asserts a counter bound of 4 where the AC says 3 | tests/it/export_temporal_lowering.rs:166-201 |
| FND-002 | medium | FR-813 Behavior past operators: historically, since and triggered have no test in either target; only `once` over an atom, the one case where the pre-origin reading coincides with FR-092, is covered | tests/it/export_temporal_lowering.rs:268-315 |

## Verdict

AC-1 (always, both targets), AC-2 (until, both targets, plus release), AC-4 weak and strong fairness half, AC-5 (Lean has no theorem, proof or sorry) and AC-6 co-Buchi half have exact-text goldens that assert the whole clause text. Records are asserted `preserved` with regions for always, until and eventually-always. The two gaps above are the ones that let code-review SR-202611200234060 FND-001 ship unguarded.
