---
id: SR-2026112105500002
title: "Gap analysis — PR #3090 QSL-2003 against FR-766"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3090 at 901c5e27ba against FR-766-AC-1..AC-9, TC-2195, TC-2240 and the QSL-2003 language criteria"
review_set: subset
evaluated_revision: "901c5e27ba"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-766
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-2240
    type: reviews
---

## Summary

Ticket: QSL-2003. Plan completion: not assessed. This is a manual mapping of each FR-766 criterion to a tagged test and to the code that serves it, read from the diff and the head tree with `git show`. `quoin matrix` was not run, because the brief allows no builds and the matrix is not needed to map a nine-row FR.

| AC | Test (trace) | Code |
| --- | --- | --- |
| FR-766-AC-1 | collection_queries.rs `q14_count_refuses_unproved_range_at_check`, `q14_count_proves_when_the_bound_fits` (TC-2195) | facts.rs Count arm |
| FR-766-AC-2 | `q14_count_under_a_size_guard_proves` | facts.rs `Subject::Size` lookup |
| FR-766-AC-3 | `q14_count_refuses_unproved_range_at_check` (`Int[1,3]`, unbounded) | facts.rs lower bound `0`, open upper |
| FR-766-AC-4 | `coerce_and_count_refusals_carry_the_declared_domain`, new `q14_count_kernel_refusal_records_its_domain_at_the_count` | S6a (unchanged) |
| FR-766-AC-5 | clause_paths.rs `count_out_of_domain_names_its_cause` (enabled) | facts.rs via spine Linked |
| FR-766-AC-6 | `a_count_refused_at_compile_is_a_declined_run_item_with_its_cause` (enabled) | existing compile-cause path |
| FR-766-AC-7 | `a_kernel_refusal_record_names_its_cause_on_the_run_item` (enabled) | outcome.rs new `CallOutcome::Refused` arm |
| FR-766-AC-8 | check_mode.rs `a_count_range_in_a_state_clause_refuses_under_both_modes` (TC-2240) | facts.rs; state clauses keep Linked definedness |
| FR-766-AC-9 | check_mode.rs `an_unannotated_literal_count_source_is_an_ambiguous_literal` (TC-2240) | existing literal typing |

The ticket's language criteria are met by the same tests. The ticket's AC naming `count_out_of_domain_names_its_cause` maps to FR-766-AC-5, and the run-cause half maps to AC-7. The docs and H3 criteria are out of scope, as the QSL:plan comment of 2026-11-15 says. Every test is real: none is a stub or tautology, and each asserts typed causes or records. No production code in the diff lacks an owning requirement.

## Verdict

PASS for coverage: every FR-766 criterion has a tagged, non-ignored test and serving code. The suite breakages that the new obligation causes outside the diff are recorded in the code review (SR-2026112105500001 FND-001 to FND-003), not here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Findings data

```yaml
clean: true
scope:
  - {id: FR-766-AC-1, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "Under `CheckMode::Linked`, for `q` of `Sequence<Integer>[0, 5]`, `Bag<Integer>[0, 5]` and `Set<Integer>[0, 5]`, `count<Int[0, 3]>(x in q: x > 0)` refuses `undefined_expression`/`unproved-range` at the `count` with required `[0, 3]` and proved `[0, 5]`; the same count into `Int[0, 5]` and into `Integer` checks."}
  - {id: FR-766-AC-2, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "`if size(q) <= 3 then count<Int[0, 3]>(x in q: x > 0) else 0` checks for the AC-1 sources, and `if size(q) <= 4 ...` refuses `unproved-range` with proved `[0, 4]`."}
  - {id: FR-766-AC-3, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "`count<Int[1, 3]>(x in q: true)` for `q` of `Sequence<Integer>[0, 3]` refuses `unproved-range` with required `[1, 3]` and proved `[0, 3]`; ... an unbounded `q` refuses ... with a proved interval that has no upper bound."}
  - {id: FR-766-AC-4, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "Under `CheckMode::Kernel`, `count<Int[0, 3]>(x in q: x)` for `q` of `Sequence<Boolean>[0, 5]` checks, and evaluating it over five `true` occurrences refuses `IntegerOutOfDomain` with target `Int[0, 3]`"}
  - {id: FR-766-AC-5, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "invariant AtMostOne ... refuses at compile `undefined_expression`/`unproved-range` at the `count`, exit 20; with `type One = Int[0, 2];` it compiles."}
  - {id: FR-766-AC-6, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "The AC-5 `Int[0, 1]` unit run as a clause run of `AtMostOne` returns a compile disposition, and the run item ... carries ... cause `{code: undefined_expression, kind: unproved-range}`."}
  - {id: FR-766-AC-7, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "A clause-run report whose disposition is an S6a kernel refusal record ... projects to a run item with ... cause `{code: integer_out_of_domain, kind: outside-domain}`, never a `null` cause."}
  - {id: FR-766-AC-8, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "invariant I ... { let q: Sequence<Integer>[0, 1] = sequence[self.versionNumber] in count<Zero>(m in q: m >= 0) = 0 } ... refuses `undefined_expression`/`unproved-range` at the `count` in both modes"}
  - {id: FR-766-AC-9, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "the AC-8 invariant written with the literal as `count`'s source ... refuses `ill_typed`/`ambiguous-literal` in both modes"}
findings: []
```
