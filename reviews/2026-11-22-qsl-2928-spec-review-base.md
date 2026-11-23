---
id: SR-967175026302197768900474181941935084785
title: "QSL-2928 FR-166 closure-stop review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR #3556 at dbc23be76ef88ecd69e5a6f1b4cfd45b967ab0f9; FR-166 and ADR-022 GE-2, with FR-101 and FSpec FR-392 context"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-166
    type: reviews
  - target: ix://agent-ix/filament-spec-language/ADR-022
    type: reviews
---

## Summary

Reviewed the early closure stop against FR-101's sampling API and provenance, FR-392's seed default, and the changed FR-166/ADR-022 criteria. The closure condition is logically sound once established, but the normative behavior cannot be implemented through the required `sample_request` call as specified.

## Verdict

**FAIL** — FR-166 requires a behavior its named sampler interface cannot perform or report.

## Scope and evidence

| Unit | Role | Evidence |
| --- | --- | --- |
| FR-166 Behavior, sampler invocation | examined | FR-166 calls `sample_request` with a fixed step ceiling before separately requiring an early stop. |
| FR-166 closure-stop rule | examined | Closure requires a complete successor set for each visited state, all corresponding edges taken, and all destinations visited. |
| FR-166-AC-1 | examined | A witness ending at its first target remains byte-identical, including provenance. |
| FR-166-AC-2 and FR-166-AC-7 | examined | The 9-state/18-edge model uses a 512-step cap; tests record total steps and expect zero max-step stops. |
| FR-166-AC-8 | examined | The synthetic graph enumerates 4 states and 10 edges, sets a 256-step cap, and asserts exact exploration counts. |
| ADR-022 GE-2 | examined | Duplicates the early-stop condition and distinguishes it from max_walk_steps stops. |
| FR-101 sampler contract | context_only | `sample_request` accepts seed, trace, and max_steps and returns a complete `Trace`; the sampler advances per transition and reports StepLimit, NoSuccessors, or Stopped. |
| FSpec FR-392 | context_only | The request seed defaults to 0; this makes the seeded acceptance vectors reproducible. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-166 requires early closure termination that `sample_request` cannot express | FR-166:105-110; FR-101:188-197, 392-393 |

FR-166 requires the engine to call `sample_request` with a fixed ceiling, then stop that same walk before the ceiling when observed closure occurs. FR-101 exposes only a call that runs to a terminal trace and returns it; it has no incremental/pause callback or closure predicate. Its trace stop reasons also have no normal closure outcome. An implementation therefore cannot both stop early and preserve/report the sampler's cursor and trace provenance using the mandated interface. Specify a shared incremental sampling primitive or revise FR-101's API and stop-reason contract, then define how the completed trace represents closure. The existing step bounds are concrete deterministic assertions for seed 0, and the 4-state/10-edge graph is concrete; the obstacle is execution semantics rather than graph sizing.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-101 now defines an incremental observer API that emits either a complete expansion or its stop cause, returns only a prefix without Trace/provenance on observer stop, and guarantees always-continue equivalence to `sample_request`. |
