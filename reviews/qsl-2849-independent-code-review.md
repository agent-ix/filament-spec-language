---
id: SR-3639004
title: "Independent code review of PR 3639 (QSL-2849)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3639, EN-1 initial states, dense solver limit references and EN-7 recurrence documentation"
review_set: subset
---

## Summary

Recovered the independent review recorded at bc7fa1b398 from Linear comment ea096aca-c5de-4853-9df1-82bcac999aed. This review has a separate ID and no committed counterpart at the reviewed head. The original table summaries are reproduced from that record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | EN-1 still allocates the whole initial set before the refusal: `SubjectSystem::initial()` returns a `Vec`, so check.rs builds every initial state and only then hands them to admit_initial. Canonicalising and keying are bounded; the allocation is not. | fsl-analyze/src/model_check/check.rs:2515-2520 |
| FND-002 | low | FR-255's `certificate_check.max_working_bytes` row still lists only the probability document writer/reader and source checker as readers (FR-201, FR-282). The exact dense solver (FR-203-AC-16) now reads the same bound. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:303 |
| FND-003 | low | ADR-026 RT-4 still says the fixed-priority recurrence "terminates with no iteration cap", which contradicts the amended FR-247 (every window iteration is charged to `max_demand_points`). | spec/decisions/ADR-026-dense-time.md:374 |
| FND-004 | low | Stale references: the Pace doc cites FR-247-AC-9 (it should cite AC-5), the `ScheduleLimits::max_demand_points` doc still reads "How many demand points QPA may check", and TC-638 step 6 says "three-state chain ... one-row bound" while the test uses a four-state product and a one-cell bound (`one_row = size_of::<Rational>()`). | fsl-analyze/src/schedulability.rs:34,54 |

## Verdict

The earlier reviewer recorded four findings. Round-two outcomes follow.

## Dispositions

Round 2 reviewed `1ac4da2a725290e8986f265eeb45bf73917e7d0d`. The PR requires another fix round.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The direct model/protocol exploration loop now receives initial_iter. Supported reduced-model and sampled state-graph requests still collect all generated initial states before reaching that loop. See reduce.rs:244, state_graph/sample.rs:237, and SR-3639001 FND-010 and FND-011. |
| FND-002 | fixed | FR-255 names the dense solver, FR-203-AC-16 and FR-203 in the settings row. |
| FND-003 | fixed | ADR-026 RT-4 now charges each recurrence iteration to max_demand_points. |
| FND-004 | fixed | Pace cites AC-5, ScheduleLimits includes both recurrence types, and TC-638 step 6 describes the four-state, one-cell control. |
