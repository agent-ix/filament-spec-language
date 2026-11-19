---
id: SR-2026111612172400000052
title: "Code review of QSL-2510 verification_plan request reader and outcome writer (filament-spec-language PR 2996)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@b4354ddca1d7e5d85c54af33a11567522086fe45; git diff origin/main...HEAD (PR 2996): fsl-command/src/command.rs, fsl-command/src/command/output.rs, fsl-command/src/command/plan.rs, fsl-command/src/command/plan/request.rs, fsl-command/src/command/plan/outcome.rs, tests/it/request_surface_s1_16.rs"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2510 (parent QSL-2502). Code review with the rust-review lane folded in, over
the PR diff only. Partial delivery is intended: plan selection, emission and execution are
blocked on QSL-2508, QSL-2509 and QSL-2511 (not on main), so the outcome writer is not wired
and the selection and execution tests stay ignored. Judged on what is delivered: the strict
`verification_plan` reader wired before any stage, the `RunCause::PlanRequest` envelope, and
the `filament.verification-plan-outcome/v1` writer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `check_member` always reads with `Route::Source`, so on a package-route run (`request.package`, FR-1039) a `verification_plan.document` that FSpec FR-824 requires refuses `unknown-member` with the false reason "a source run emits its plan documents and takes none", and a package-route member without `document` passes the reader instead of refusing `missing-member` at `/document` | fsl-command/src/command/plan.rs:18-31 |
| FND-002 | medium | The writer adds a sibling `decision` member to a `measured` disposition. Arch ruling (4) (FR-1038-AC-3 in language PR 2997, FSpec FR-824-AC-6 in FSpec PR 724): a disposition has exactly `status`, `method`, `evidence`, `blocked_by`, `result`, with the decision only at `result.verdict.decision.type`. The PR deviates, and its unit test asserts the extra member | fsl-command/src/command/plan/outcome.rs:198-214; fsl-command/src/command/plan/outcome.rs:300-323 |
| FND-003 | medium | Arch ruling (1) keeps the `/request/verification_plan<FR-824 pointer>` form the code writes, and says tests assert `details.path` exactly. The it test checks cause and pointer by substring over the whole document (`refusal_names`), so it cannot catch a wrong prefix or a pointer that only shows up in a message | tests/it/request_surface_s1_16.rs:331-336; tests/it/request_surface_s1_16.rs:470-477 |
| FND-004 | medium | Arch ruling (2): a `null` member inside the object means absent. The reader refuses `wrong-value-kind` for `"seed": null`, `"emit": null` and `"initial": null`, so FR-1038-AC-3's `{"plan": "Release", "seed": null, "emit": null, "initial": null}` is refused. A top-level `"verification_plan": null` already refuses `wrong-value-kind` at `/request/verification_plan`, which matches the ruling | fsl-command/src/command/plan/request.rs:91-122 |
| FND-005 | medium | Arch ruling (5): an emit-only run (`execute: false`) writes `status` `null` with exit `0`. `PlanRunOutcome::status()` returns `"incomplete"` when `dispositions` is `None`, and no test asserts the status | fsl-command/src/command/plan/outcome.rs:150-153 |
| FND-006 | low | `pub mod plan` and its `pub` items (`outcome::*`, `PlanRequest::close_by_plan`, `effective_seed`, `Route::Package`) have no production caller; making them crate-public adds unstable public API to `fsl_command::command` and keeps `dead_code` quiet without wiring. `pub(crate)` plus wiring in QSL-2511 would be the narrower surface | fsl-command/src/command.rs:12; fsl-command/src/command/plan.rs:6-7 |

## Scope and evidence

Examined (clean unless listed above):
- Reader member list closed (`plan`, `emit`, `execute`, `initial`, `seed`, `document`);
  unknown member `unknown-member` at its pointer; kinds `wrong-value-kind`; `emit` and
  `execute` both false `invalid-value` at `/execute` (FSpec FR-824 "Request").
