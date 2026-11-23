---
id: SR-203
title: "Code review — QSL-2910 FSL caller-limit specification"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; FR-255 lifecycle caller-limit coverage and TC-721"
review_set: subset
---

## Summary

Reviewed the two changed specification artifacts against the relevant replay limit construction and consumption paths. One medium finding: the replay list calls several unused `CallerLimits` fields caller-configurable bounds, which would make the operation accept settings that do not control an enforced bound.

## Verdict

**CONDITIONAL** — narrow replay's accepted settings to bounds replay consumes, or specify the new consumers that make each listed setting effective.

## Assurance Context

No applicable `AssuranceProfile` exists in this repository's specification scope.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-255-AC-24 | examined | `The analyze, monitor and replay command entry points accept exactly the operation-relevant caller-limit settings enumerated in “Lifecycle caller-limit coverage”; each accepted setting changes only its corresponding bound, while recognized but operation-unavailable settings are refused before work. In particular, monitor accepts monitor.preparation_work_units and monitor.preparation_storage, and replay accepts the stage settings it executes plus replay.input_bytes through its replay-reader argument, but not as a stage_limits member.` |
| FR-255 replay coverage | examined | Replay list includes prepare.compile_work_units, prepare.code_bytes, expansion.max_candidates, simulation.max_steps, all ten model_check.* settings, evaluation.live_occurrences and all thirteen temporal_evidence.* settings. |
| TC-721 step 11 | examined | The command caller-limit object is built from every exact setting in the coverage section and each accepted name must change only its corresponding bound. |
| fsl-replay/src/limits.rs:266-281 | context_only | CallerLimits stores replay, prepare, engines and temporal_evidence limit objects. |
| fsl-replay/src/limits.rs:329-359 | context_only | for_request applies request stage entries to CallerLimits, except replay.input_bytes and format.output_bytes. |
| fsl-replay/src/model_check/graph_replay.rs:206-220 | context_only | The replay graph path reads caller.engines.model_check and uses it for a graph certificate check. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Replay's caller-limit list includes bounds no replay execution path consumes | FR-255-AC-24; fsl-replay/src/limits.rs:329-359 |

`for_request` can populate the generic `prepare`, `engines` and `temporal_evidence` fields, but that alone does not make those bounds operative. Replay's execution paths do not read `caller.prepare` or `caller.temporal_evidence`; `caller.engines` is read only for model-check replay, where the graph path uses the `max_states` and `max_transitions` fields, not every model_check field. The criterion additionally promises `expansion.max_candidates`, `simulation.max_steps` and `evaluation.live_occurrences`, without a replay consumer for those caller bounds. Since the requirement defines the exact accepted set and says each accepted name changes its enforced bound, this list can create accepted no-op limits. Specify only settings whose replay paths consume the value, or name the new consumer and how it enforces the bound for every additional setting.


## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The replay list now accepts only the three model-check bounds its graph replay consumes and explicitly refuses `expansion.max_candidates` and the other unused bounds. |
