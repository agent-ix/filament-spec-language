---
id: SR-2026112105473202
title: "Gap analysis — filament-spec-language QSL-2178 PR #3264 (FR-1040-AC-3 refinement rows)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3264 at bde7f42408d78813ee810b71ab1ed9408b3f7866; FR-1040-AC-3, FR-1040 Refusals that remain, FR-230-AC-11, TC-3084"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-1040", type: references }
  - { target: "ix://agent-ix/filament-spec-language/TC-3084", type: references }
---

## Summary

Ticket: QSL-2178. Planless, PR-scoped gap analysis by reading (no cargo, no matrix run; `git grep` over trace tags and test names at bde7f42408). Plan completion: not assessed. No production code changed, so there is no reverse gap and no stub.

## Verdict

**Partial.** The refinement-row check-and-lower half of FR-1040-AC-3 is now tested (`only` and `count` rows, binder `c` as owner, place `clock_place`), and the state-invariant half was already tested by `a_population_ref_lowers_with_the_binder_as_owner`. The evaluation clause of AC-3 and its `self.r` form remain untested anywhere. The negative test matches the amended "Refusals that remain" sentence (`RefinementCause::ClockRead` maps to `unsupported_construct`/`expression-form`, refinement.rs:365,389, refused at the read span, refinement.rs:3444). FR-230-AC-11 is not what these tests cover (code-review SR FND-001).

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-1040-AC-3 | examined | In a refinement row over `calls = {c1, c2}`, `only(c in M::calls where c.n = 2: ref(c.x))` and `count(c in M::calls where ref(c.x) = self.r)` check, lower with the binder `c` itself (typed `Reference<Call>`) as owner term, and evaluate as FSpec FR-829-AC-2 states; the same `count` in a state invariant (FR-391) checks and lowers. |
| FR-1040 "Refusals that remain", first bullet | examined | outside a refinement row clock-read-outside-constraint; inside, unsupported_construct/expression-form at the read |
| FR-230-AC-11 | examined | record Timer held by field t over three objects |
| TC-3084 step 2 | context_only | Compile and evaluate the population forms of FR-1040-AC-3 and FR-1040-AC-4. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-1040-AC-3's "evaluate as FSpec FR-829-AC-2 states" has no test: no test in fsl-eval or elsewhere evaluates the refinement-row or invariant `only`/`count` over `ref(c.x)`, and TC-3084 step 2 requires it. The AC's `count(... where ref(c.x) = self.r)` comparison against a `Reference<Clock>` field is also not exercised (the fixture has no `r`; the test compares `ref(self.x)`), and no universe `{c1, c2}` is admitted. With three tests now tagged FR-1040-AC-3 the matrix shows it covered. Add an evaluation test, or record the evaluation clause as deferred with a ticket. | fsl-semantics/tests/it/clock_places.rs:3826-3853, spec/test-cases/TC-3084-clock-place-references-and-presence-in-population-forms.md:31-32 |