- `seed`: any JSON number that is not a u64 (negative, fractional, exponent, too large; serde_json
  reads the last three as f64) refuses `invalid-value`, and a non-number refuses `wrong-value-kind`.
  This matches arch ruling (3).
- `RunCause::PlanRequest` maps to `invalid_runtime_input`, stage `request`, `details.path`, FR-805
  cause, sharing the `ExportRequest` arms; exit `20`; no `verification_plan` outcome on refusal.
- Reader runs before `export::run_bytes` and every stage, only for `native-run/1`; non-run
  formats and runs without the member are untouched.
- Outcome writer: closed status vocabulary; `satisfies_requires` (`proved`, `tested`,
  `measured`/accepted), `is_failure` (`refuted`, `failed`, `measured`/rejected); categories
  per FSpec FR-824's table; exit through `Category::most_severe`, whose order at the locked
  filament-semantic-value rev 3fd65564 is 30, 20, 21, 22, 10, 11, 0 (FSpec FR-301, FSL FR-285).
- No `#[allow]` in the diff; tests tagged with `#[trace(...)]`.
- Context (pre-existing, not in this diff): `fsl-command/src/command/monitor.rs:218-227`
  `severity_exit` ranks 30, 20, 10, 11, 21, 22, contradicting FR-285's stated order
  (FR-285-AC-2: {10, 22} exits 22; `severity_exit` gives 10). The spec is unambiguous, so
  this is a separate bug, QSL-1977 (arch is filing it), and is not to be fixed in QSL-2510.

## Arch rulings applied

The coordinator relayed arch rulings for this PR after the first post (spec: language PR 2997
FR-1038-AC-3/TC-3032, FSpec PR 724 FR-824-AC-6/TC-2046). They were folded in before any fix round:
FND-002, FND-004 and FND-005 now state the ruled behaviour, and FND-003 moved from the pointer
form, which the ruling keeps, to how the test asserts it.

## Verdict

The reader is strict, small and follows the export reader's shape. The writer's aggregation
(failed_steps, exit) matches FSpec FR-824 and FR-301. Against the arch rulings, the PR deviates in
four places: the package route (FND-001), the sibling `decision` member (FND-002), `null` treated as
present (FND-004) and the emit-only status (FND-005). The test does not assert `details.path`
exactly (FND-003). Each is a small fix inside this PR. Not mergeable until they are fixed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | The FND-006 fix (`pub` to `pub(crate)`) makes every unwired plan item dead code: `cargo clippy -p fsl-command --all-targets` now reports 11 new `dead_code` warnings in the diff (`Decision`, `StepStatus` and its methods, `StepDisposition`, `PlanRunOutcome` and its methods, `step_value`, `PlannedStep`, `Settled`, `settle_in_order`, `PlanRequest::effective_seed`/`close_by_plan`). CI runs `clippy ... -- -D warnings`, so these are new errors, and the house rule rules out `#[allow]`. Revert the visibility to `pub` (FND-006 then accepted-no-change) or wire the items | fsl-command/src/command/plan/outcome.rs:11-227; fsl-command/src/command/plan/request.rs:45-72 |
| FND-008 | medium | `settle_in_order` settles steps in source order and only looks for blockers among steps it has already settled. FSpec FR-192 lets `depends` name a later step (it refuses only cycles) and has the coordinator dispatch topologically. A step whose prerequisite comes later in the source is therefore never blocked, and runs before its prerequisite. Settle in topological order, and keep `blocked_by` as the first unsatisfied prerequisite in source order. Not wired yet, but QSL-2511 would inherit it | fsl-command/src/command/plan/outcome.rs:227-264 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | rejected | withdrawn by the reviewer. Until QSL-2511 wires these items, `pub` is what keeps them clean under `-D warnings` without `#[allow]`; the `pub(crate)` fix produced FND-007 |
