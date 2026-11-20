---
id: SR-2026112022042102
title: "code-review of PR 3058 verification plan coordinator, per-method dispatch and native-run plan member (QSL-2565, QSL-2511, QSL-2236)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3058 head d14af1a824cb4db474198cdaf199aa36fe68dad5; fsl-analyze/src/plan.rs, fsl-command/src/command/plan.rs, fsl-command/src/command/plan/outcome.rs, fsl-command/src/command/plan/request.rs, fsl-command/src/command.rs, fsl-command/src/command/package_run.rs, fsl-package/src/verification_plan.rs, fsl-eval/src/simulation/model.rs and the changed tests"
review_set: subset
---

## Summary

Ticket: QSL-2565 (QSL-2511 and QSL-2236 ride the same PR). PR filament-spec-language#3058, 7 commits, +3066/-286 in 18 files. This is a diff-only code review with the rust-review lane folded in. The gap check against FSL FR-753-AC-1..5 and FR-1038-AC-1..4, and against FSpec FR-824 and FR-192 (read at filament-specification origin/main cd89fcb319, because the local checkout at 43f0806ad predates FR-824), was done by hand in this pass. No build or test run.

No earlier review of #3058 is recorded on QSL-2565, QSL-2511 or QSL-2236. The QSL-2511 blocker note of 2026-11-19 (the `model_based_test` route returned an integration fault) is resolved at this head: the step now dispatches to `run_model_based_test`.

## Verdict

FIX ROUND. Two highs, four mediums, one low.

What holds:

- Preflight order matches FR-753 "PlanRefusal": `read_verification_plan` (reader refusal, then package identity) → `execution_order` (cycle) → `plan_consumers` (`initial` admission) → `max_trace_length` → `max_value_bits`. The tests cover each pair (`reader_refusal_precedes_identity…`, `package_identity_precedes_missing_inputs…` with `Some(8)` and `None`, `initial_admission_precedes_missing_generation_bounds`, `missing_value_size_follows_trace_bound…`).
- The `requires` gate. `execution_order` settles every prerequisite before its dependents. `settle_in_order` blocks a node when any `requires` prerequisite settled without `satisfies_requires()` (proved, tested or measured-accepted only). It names the first such prerequisite in source order, and never calls `run` for the node, so the node is not negotiated. `Blocked` does not satisfy, so blocking is transitive to every descendant, and a node with no failed ancestor is never blocked. The reader admits only `requires` edges.
- Negotiation vs local dispatch matches the FR-753 table: `model_based_test` (FR-752 entry) and `simulate` (simulation item) are never negotiated. Every other method builds a `ClaimItem` and calls `negotiate`.
- Plan verdict: `status()` is `passed` only when every node satisfies `requires`, and `failed` when any node is refuted, failed or measured-rejected. The exit is the most severe FR-301 category over all nodes. So the plan is never better than its nodes, but FND-002 shows a node itself can be better than its evidence justifies.
- Seed, `max_trace_length` and `max_value_bits` reach only consuming steps. `close_by_plan` closes the member set in FR-824 table order, after `initial` admission.
- No Python and no logic-bearing shell.

Test changes the coder made "to match the ACs":

- The emit-only request gains `"execute": false`. FR-1038-AC-1/AC-3 and FSpec FR-824-AC-1/AC-6 spell exactly `{"plan": "Release", "emit": true, "execute": false}`. Not bent.
- `max_trace_length: 8` on the execute and null-member requests. FR-1038-AC-1 and AC-3 spell it. Not bent.
- The initialization-anchored `initial` in place of `spine-snapshot.json`. This follows FR-824 (`initial` is the state a step starts from), and the handler snapshot is rightly refused `wrong-anchor`. It does depart from FR-1038-AC-1's literal file and `inputs` echo, and the test now asserts neither (FND-007). The same test carries `FSpec-FR-824-AC-1`, whose request has no `initial`, and that case does not pass (FND-001).
- The stub-fault assertions are replaced with real FR-190 records. That is legitimate now that QSL-2237 landed. One replacement accepts either verdict (FND-005).

