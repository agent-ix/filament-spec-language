---
id: SR-13628
title: "QSL-2323 gap-analysis of integer division fixtures"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2323; fsl-analyze/tests/it/value_validity.rs; fsl-eval/tests/it/source_call.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-495", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-091", type: references }
---

## Summary

Ticket: QSL-2323. PR: filament-spec-language#2787. Diff-scoped gap analysis found the existing Linked refusal and bounded-nonzero acceptance coverage intact. The code-review artifact owns the law-identity fixture finding.

## Verdict

PASS for the requested change scope — no additional gap introduced by this fixture-only change. This is not full-repository acceptance or an aggregate gate result.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Review scope is the two-file diff origin/main...HEAD at 2d0eff34b42e19dec2f8600ef3788a327f01911c, with contextual inspection of division_law_run.rs, division_law_selection.rs, check_mode.rs and the checker discharge logic. No production code, spec, CI or threshold changes. No applicable AssuranceProfile found. No duplicated production semantics or new external dependency.

The runtime fixture helper retains Linked as its default in fsl-analyze. Kernel permits the unbounded zero-divisor counterexample and its guarded replay control without narrowing mathematical Integer inputs. fsl-eval's shared integer fixture exercises signed laws and zero-divisor/minimum-integer runtime outcomes. The mod-without-law positive case explicitly retains Linked using b: Int[1, 9]. Existing AC-17/18 tests keep both bounded-nonzero admission and possibly-zero refusal. AC-3 is broad: the reviewed checker/runtime distinction is supported; this review does not claim every decimal/quantity vector was executed. The AC-5 binding covers its documented FSL replay half, not provider settlement.

Reconciliation: quoin matrix 0.29.4 (quire 0.38.3, engine 0.52.1), whole repository queried; all five selected criteria are tagged with no run evidence in the store. Whole-repository background counts: tagged 3825; tagged-by-ignored-test 381; untagged 1144; method-without-symbol 102; retired 13. Unchanged repository-wide gaps are outside this requested diff review, so this is not a whole-repository assurance PASS.

No Cargo commands ran in this review, per dispatch. PR-reported targeted evidence: analyzer division 1 passed/1 ignored, evaluator division 13 passed, evaluator nonzero 6 passed; analyzer nonzero matches no tests. These are reported results, not independently rerun evidence. git diff --check passed. The reported 121 analyzer failures were not reproduced by those filters; no wider-suite conclusion or speculative defect is recorded. Plan completion: not assessed.

Semantic review ran for the explicitly requested mode/criterion agreement; no plan supplied. Reverse gaps and stubs: no production behavior added, and changed tests retain concrete behavioral assertions. No wider optional semantic review was performed.

## Examined units

Excerpts below identify each examined criterion; long rows are excerpted to the first 590 characters. Conclusions are limited to the affected fixture behavior, not every clause of each broad criterion.

```yaml
{
  "scope": [
    {
      "id": "FR-495-AC-3",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "`function d1 using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 9] pure { a div b }`, the same with `rem` and `mod`, `function d2 using v(a: P, b: P): P pure { a / b }` with `type P = Decimal[-1000, 1000; 0, 2; nearest-even];`, `function d3 using v(): Boolean pure { 1 cm / 0 cm = 1 cm / 0 cm }` (with TC-187's units) and `function d4 using v(): P pure { decimal(1, 0) / decimal(0, 3) }` each refuse `undefined_expression`/`unproved-nonzero` at the operator under `CheckMode::Linked`; `function d5 using v(a: Int[0, 9], b: Int[1, 9]): Int[0, 9] pure { a div b }` and `if b != 0 then a div b else 0"
    },
    {
      "id": "FR-495-AC-5",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "With a cvc5 provider registered, FSpec TC-192 DIV-14's holding obligation settles `proved` and its violated obligation settles `refuted` through the FR-349 route, and the `refuted` counterexample replays through the FSL complete-V1 executor to the violation (FR-098); a lock with no provider reports DIV-14 pending on the provider and never `proved` or `refuted`; Z3 absent leaves cvc5's verdict settled with the absent-cross-check warning."
    },
    {
      "id": "FR-495-AC-17",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "Under `CheckMode::Linked`, from source with no law header: `function m0 using v(a: Int[0, 100], b: Int[0, 12]): Int[0, 11] pure { a mod b }` refuses `undefined_expression`/`unproved-nonzero` at the `mod`, as its only refusal; `function m1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure { a mod b }` checks and the checked `mod` node has type `Int[0, 11]`; `function m2 using v(a: Integer, b: Int[1, 12]): Int[0, 11] pure { a mod b }` checks (the remainder is bounded by the divisor alone); `function m3 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 10] pure { a mod b }` refuses `"
    },
    {
      "id": "FR-495-AC-18",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "Under `CheckMode::Linked` and a law header for each of the three laws (FR-670): `function r1 using v(a: Int[-100, 100], b: Int[1, 12]): Int[-11, 11] pure { a rem b }` checks, its `rem` node typed `Int[0, 11]` under Euclidean and floor and `Int[-11, 11]` under truncating; the same with result `Int[0, 11]` checks under Euclidean and floor and refuses `unproved-range` at the `rem` under truncating; `function q1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 100] pure { a div b }` checks with its `div` node typed `Int[0, 100]`; and `q1` and `r1` with `b: Int[0, 12]` each refuse `undefine"
    },
    {
      "id": "FR-091-AC-39",
      "path": "spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md",
      "role": "examined",
      "excerpt": "S2 maps `a div b`, `a rem b` and `a mod b` to distinct `IntegerDiv`, `IntegerRem` and `IntegerMod` binary forms with operand order and original spans, including in an unreachable branch. Check admits each well-typed operator under a resolved `filament.value.complete/v1` selection, using the checked package's exactly-one law for `div`/`rem` and Euclidean meaning for `mod`; without that facet, each receives a located `unsupported_construct` rather than `UnrepresentedConstruct`. `/` never becomes integer division; only under complete value does a rational `/` with integer operands and a"
    }
  ],
  "bindings": [
    {
      "test_id": "lowered_division_nodes_keep_the_selected_law_in_their_identity",
      "ac_id": "FR-091-AC-39",
      "trace": "correct"
    },
    {
      "test_id": "complete_value_requires_one_admitted_division_law_but_mod_needs_none",
      "ac_id": "FR-091-AC-39",
      "trace": "correct"
    },
    {
      "test_id": "integer_division_zero_and_signed_minimum_remain_exact",
      "ac_id": "FR-091-AC-39",
      "trace": "correct"
    },
    {
      "test_id": "integer_division_operators_follow_each_selected_signed_law",
      "ac_id": "FR-091-AC-39",
      "trace": "correct"
    },
    {
      "test_id": "a_mathematical_integer_division_counterexample_replays_to_the_violation",
      "ac_id": "FR-495-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "an_unproved_nonzero_divisor_checks_only_under_kernel",
      "ac_id": "FR-495-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "a_mod_node_is_typed_by_its_divisor_interval",
      "ac_id": "FR-495-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "a_mod_divisor_that_may_be_zero_refuses_unproved_nonzero",
      "ac_id": "FR-495-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "a_divisor_the_checker_cannot_exclude_is_undefined_only_under_kernel",
      "ac_id": "FR-495-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "div_and_rem_carry_their_interval_and_nonzero_obligation_under_each_law",
      "ac_id": "FR-495-AC-18",
      "trace": "correct"
    }
  ]
}
```

