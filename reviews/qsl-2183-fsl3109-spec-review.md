---
id: SR-2026111921471500000002
title: "spec-review of QSL-2183 NFR-130-AC-5 and FR-485 caller live-occurrence limit"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3109 at f85f5492; spec/non-functional/NFR-130-bound-evaluator-memory-through-the-work-budget.md (statement, NFR-130-AC-5), spec/functional/FR-485-acquire-and-apply-a-checked-closure-through-the-replay-facade.md (items 6-7, FR-485-AC-19, Dependencies), spec/test-cases/TC-3124, TC-3125; context: fsl-replay/src/application.rs begin_call, fsl-eval/src/value/expression/closure.rs advance, tests/fixtures/checked-closure-consumer, FR-401 item 4, filament-eval FR-013 (PR #43)"
review_set: subset
---

## Summary

Spec review of FSL PR #3109. Ticket: QSL-2183. The PR states one live-occurrence sum per application in NFR-130 (with NFR-130-AC-5), adds `limits` to `ApplicationRegistry::begin_call` and the adapter's `LiveContext` duty to FR-485 items 6-7 with FR-485-AC-19, and adds TC-3124 and TC-3125. `quire validate` passes on all four files. TC-3124, TC-3125, NFR-130-AC-5 and FR-485-AC-19 are free on origin/main (highest TC there is TC-3121) and in all 26 other open PRs. The branch is 3 commits behind main, none touching these files.

## Examined scope

- Current behaviour. `ApplicationRegistry::begin_call` today passes `filament_eval::SessionLimits::default()` (application.rs:909-913) and takes no `limits` argument; the std adapter runs a whole body in one `advance`. Both claims hold. (examined)
- Arithmetic. AC-19: generated p 3, c admitted 7, n inside c 11>10, consumed 7, next_charge 4; interpreted p counts 3+4 in-transition, same stop; two-argument c gives 10 and completes; limit 6 stops at c with consumed 3, next_charge 4. Correct. (examined)
- Failing power. The limit-6 case fails a facade that keeps the default; the limit-10 case fails an adapter that ignores the context (4+4=8 completes). (examined)
- Callers. `begin_call` callers in application.rs unit tests, fsl-replay/tests/it/checked_body_failure.rs and tests/fixtures/checked-closure-consumer must add `limits`; the consumer already depends on `filament-eval` directly, so it can build `SessionLimits` without an FSL re-export. (examined)
- NFR-130-AC-5, TC-3124, TC-3125 against FR-485-AC-19. They are consistent with one another and with FR-013's numbers. (examined)

## Verdict

Mergeable after filament-eval #43, with no findings of its own. Its delivery and `frame_depth` semantics inherit FR-013, so the fixes for #43's FND-001 and FND-002 must keep AC-19's "equal `location` and `frame_depth`" true.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
