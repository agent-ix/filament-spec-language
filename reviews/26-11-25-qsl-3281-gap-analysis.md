---
id: SR-967175026302197768900474181941935084852
title: "Gap analysis of filament-spec-language PR 3902 (QSL-3281)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@6920e185ee1aefd037d9ff4a67931c9314110843; PR #3902 diff against origin/main 93ac255c48; fsl-analyze/src/exact_probabilistic/digital.rs, fsl-eval/src/probabilistic.rs, fsl-replay/src/probability_certificate/document/{key,timed}.rs and their tests"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-204
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-498
---
# Gap analysis of filament-spec-language PR #3902

## Summary

Ticket: QSL-3281. Planless gap analysis scoped to the PR's diff. The computed
matrix was read with `quire matrix --scope . --format json` on the ticket
worktree at 6920e185ee (exit 0). No tests were run by the reviewer.

- The dual nested monitor (`digital.rs` `Form::of`, `probabilistic.rs`
  `advance_nested_timed_deadline`, checker `bounded_event`, key admission
  `nested_timed_formula_key`) is owned by FR-204 "Form shapes" and
  FR-204-AC-21 / FR-498-AC-8. Both are `tagged`. FR-204-AC-21 binds
  `a_bounded_nested_punctual_claim_settles_an_exact_verdict` and the two new
  dual tests in `bounded_monitor_profile.rs`. FR-498-AC-8 binds the former.
- FR-498-AC-9 is `tagged`, bound to
  `a_timed_profile_refuses_an_open_window_at_any_depth`, which asserts exactly
  one S3 `unsupported_construct`/`expression-form` diagnostic at `[2 ms,*]`
  for both formulas.
- The `TimedTrace` removal returns the lower-0 Eventually to FR-196's deadline
  monitor (FR-196 key table `["deadline",kind,phase]`). FR-186-AC-7 stays
  `tagged` through the rewritten zero-delay cycle test.
- The replay clock-bound fix has no test of its own. It is exercised by the
  refuted `>= 0.99` arm of the FR-498-AC-8 test, whose refutation goes through
  witness replay of a nested window with inner offset 2.
- No code in the diff lacks an owning requirement. No stub or tautological
  assertion was added.

## Verdict

PASS for the PR scope, with one low coverage gap that the PR body
acknowledges.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-498 statement says a `TimedFormShape` member path carries `formulas` naming the formula nodes when `body` ends in `formula`. The only test of a non-empty `formulas` list was replaced. Nested shapes that still settle `TimedFormShape` (same operator twice, a non-punctual inner window, three levels) keep that path reachable, so it now has no test. The PR body says so. | fsl-analyze/tests/it/digital_clocks.rs:1654-1730, spec/functional/FR-498-carry-a-probabilistic-claim-s-checked-package-identity.md:191-201 |
