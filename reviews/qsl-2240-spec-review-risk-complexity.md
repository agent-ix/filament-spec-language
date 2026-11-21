---
id: SR-20261121181100006
title: "QSL-2240 spec-review-risk-complexity review"
type: SpecReview
analysis: risk-complexity
scope: "agent-ix/filament-spec-language; PR 3349; FR-249-AC-8, TC-3008 and the source-closure/transport diff"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-249
---

## Summary

Frozen PR head and local HEAD both 1e91c6084f651d85ad2139fe64b1de11885dc463; clean working tree; origin/main merge base 49f9e7a18cce9e02769ce0a44e4ed7e69f272bdd. Ticket QSL-2240; PR filament-spec-language#3349. No source edits, pushes, merges, relay messages, full gates, whole-crate tests, Python, or duplicate heavy benchmark runs.

## Verdict

CONDITIONAL. No new defect found; acceptance remains conditional on the unresolved execution evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Risk register: FR-249 technical risk high (finite region/product exploration and resource growth), volatility low (explicit authorized fixture/closure contract); mitigation: retain honest typed incomplete result, actual named-case measurements and independent checking. No new hard performance guarantee is introduced by retiming. Main hazards: unmeasured default-bound name; three unresolved reduced proof cases; historical binary vs current-head provenance. Existing frozen229 runs should finish or be resource-classified before any replacement current-head run is considered; never infer completion from admission or resource cancellation.

Current-head evidence: source-transport-context-resume-1e91.log exit 0, two named tests passed (request-level unreachable checked release and clockful context/tamper); source-transport-finite-regression-1e91.log exit 0, three named tests passed (fixed-work single-task proof, refutation before symbolic pass, low max_check_work typed stop). Historical cheap7: six passes, one genuine codec failure superseded by the current-head two-test success. Frozen229 FP4: one pass, refuted and replay true, 0.40 s. Frozen229 FP3/EDF3/EDF4 remain unresolved measurements, not current-head qualification. Original B resource-cancelled without an engine verdict; max_check_work is an unmeasured expectation. No full-engine or repository PASS is claimed. Existing clippy logs are historical baseline failures, not a successful current-head gate.

Whole-repository `quoin matrix --repo . --json`, static `quire matrix --scope . --format json`, `quire coverage --scope . --json` and `quoin advise --json` were attempted. They produced no JSON after approximately 5 minutes for matrix and 4.5 minutes for the other commands, and this reviewer terminated only those owned metadata processes (143). Catalog methods loaded successfully. No computed criterion counts or advisor recommendations are available; this is an extraction/evidence limitation, not a new implementation defect. No engine-backed whole-repository coverage PASS is claimed. All changed production/test files were inspected. Broader source inventory is bounded to the PR; this review does not claim an exhaustive repository audit.

## Examined scope

```yaml
{
  "clean": true,
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
      "id": "FSpec-FR-418-AC-14",
      "path": "filament-specification/spec/objects/interfaces/FR-418-zone-certificate.md",
      "role": "context_only",
      "excerpt": "A task-product certificate may use `closed-no-release-product/v1` only with `live_job_cap: 0`, closure `recompute-all-source-successors-no-task-release/v1`, checked task demand intervals, no scheduler clock metadata and no task-transition edge; the reader recomputes every source successor from every initial state and accepts this closure only when no checked release binding is reachable. It refuses a positive cap, scheduler clocks, or a reachable release under this method."
    }
  ],
  "findings": [],
  "bindings": []
}
```

## Latest review state (disposition round 1)

Reviewed local and pushed PR head `909adbbf2bc8b73a91853fbcb0f3546f145133fe` independently; worktree clean. Compared every Rust change against initial reviewed head `1e91c6084f651d85ad2139fe64b1de11885dc463`: exactly six trace attributes changed, with no runtime code, fixture, assertion, limit or algorithm change. The remaining diff adds the seven original review artifacts. This round normalizes only scratch artifact EOF whitespace, for the orchestrator to copy verbatim later.

Latest verdict: CONDITIONAL solely on retained evidence limits. No new finding in this round. All source findings from the initial review are explicitly fixed: the new external tags are qualified and the false AC-20 claim is removed; the existing genuine max_check_work test remains intact. This does not qualify original B's first default bound or the reduced proving quartet. Frozen229 FP3/EDF3/EDF4 remain running/unresolved per the orchestrator; original B was resource-cancelled without a typed engine verdict or known exit receipt. No completion or typed bound is inferred.

Change impact: inspected the actual frozen229-to-current diff. The source-aware transport is selected only for ClosedNoReleaseProduct; the reduced fixtures have reachable releases and take read_back_unchanged with source=None. The finite symbolic producer and finite checker branch are unchanged. The shared read_back implementation is not byte-identical: it additionally compares LU bounds, edge counts, endpoints, part, horizon and silent metadata. Finite certificates construct empty LU bounds and absent part/horizon/silent metadata; the ordinary finite writer/reader preserves endpoints and edge order, so those added equality checks hold for these fixtures. The current 1e91 finite regression's three passes independently exercise this retained adapter route. FP4 concrete refutation exits before prove. Retained frozen229 measurements therefore remain relevant with this explicit static change-impact argument and source/binary provenance; they are not current-head executions. This fix round changes trace annotations only, so no Cargo rerun is justified. No matrix, coverage, advisor, heavy-runtime or full-gate command was repeated. FSpec PR 799 merged at `875fcaada84fd0c7769f1d1a53be913c6ba0ac25`; no runtime spec-pin dependency was introduced.

The original Findings table's one placeholder is required by Quire CR-010 for valid file structure only; it is not an actual finding. Initial comment data remains clean: true, findings: [].