Not verified without a build: that the head tests pass; whether `ModelSystem::over_model` succeeds with an empty `initial` (if it does not, the `Sized` run of FR-1038-AC-4, which has no `initial`, never classifies `s` as a `max_value_bits` consumer); and the 3 red `request_surface_s1_16` tests the PR attributes to main.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A step that consumes `initial` and gets none is refused by the FR-752 entry (`EmptyInitial`) and settles `failed`. That puts it in `failed_steps`, makes the plan `failed` and the exit 30. FSpec FR-824 says such a step starts from every state its `domain` admits. FSpec FR-824-AC-1 (`{"plan":"Release","seed":7,"max_trace_length":8}`, `inputs.initial` null, `tested` `tested`) therefore fails. FR-1038-AC-3's null-`initial` run settles every step `failed`, and its test checks only `inputs`. The Sized run of FR-1038-AC-4 (no `initial`) cannot produce its record. | fsl-analyze/src/plan.rs:527-540, fsl-analyze/src/plan.rs:566-572, fsl-command/src/command/plan.rs:187-191, tests/it/request_surface_s1_16.rs:405-444, tests/it/request_surface_s1_16.rs:529-550 |
| FND-002 | high | A `simulate` step settles `tested` whenever the enumeration is exhausted. `tested` satisfies `requires` and counts toward `passed`, but the simulation item never evaluates the step's claim (`subject: None`, claim unused), so `check s claim Never method simulate` passes the plan. The test `successful_simulation_satisfies_a_prerequisite_without_negotiation` asserts this rule. | fsl-analyze/src/plan.rs:844-879, fsl-analyze/tests/it/plan.rs:270-299 |
| FND-003 | medium | Any FR-752 request refusal settles the step `failed` (FR-301 internal failure, exit 30), always coded `invalid_runtime_input`, whatever the entry's cause. An `invalid_model_binding/malformed-declaration` empty domain is one example. FR-824 says a step settles as the same item does outside a plan, and there it is a request refusal. | fsl-analyze/src/plan.rs:566-572 |
| FND-004 | medium | Three per-step failures abort the whole run as an internal fault (`RunCause::AnalyzeFault`, exit 30, no outcome document) instead of settling that one step. They are: a method whose kind does not apply to the claim (`ClaimItem::new` → `inapplicable-plan-claim`, reachable because S3 only refuses `schedulability` on a temporal claim), an `analyze` refusal of the item's binding, and a simulation `Binding` failure. FR-824 says a step that runs no engine never stops a sibling. | fsl-analyze/src/plan.rs:704-717, fsl-analyze/src/plan.rs:748-749, fsl-analyze/src/plan.rs:865-867 |
| FND-005 | medium | `finite_model_generation_consumes_no_value_size_and_runs_on_the_model_based_test_entry` accepts `Tested \| Refuted`, so it passes whichever verdict the entry returns and cannot detect a wrong one. | fsl-analyze/tests/it/plan.rs:703-706 |
| FND-006 | medium | The plan run uses `AnalyzeLimits::unbounded()` and never reads the run request's `limits` member or deadline. The `native-analyze/1` route applies both through `analyze_limits`. A plan run cannot be bounded by its caller, and a supplied `limits` is silently ignored. | fsl-command/src/command/plan.rs:210 |
| FND-007 | low | The execute test does not assert FR-1038-AC-1's full `inputs` echo (`initial`, `max_value_bits: null`) or `tested`'s status. The AC's literal `"initial": "spine-snapshot.json"` names the fixture's handler-anchored snapshot, which admission refuses. | tests/it/request_surface_s1_16.rs:430-444, spec/functional/FR-1038-read-the-verification-plan-member-and-run-the-selected-plan.md:82 |

Fixes:

- FND-001: when `initial` is absent, build the model-based-test (and model_check/simulate/contract_check) subject from the initial states the step's `domain` admits, as FR-190 draws them. If FR-752 cannot do that yet, settle the step `unsupported` with a stated cause. It must never be `failed`.
- FND-002: settle an exhausted claim-free simulation as non-satisfying (`inconclusive`), unless the run evaluates the claim. Map the simulation's refusal and internal-failure categories to `declined` and `failed` instead of `incomplete`, and invert the test.
- FND-003: carry the entry's own code and cause into the step's result, and settle a request refusal as the same item does outside a plan (`declined`), not `failed`.
- FND-004: settle the step instead of faulting: `unsupported` with `invalid_capability`/`inapplicable-kind`, or the analyze or admission refusal as that step's record. Then continue to the siblings.
- FND-005: assert the one status the counter fixture determines.
- FND-006: build the plan's `AnalyzeLimits` with `analyze::analyze_limits(&request.limits, deadline)`, as the analyze route does.
- FND-007: assert `inputs == {"initial":"initial.json","seed":7,"max_trace_length":8,"max_value_bits":null}` and `tested` `tested`, and amend FR-1038-AC-1 to name an initialization-anchored snapshot.

## Spec questions

(a) A `model_based_test` step with no `bound`. FSpec FR-192 says the absence "requests the method's unbounded extent, which FR-290 negotiation settles". A language method is not negotiated, and FR-190 has no run without `max_cases`, so FSpec is silent here. The code settles `unsupported` with `unsupported_projection`/`requires-bound`, runs no case, blocks dependents, and keeps the plan from `passed` (exit 21). This is the conservative reading and mirrors FR-290's bounded-only arm, so it is accepted for this PR. One nit: FR-290 pairs `unsupported_projection` with `unbounded-extent`, and uses `requires-bound` only as a disposition. FSpec should state the case, either as an S3 refusal of a bound-less `model_based_test` or as the settlement FR-824 assigns. That is a spec follow-up, not a code defect.

(b) A consuming step with no `initial`. FSpec FR-824 is explicit: the step "starts from every state the step's `domain` admits". The FR-752 entry's `EmptyInitial` refusal is an FSL limitation, not a spec ambiguity. Settling `failed` is wrong twice: the step does not run, and it reports internal failure. This is FND-001.

## Package route

A selected-package run with `verification_plan` refuses "not implemented yet" (package_run.rs:147). The reader still refuses a missing `document` at `/request/verification_plan/document` first. No FR-753 AC is left unmet by this gap: AC-1..AC-5 are source-route or direct-coordinator criteria, and AC-5's identity refusal is met at `execute_plan`. Unmet: FSpec FR-824-AC-3's "with a plan document of another package `invalid-value` at `/document` naming both identities", the package-route paragraph of FSpec FR-824 "Selection", and FR-1038's selection of the plan for a selected package. FND-001 separately leaves FSpec FR-824-AC-1, FR-1038-AC-3's run and FR-1038-AC-4's Sized run unmet.
