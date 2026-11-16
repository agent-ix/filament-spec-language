---
id: SR-2026111605245700000002
title: "code-review of QSL-2002 PR 2738 exact modulo intervals"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 2738; QSL-2002; fsl-semantics/src/check/check.rs; fsl-semantics/src/check/check/typing.rs; fsl-semantics/src/check/check/modulo_interval.rs; fsl-semantics/tests/it/division_law_selection.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: references
---

## Summary

Ticket: QSL-2002. Reviewed frozen head `d7f2021b2e33109f27b3c47d7120b0c5f6562470` against main `70093ace3a78d554d45d756559582ba523cc0141` in the one assigned worktree. This is a new PR review; the closed PR #2731's earlier QSL-2002/QSL-2004 review artifacts remain unchanged. Scope is exact mod intervals and checking-work limits; QSL-1962 zero-divisor behavior and QSL-1989 div/rem work are excluded.

## Verdict

**FAIL** — known operand ranges are lost before interval computation, rejecting valid programs. PR #2738 is not cleared for merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Passing only operand ValueTypes discards known ranges of literals, arithmetic and negation: a:Int[-7,-7] mod 3 is typed Integer instead of Int[2,2], and (a+1) mod b for a:Int[0,1], b:Int[5,6] is typed Int[0,5] instead of Int[1,2], refusing valid bounded consumers. | fsl-semantics/src/check/check/typing.rs:3025-3027 |

## Analysis

The failure is established by tracing the real checker, without a source mutation or a new test. Integer literals are emitted with `ValueType::Integer` at typing.rs:1197-1207; integer arithmetic and negation also retain `Integer` at check.rs:4023-4037 and 4175-4177. The new call forwards only those types. `modulo_interval::bounded` then returns None, and the negative-dividend/unbounded-divisor arm returns Integer. `Definedness::IntervalWalk` has no IntegerDivision arm and falls back to the node's declared type at facts.rs:1719-1722, so the Coerce obligation at facts.rs:1235-1248 refuses an otherwise exact consumer.

Concrete source under the complete-value header: `function f using v(a: Int[-7,-7]): Int[2,2] pure { a mod 3 }`. The divisor is the nonzero constant 3; the only remainder is 2, yet the new typing path remains unbounded. A second source, `function g using v(a: Int[0,1], b: Int[5,6]): Int[1,2] pure { (a + 1) mod b }`, is assigned [0,5] although its result is exactly [1,2]. Both are entirely within mod-range scope and require no QSL-1962 change.

The repository already provides `Typer::static_range` at check.rs:2820 and `ir::static_integer_range` at ir.rs:940 for literals, arithmetic, negation, conditional hulls and declared bounded nodes. Preserve those operand ranges when calling the new interval engine; do not infer unboundedness solely from ValueType.

Reviewed every arm of the finite quotient-run algorithm, including floor-quotient negative endpoints, divisor magnitude reduction, both closed forms, the descending maximal-run boundary, early stop, unbounded operands and refusal propagation. No second arithmetic defect was found in those branches. Searched existing static range and remainder helpers across fsl-semantics/src; no duplicate run-walk implementation is introduced. No Cargo, wire, unsafe, workflow or public API change exists. No applicable AssuranceProfile is declared; textual mentions in old review reports are not profiles. Both old rem assertions are preserved in the separately ignored QSL-1989 test.

## Coverage

Reconciliation: quoin matrix (quoin 0.29.4), cross-checked against quire matrix (quire 0.38.3, engine 0.52.1). Whole-repository static counts: tagged 3715; untagged 1045; tagged-by-ignored-test 302; method-without-symbol 98; retired 13. These baseline counts are not a repository-wide PASS; findings are scoped to the dispatched PR. The changed criteria FR-495-AC-17/19/20 are tagged, with excluded rem/nonzero subcases still ignored. No run evidence is bound in the matrix for these criteria.

Plan completion: not assessed

Methods: code-review with Rust lane and gap-analysis. Spec-review does not apply because no spec/plan file changed. Optional gap semantic audit was not separately invoked; requirement/code/test faithfulness was checked by the required code-review method. No source edits, tests, builds or aggregate gate were run by this reviewer. The PR reports post-rebase fmt, workspace check, default/no-default Clippy, fsl-semantics tests (900 unit, 587 integration, 26 doctests) and qualified-core passing; these are coder evidence, not an independent rerun. Earlier /private/tmp logs report 581 integration tests and were not represented as this head's evidence.

## Examined scope and bindings

Publication: Linear comment `5a7cbba7-99c9-4afa-829b-45d1253a3ed5`. Exact body read-back, finding-id/field checks and real YAML parsing via `quire parse` all passed. The existing QSL `has-review` label was applied idempotently. Quire validation passed with the installed inline-schema and duplicate-archetype warnings.

