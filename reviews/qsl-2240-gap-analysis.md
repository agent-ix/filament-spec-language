---
id: SR-20261121181100002
title: "QSL-2240 gap-analysis review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3349; FR-249-AC-8, TC-3008 and the source-closure/transport diff"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-249
---

## Summary

Frozen PR head and local HEAD both 1e91c6084f651d85ad2139fe64b1de11885dc463; clean working tree; origin/main merge base 49f9e7a18cce9e02769ce0a44e4ed7e69f272bdd. Ticket QSL-2240; PR filament-spec-language#3349. No source edits, pushes, merges, relay messages, full gates, whole-crate tests, Python, or duplicate heavy benchmark runs.

## Verdict

CONDITIONAL. 1 concrete medium finding; acceptance remains conditional on the unresolved execution evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new source-producer cancellation test claims FR-418-AC-20, but sets max_symbolic_states to zero and checks cancellation; it never checks an accepted fixed-work certificate or max_check_work. It would still pass if the entire AC-20 behavior regressed. Remove this incorrect AC-20 claim or add the actual checker/request budget assertions; retain the existing genuine max_check_work test. | fsl-analyze/tests/it/task_product_producer.rs:292-318 |

## Coverage

Plan completion: not assessed

New public behavior inventory: no_release_task_product_certificate, read_back_unchanged_with_source, clock_place_identity, to_wire_with_source, from_wire_with_source (5), plus stop_cause reuse and source checker matching. All map to FR-249 and the existing FSpec FR-418 contract. New unowned behaviors: 0; confirmed new source stubs: 0; confirmed new test stubs: 0. No coverage threshold or CI gate change. The new source-limit test's false criterion binding is a concrete new defect; inherited extraction limitations remain conditional and are not reopened as new tickets. Optional semantic gap-analysis mode was not separately invoked; code/test faithfulness was assessed under code-review.

Current-head evidence: source-transport-context-resume-1e91.log exit 0, two named tests passed (request-level unreachable checked release and clockful context/tamper); source-transport-finite-regression-1e91.log exit 0, three named tests passed (fixed-work single-task proof, refutation before symbolic pass, low max_check_work typed stop). Historical cheap7: six passes, one genuine codec failure superseded by the current-head two-test success. Frozen229 FP4: one pass, refuted and replay true, 0.40 s. Frozen229 FP3/EDF3/EDF4 remain unresolved measurements, not current-head qualification. Original B resource-cancelled without an engine verdict; max_check_work is an unmeasured expectation. No full-engine or repository PASS is claimed. Existing clippy logs are historical baseline failures, not a successful current-head gate.

Whole-repository `quoin matrix --repo . --json`, static `quire matrix --scope . --format json`, `quire coverage --scope . --json` and `quoin advise --json` were attempted. They produced no JSON after approximately 5 minutes for matrix and 4.5 minutes for the other commands, and this reviewer terminated only those owned metadata processes (143). Catalog methods loaded successfully. No computed criterion counts or advisor recommendations are available; this is an extraction/evidence limitation, not a new implementation defect. No engine-backed whole-repository coverage PASS is claimed. All changed production/test files were inspected. Broader source inventory is bounded to the PR; this review does not claim an exhaustive repository audit.

## Examined scope

