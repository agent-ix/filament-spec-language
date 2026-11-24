---
id: SR-967175026302197768900474181941935084804
title: "gap-analysis of PR3641 on-each profile boundary"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3641 head bef20c929aac956e9d0bec9128105cef4a469dd5; FR-875, TC-3211, FR-202-AC-36; diff origin/main...HEAD only"
review_set: subset
---

## Summary

Tickets: QSL-2995, QSL-2923, QSL-2884. Plan completion: not assessed. `quoin matrix --json` did not finish within 300 s on this repository (killed by timeout, empty output), so the matrix step fell back to a labelled manual trace grep of `#[trace(...)]` tags over the diff; the reverse-gap and stub checks were done by reading the diff. FR-875-AC-1 to AC-8 each have at least one tagged test on the head. One behaviour change has no acceptance criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Empty-population precedence over the profile boundary (out-of-profile form with no instance settles no-instance, not unsupported) is only Behavior prose in FR-875; no AC states it, yet the new tests assert it under tags FR-875-AC-4..8, whose text covers only the non-empty case | fsl-analyze/tests/it/probability_from_source.rs:1972-1980 |
| FND-002 | low | The new statistical on-each settlement path (items.rs intercept, instances::statistical) has no test for its admission-failure or limit branches; only the empty and non-empty outcomes are covered | fsl-analyze/src/analyze/exact/instances.rs:127-142 |

## Scope and evidence

- FR-875-AC-1 (TC-2485, probability_from_source.rs:694): examined, tagged.
- FR-875-AC-4 to FR-875-AC-8 (TC-3211, probability_from_source.rs:1983-2140): examined, tagged; non-empty assertions discriminate (the cyclic and form cases would otherwise settle proved or a verdict).
- FR-202-AC-36 (TC-3162, subsystem_refutation.rs:622): examined, tagged.
- Reverse gap: on_each_has_instance and statistical_on_each are owned by FR-875 "Instances" and "Profile boundary"; no orphan code. No stubs or ignored tests added.
- QSL-2995 acceptance ("built ... or shown out of FR-875's scope by a spec statement"): satisfied by FR-875's merged Profile boundary (#3619) for the listed forms; no on-each item in the tests settles unsupported for its activation.
- QSL-2884: no commit; not re-measured per lane report.

## Verdict

No blocking gaps; two lows.
