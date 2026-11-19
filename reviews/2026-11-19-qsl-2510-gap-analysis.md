---
id: SR-2026111612172400000053
title: "Gap analysis of QSL-2510 verification_plan reader and outcome writer (filament-spec-language PR 2996)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@b4354ddca1d7e5d85c54af33a11567522086fe45; git diff origin/main...HEAD (PR 2996); FSL FR-1038 AC-1, AC-2 (TC-3018); FSpec FR-824 AC-1..AC-5 (TC-2039, TC-2045) at agent-ix/filament-specification origin/main"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2510 (parent QSL-2502). AC-to-test check over the diff's tagged tests and code.
`quoin matrix --repo . --json` did not finish within 120 s in this worktree, so the computed
matrix was unavailable and this is a manual check. Plan completion: not assessed. Partial
delivery is intended: selection, emission and execution are blocked on QSL-2508, QSL-2509 and
QSL-2511, and only the delivered reader and writer are judged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Coverage inflation: the writer's unit tests carry `FR-1038-AC-1`, `FSpec-FR-824-AC-1` and `-AC-5` but build `StepDisposition`s by hand; the AC-5 test chooses the blocked dependent itself (`if dependent_runs ... else Blocked`), so the gating the AC states is decided by the test, not by code under test. A matrix reading these tags shows the ACs covered while the end-to-end `executing_a_plan_blocks_only_descendants_of_a_failed_step` stays ignored | fsl-command/src/command/plan/outcome.rs:248-347 |
| FND-002 | medium | FSpec FR-824-AC-3 is tagged on the active it test, which covers 3 of its 6 clauses (`seed` `"7"`, both-false, `engine`). `seed` on a `model_check`-only plan (`PlanRequest::close_by_plan`) has no caller and no test at all, and the package-route `document` clauses (`missing-member`, other-package `invalid-value` at `/document`) are untested and misrouted (SR-2026111612172400000052 FND-001) | fsl-command/src/command/plan/request.rs:56-72; tests/it/request_surface_s1_16.rs:432-479 |
| FND-003 | low | The ignore reasons still cite the umbrella QSL-2236, not the blockers QSL-2508, QSL-2509 and QSL-2511; and `the_verification_plan_member_selects_one_plan_or_refuses` says "selection and request members not implemented yet" though this PR delivers the request members (its three request cases moved to the active test), so only selection is still missing | tests/it/request_surface_s1_16.rs:364; tests/it/request_surface_s1_16.rs:402; tests/it/request_surface_s1_16.rs:488 |
| FND-004 | medium | The two writer unit tests tagged `FSpec-FR-824-AC-4` (`an_unsupported_step_leaves_the_plan_incomplete_with_no_failed_step_and_exit_21`, `the_exit_is_the_highest_severity_code_in_fr_301_order`) assert only status, failed_steps and exit; nothing checks negotiation, the `unsupported_projection`/`unsupported-requested-capability` diagnostic, that no engine runs, or that the blocked step is not negotiated | fsl-command/src/command/plan/outcome.rs:284-347 |
| FND-005 | medium | No test is tagged `TC-3032`/`FR-1038-AC-3` or `FSpec-TC-2046`/`FSpec-FR-824-AC-6`, which arch ruled for these rules (language PR 2997, FSpec PR 724). The AC-3 clauses are untested: `seed` `7.5` and `18446744073709551616` (only `-1` is driven), `"verification_plan": null` at exactly `/request/verification_plan`, null members as absent with `inputs` `{"initial": null, "seed": 0}`, the measured disposition's exact members, and emit-only `status` `null` | tests/it/request_surface_s1_16.rs:432-479; fsl-command/src/command/plan/outcome.rs:248-347 |

## Scope and evidence

Examined (clean unless listed above):
- FR-1038-AC-2, `seed` `"7"` refuses `wrong-value-kind` at `/verification_plan/seed`, exit 20,
  no outcome: covered by the active `the_verification_plan_reader_refuses_a_malformed_member_at_its_pointer`.
  The other clauses (one plan runs, two plans `missing-member`, `Nope` and `Never`
  `missing-name`) need selection and stay ignored, correctly.
- FR-1038-AC-1: execution and emit-only need QSL-2509/QSL-2511; the end-to-end test stays
  ignored, correctly; only the writer half is delivered (FND-001).
- FSpec FR-824-AC-1: writer half (status, failed_steps, inputs, blocked_by, null result,
  emit-only null dispositions, exit 22 and 0) covered by unit tests.
- FSpec FR-824-AC-2: selection, blocked; no delivered code claims it.
- FSpec FR-824-AC-3: partial (FND-002).
- FSpec FR-824-AC-4/AC-5: writer aggregation (status, failed_steps, exit 21/22/0) covered;
  negotiation and gating not delivered (FND-004, FND-001).
- FR-1038 Behavior "strict reader before any stage runs": covered (refusal exits 20 with
  no outcome over a unit that would otherwise compile).
- Code with no owning requirement: none; every new item cites FR-1038 / FSpec FR-824.
- Stubs: none in production code; the remaining stubs are `#[ignore]` with reasons (FND-003).

## Arch rulings applied

The coordinator relayed arch rulings after the first post. FND-005 records the ruled AC
(FR-1038-AC-3 / FSpec FR-824-AC-6), which this PR does not tag or test yet. The specs are open
PRs, language PR 2997 and FSpec PR 724.

## Verdict

The delivered reader is covered for its active clauses and the writer's aggregation is
exercised. The gaps are trace accuracy (FND-001, FND-004: tags that claim coordinator behaviour
the writer tests do not exercise), the untested `close_by_plan` and package route (FND-002), and
stale ignore reasons (FND-003). The newly ruled FR-1038-AC-3 / FSpec FR-824-AC-6 has no tagged
tests (FND-005).