```yaml
{
  "clean": false,
  "scope": [
    {
      "id": "FR-249-AC-8",
      "path": "spec/functional/FR-249-lower-a-task-automaton-to-a-timed-subject.md",
      "role": "examined",
      "excerpt": "Task set `B` in `ms`, `(C, T = D)` of `t1 (1, 4)`, `t2 (2, 6)`, `t3 (3, 10)`, `bcet` = `wcet`, priorities 1 to 3, each task released by its own always-enabled operation. EN-7 returns `Schedulable` under `fixed_priority` with fixpoints `(1, 3, 10)`, `t3` iterating `3, 6, 7, 9, 10, 10`, and under `edf` with utilization `53/60`. With `t3`'s `bcet` = `wcet` = 4, EN-7 returns `Miss` for `t3`, job 0, iterates `4, 7, 10, 11` under `fixed_priority` and `Schedulable` (utilization `59/60`) under `edf`. The original `B` work-3 fixed-priority claim on EN-6 with default limits settles typed `inco"
    },
    {
      "id": "TC-3008",
      "path": "spec/test-cases/TC-3008-a-task-automaton-carries-its-timing-and-agrees-with-response-time-analysis.md",
      "role": "examined",
      "excerpt": "Verify that the lowered task-product subject carries the declared periods\nand deadlines, that EN-6's verdict on a task automaton equals EN-7's\nresponse-time verdict on the retimed three-task set `BReduced`, that an unschedulable set\nyields a replayed counterexample whose first deadline miss is exact, and\nthat a non-positive period is refused at check. Scope: FR-249-AC-7 to\nFR-249-AC-10."
    },
    {
      "id": "FR-249-AC-7",
      "path": "spec/functional/FR-249-lower-a-task-automaton-to-a-timed-subject.md",
      "role": "context_only",
      "excerpt": "Task set `Pair` in `ms`: `a` periodic 4, deadline 3, `bcet` = `wcet` = 1, priority 1; `b` sporadic 6, deadline 5, `bcet` = `wcet` = 2, priority 2; each released by its own always-enabled operation of a `time dense` model. `schedulable PairFp on Pair under fixed_priority;` is release-backed: unnamed its candidate set is `[en-7]`, and its item with `backend` `en-6` has candidates `[en-6]` and proves with an accepted certificate. The lowered subject's task-timing rows, and the certificate's `task_product.queue_bounds`, are in order `(Pair, a, periodic, interarrival 4, relative_deadline "
    },
    {
      "id": "FR-249-AC-9",
      "path": "spec/functional/FR-249-lower-a-task-automaton-to-a-timed-subject.md",
      "role": "context_only",
      "excerpt": "The refuted EN-6 item of `B` with `t3` work 4 under `fixed_priority` carries a model-arm counterexample whose `prefix[trace_position]` is the product's earliest deadline miss: `t3`, job 0, `demand` 4, absolute `deadline` 10, at elapsed time 10 with completed service 3, every task having activated at time 0. No earlier step is a `deadline-miss`, and no later step of the prefix or loop is one. No behaviour misses earlier, since `t1` and `t2` meet every deadline and a `t3` miss needs 10 ms after its activation. The prefix's last step is `activate` of `t3` job 1 at elapsed 10. `loop` is "
    },
    {
      "id": "AGENTS.md",
      "path": "AGENTS.md",
      "role": "examined",
      "excerpt": "A `#[trace]` tag with a bare id (`TC-196`, `FR-075-AC-4`) names an FSL\nartifact. A tag that names a filament-specification requirement or test case\ncarries the `FSpec-` prefix (`FSpec-TC-196`, `FSpec-FR-151-AC-2`), because\nthe two repositories number their artifacts independently and the same id\nnames different artifacts in each."
    },
    {
      "id": "fsl-analyze/src/analyze/task_product_item.rs",
      "path": "fsl-analyze/src/analyze/task_product_item.rs",
      "role": "examined",
      "excerpt": "fn prove<'a>("
    },
    {
      "id": "fsl-analyze/src/zone_check.rs",
      "path": "fsl-analyze/src/zone_check.rs",
      "role": "examined",
      "excerpt": "pub(crate) use settle::stop_cause;"
    },
    {
      "id": "fsl-analyze/src/zone_check/certificate.rs",
      "path": "fsl-analyze/src/zone_check/certificate.rs",
      "role": "examined",
      "excerpt": "pub(super) fn proving<O>(self, obligation: O) -> ZoneCertificate<O> {"
    },
    {
      "id": "fsl-analyze/src/zone_check/search.rs",
      "path": "fsl-analyze/src/zone_check/search.rs",
      "role": "examined",
      "excerpt": "    SourceClosure,"
    },
    {
      "id": "fsl-analyze/src/zone_check/settle.rs",
      "path": "fsl-analyze/src/zone_check/settle.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn stop_cause(cause: ZoneStop, cancel: &Cancel) -> Option<IncompleteCause> {"
    },
    {
      "id": "fsl-analyze/src/zone_check/task_certificate.rs",
      "path": "fsl-analyze/src/zone_check/task_certificate.rs",
      "role": "examined",
      "excerpt": "pub fn no_release_task_product_certificate("
    },
    {
      "id": "fsl-eval/src/simulation/model/timed.rs",
      "path": "fsl-eval/src/simulation/model/timed.rs",
      "role": "examined",
      "excerpt": "    pub fn clock_place_identity(&self, place: usize) -> Option<String> {"
    },
    {
      "id": "fsl-replay/src/certificate/task_product.rs",
      "path": "fsl-replay/src/certificate/task_product.rs",
      "role": "examined",
      "excerpt": "    array, child, count, integer, member, object, rational, rational_wire, refuse,"
    },
    {
      "id": "fsl-replay/src/certificate/task_product/wire.rs",
      "path": "fsl-replay/src/certificate/task_product/wire.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn rational(value: &Value, pointer: &str) -> Result<Rational, TaskProductShapeRefusal> {"
    },
    {
      "id": "fsl-replay/src/certificate/zone_certificate.rs",
      "path": "fsl-replay/src/certificate/zone_certificate.rs",
      "role": "examined",
      "excerpt": "                        (Some(own), Some(listed)) if self.release_ops.is_some() => {"
    },
    {
      "id": "fsl-replay/src/certificate/zone_certificate/wire.rs",
      "path": "fsl-replay/src/certificate/zone_certificate/wire.rs",
      "role": "examined",
      "excerpt": "    pub fn to_wire_with_source("
    },
    {
      "id": "fsl-analyze/tests/it/schedulability.rs",
      "path": "fsl-analyze/tests/it/schedulability.rs",
      "role": "examined",
      "excerpt": "fn the_original_task_set_b_retains_its_response_time_results() {"
    },
    {
      "id": "fsl-analyze/tests/it/task_automaton_lowering.rs",
      "path": "fsl-analyze/tests/it/task_automaton_lowering.rs",
      "role": "examined",
      "excerpt": "fn an_unreachable_checked_release_proves_with_the_source_certificate() {"
    },
    {
      "id": "fsl-analyze/tests/it/task_product_certificate.rs",
      "path": "fsl-analyze/tests/it/task_product_certificate.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn no_release() -> World {"
    },
    {
      "id": "fsl-analyze/tests/it/task_product_producer.rs",
      "path": "fsl-analyze/tests/it/task_product_producer.rs",
      "role": "examined",
      "excerpt": "fn clockful_no_release_transport_preserves_context_and_rejects_tampering() {"
    },
    {
      "id": "FSpec-FR-418-AC-14",
      "path": "filament-specification/spec/objects/interfaces/FR-418-zone-certificate.md",
      "role": "examined",
      "excerpt": "A task-product certificate may use `closed-no-release-product/v1` only with `live_job_cap: 0`, closure `recompute-all-source-successors-no-task-release/v1`, checked task demand intervals, no scheduler clock metadata and no task-transition edge; the reader recomputes every source successor from every initial state and accepts this closure only when no checked release binding is reachable. It refuses a positive cap, scheduler clocks, or a reachable release under this method."
    },
    {
      "id": "FSpec-FR-418-AC-20",
      "path": "filament-specification/spec/objects/interfaces/FR-418-zone-certificate.md",
      "role": "examined",
      "excerpt": "Checking an accepted fixed-work task-product certificate with `zone_check.max_check_work` set below the work its exhaustive exploration needs settles `incomplete` with `limit-reached` naming `max_check_work` and the setting `zone_check.max_check_work`, never `proved`; with the setting at its published default, 100000000 work units (FR-461), the same certificate settles `proved`."
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "medium",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "trace",
      "artifact_id": "FSpec-FR-418-AC-20",
      "related_ids": [
        "FSpec-FR-418-AC-14"
      ],
      "path": "fsl-analyze/tests/it/task_product_producer.rs",
      "lines": "292-318",
      "excerpt": "Checking an accepted fixed-work task-product certificate with `zone_check.max_check_work` set below the work its exhaustive exploration needs settles `incomplete` with `limit-reached` naming `max_check_work` and the setting `zone_check.max_check_work`, never `proved`; with the setting at its published default, 100000000 work units (FR-461), the same certificate settles `proved`.",
      "finding": "The new source-producer cancellation test claims FR-418-AC-20, but sets max_symbolic_states to zero and checks cancellation; it never checks an accepted fixed-work certificate or max_check_work. It would still pass if the entire AC-20 behavior regressed. Remove this incorrect AC-20 claim or add the actual checker/request budget assertions; retain the existing genuine max_check_work test.",
      "test_id": "the_no_release_producer_preserves_source_limits_and_cancellation",
      "claimed_ac": "FR-418-AC-20",
      "correct_ac": "none (source-producer cancellation and max_symbolic_states are not AC-20)"
    }
  ],
  "bindings": [
    {
      "test_id": "the_no_release_producer_preserves_source_limits_and_cancellation",
      "ac_id": "FR-418-AC-20",
      "trace": "wrong"
    }
  ]
}
```

