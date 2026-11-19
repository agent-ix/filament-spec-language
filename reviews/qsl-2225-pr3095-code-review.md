---
id: SR-2026111919340478491681405
title: "Code review — QSL-2225 PR #3095 timed await admission and projection"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/protocol_projection.rs; fsl-package/src/emit/tests/protocol_compensation.rs; FR-892-AC-1..4; FR-575-AC-1; TC-2525"
review_set: subset
---

## Summary

Reviewed PR #3095 at `d95c0834561321d6df9deca737e318fe78701def`, limited to its two-file Rust test diff for QSL-2225. The formal FR-892 acceptance tests now run without ignores and assert the await's emitted clock binding, but a newer QSL-2225 comment adds two original-source regression cases not exercised by this diff.

## Verdict

**CONDITIONAL** — Add focused coverage for the two accepted original-source cases recorded in Linear comment `86dcda9e-fd7c-46c4-b12e-217fd91d5773`: the public compile request with a timestamped-event clock for the protocol await, and the original Boolean-result attempt/compensation source whose emitted handoff must pass strict IR admission. No production or CI files changed. `git diff --check origin/main...HEAD` passed. The PR reports its focused tests passing; the reviewer did not rerun them while the coder's targeted package checks are in progress. GitHub reports the CLA internal check and gate passing, the duplicate CLA check skipped, and no Rust test check in this PR run.

## Examined Scope

- **FR-892-AC-1 — examined:** The staged saga with fan-out, join, timed compensation and timed await compiles with the configured clock; the package contains both declarations, the `requires temporal` edge, and is admitted. `a_compensation_and_a_timed_await_emit_together` now asserts the await node's matching `temporal_bindings` row, clock name/configuration, verified readback and no omissions. `a_timed_await_package_is_admitted_at_its_binding_row` checks strict-reader admission and one binding row.
- **FR-892-AC-2 — examined:** The protocol run settles deadlock freedom and reaches recovered compensation and timeout states. Existing `a_branching_compensated_choreography_runs_end_to_end` exercises these assertions; the PR reports it passed.
- **FR-892-AC-3 — examined:** Bounded projection returns an await action with its authored window and clock. `a_timed_await_projects_through_the_clock` asserts `Await`, window `(0, 1)`, and clock `c`; this PR removes its ignore.
- **FR-892-AC-4 — examined:** Missing compensation and await clock selections remain refused with the declaration named. `the_missing_clock_message_names_the_template` and `a_missing_await_clock_names_the_await` cover those refusals; the PR reports both passed.
- **FR-575-AC-1 — examined:** A timed await outside the bounded profile returns `UnsupportedSlug::TimedAwait` at the await node in `a_timed_await_answers_unsupported_when_its_package_is_admitted`; this PR removes its ignore.
- **FR-218-AC-16 / TC-2525 — context examined:** `requires_temporal_is_reachable` checks that the temporal requirement resolves without refusal; the PR reports it passed. TC-2525 ties the staged saga procedure to FR-892-AC-1 through AC-4.
- **QSL-2225 Linear comment 86dcda9e-fd7c-46c4-b12e-217fd91d5773 — examined:** The two accepted hardening cases use `tests/corpus/protocol/await.native` with `await.request.json` and the original Boolean-result compensation source, respectively. Neither fixture/request appears in this PR diff; the new package test constructs a different synthetic event-position-clock source.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The newly accepted original-source regressions are not covered: no changed test compiles the corpus await using its timestamped-event request clock, and no changed test reads the original Boolean-result compensation handoff that previously failed strict IR admission. | fsl-package/src/emit/tests/protocol_compensation.rs:154-229 |

## Review Notes

- The test assertion change correctly locates a `control` node whose operation member is `await`, then requires exactly one binding row keyed to that node with `model-steps` and the expected event-position configuration.
- The former incorrect FR-892-AC-3 trace on the admission-only test is corrected to FR-892-AC-1. The separate bounded projection test remains tagged FR-892-AC-3.
- Only Rust test code changed; no production APIs, dependencies, CI workflows, or fixture copies were added. No duplication finding.
- Targeted test evidence was read from the PR description and dispatching coder report, not rerun by the reviewer. No full `fsl-analyze` integration target or high-memory `protocol_constructs::two_protocols_declaring_one_channel_name_keep_their_queues_apart` test was run.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The fix adds exact corpus regressions for the original await request's timestamped-event binding and the original Boolean-result compensation source's strict handoff admission. |

### FND-001 after excerpt

```rust
        assert!(
            handoff
                .admitted()
                .resolved_graph()
                .nodes
                .iter()
                .any(|node| &*node.semantic_form == "compensation"),
            "the strict reader admits the original compensation handoff"
        );
        assert_eq!(await_bindings[0]["clock_name"], "timer");
        assert_eq!(
            await_bindings[0]["clock"],
            json!({"kind": "timestamped_event", "timestamp_unit": "s"})
        );
```

## Disposition Round 1 Verification

Reviewed PR #3095 at `4fe76af7c33dadabb3f7652da5a346d4f5ac0552`. `tests/it/protocol_source_routes.rs` now compiles `staged/await.request.json`, strict-reads its package, and checks that the await node has one `timer` binding with `timestamped_event` seconds. It also compiles the original Boolean-result compensation source through `handoff()` and requires an admitted graph containing a compensation node. These are the two cases from FND-001. No new code defects were found in the changed tests. No builds or tests were run by the reviewer; `git diff --check` and `quire validate --scope . reviews/qsl-2225-pr3095-code-review.md` passed. The PR's in-repository copy of this artifact is still at the initial-review version and must be updated from this disposition version before merge.