```yaml
{
  "clean": false,
  "head": "d7f2021b2e33109f27b3c47d7120b0c5f6562470",
  "scope": [
    {
      "id": "FR-495",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "The checker SHALL give `a div b`, `a rem b`\n  and `a mod b` the smallest interval that contains the result of the operator\n  for every pair `(x, y)` with `x` in the range of `a`, `y` in the range of `b`\n  and `y` not zero, taking the retained division law for `div` and `rem`\n  (FSpec FR-147) and the Euclidean remainder for `mod`. When both ranges are\n  singletons the result range is the singleton of the exact result."
    },
    {
      "id": "FR-495-AC-17",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "`function m1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure { a mod b }` checks and the checked `mod` node has type `Int[0, 11]`"
    },
    {
      "id": "FR-495-AC-19",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "With `b: Integer`, `if b != 0 then a mod b else 0` over `a: Int[0, 9]` checks into `Int[0, 9]`, and over `a: Int[-1, 9]` refuses `unproved-range` into `Int[0, 1000]` (the remainder of a negative dividend by an unbounded divisor is unbounded)."
    },
    {
      "id": "FR-495-AC-20",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "With the work ceiling set to 10000 (`with_work_budget(10000)`), `big` refuses `stage_limit_exceeded` with the `WorkBudget` kind and bound 10000 at the `mod`, as its only refusal, never `unproved-range`, and no checked graph is returned."
    },
    {
      "id": "FR-495/remainder-run-walk",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "The walk visits the runs from the largest\n    magnitude down and stops as soon as the minimum found is 0 and the maximum\n    found is `U - 1`, the bounds no run can pass; otherwise it visits every\n    run. The interval is the minimum and maximum over the visited runs."
    },
    {
      "id": "FSpec-FR-147-AC-12",
      "path": "../filament-specification/spec/functional/expressions/FR-147-evaluate-integer-division-domains.md",
      "role": "examined",
      "excerpt": "For `a: Int[0, 100]` and `b: Int[1, 12]`, `a mod b` has result type `Int[0, 11]` and is admitted into an `Int[0, 11]` consumer, and is refused `unproved-range` into `Int[0, 10]`; with `a` of the unbounded integer type `a mod b` still has type `Int[0, 11]`."
    },
    {
      "id": "FSpec-FR-147-AC-14",
      "path": "../filament-specification/spec/functional/expressions/FR-147-evaluate-integer-division-domains.md",
      "role": "examined",
      "excerpt": "`a mod b` with `a: Int[0, 2]` and `b: Int[5, 6]` has type `Int[0, 2]`; with `a: Int[91, 91]` and `b: Int[2, 12]` it has type `Int[0, 7]`; with `a: Int[97, 97]` and `b: Int[2, 12]` it has type `Int[1, 9]`; with `a: Int[-7, -7]` and `b: Int[3, 5]` it has type `Int[1, 3]`."
    },
    {
      "id": "TC-2210",
      "path": "spec/test-cases/TC-2210-a-remainder-node-carries-its-exact-smallest-interval.md",
      "role": "context_only",
      "excerpt": "Verify from source that `mod` and `rem` are typed by the exact smallest\ninterval of their remainders, not by the divisor bound `[0, m - 1]`,"
    },
    {
      "id": "TC-2211",
      "path": "spec/test-cases/TC-2211-the-remainder-walk-is-bounded-by-the-checking-work-ceiling.md",
      "role": "context_only",
      "excerpt": "Verify that the exact remainder interval of a large prime dividend over a\nwide divisor range is found under the default checking work ceiling, and\nthat a ceiling below the walk's cost stops checking with the work stage\nlimit instead of widening the type."
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "code-bug",
      "artifact_id": "FR-495",
      "related_ids": [
        "FR-495-AC-17",
        "FSpec-FR-147-AC-12"
      ],
      "path": "fsl-semantics/src/check/check/typing.rs",
      "lines": "3025-3027",
      "excerpt": "The checker SHALL give `a div b`, `a rem b`\n  and `a mod b` the smallest interval that contains the result of the operator\n  for every pair `(x, y)` with `x` in the range of `a`, `y` in the range of `b`\n  and `y` not zero, taking the retained division law for `div` and `rem`\n  (FSpec FR-147) and the Euclidean remainder for `mod`. When both ranges are\n  singletons the result range is the singleton of the exact result.",
      "finding": "Passing only operand ValueTypes discards known ranges of literals, arithmetic and negation: a:Int[-7,-7] mod 3 is typed Integer instead of Int[2,2], and (a+1) mod b for a:Int[0,1], b:Int[5,6] is typed Int[0,5] instead of Int[1,2], refusing valid bounded consumers."
    }
  ],
  "bindings": [
    {
      "test_id": "a_mod_node_is_typed_by_its_divisor_interval",
      "ac_id": "FR-495-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "a_remainder_node_carries_its_exact_smallest_interval",
      "ac_id": "FR-495-AC-19",
      "trace": "correct"
    },
    {
      "test_id": "a_rem_node_carries_its_exact_smallest_interval_under_each_law",
      "ac_id": "FR-495-AC-19",
      "trace": "correct"
    },
    {
      "test_id": "small_modulo_intervals_match_all_nonzero_operand_pairs",
      "ac_id": "FR-495-AC-19",
      "trace": "correct"
    },
    {
      "test_id": "the_remainder_walk_is_bounded_by_the_checking_work_ceiling",
      "ac_id": "FR-495-AC-20",
      "trace": "correct"
    }
  ]
}
```

