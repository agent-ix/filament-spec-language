---
id: SR-2026111605245700000003
title: "gap-analysis of QSL-2002 PR 2738 exact modulo intervals"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 2738; QSL-2002; fsl-semantics/src/check/check.rs; fsl-semantics/src/check/check/typing.rs; fsl-semantics/src/check/check/modulo_interval.rs; fsl-semantics/tests/it/division_law_selection.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: references
---

## Summary

Ticket: QSL-2002. Reviewed frozen head `d7f2021b2e33109f27b3c47d7120b0c5f6562470` against main `70093ace3a78d554d45d756559582ba523cc0141` in the one assigned worktree. This is a new PR review; the closed PR #2731's earlier QSL-2002/QSL-2004 review artifacts remain unchanged. Scope is exact mod intervals and checking-work limits; QSL-1962 zero-divisor behavior and QSL-1989 div/rem work are excluded.

## Verdict

**CONDITIONAL** — missing expression-shape and checked-node-type assertions leave required behavior uncovered. PR #2738 is not cleared for merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The active exact-interval tests use parameter operands only; none covers a literal operand or an arithmetic/negation operand with a known finite range, so the ValueType-only range-loss defect is not exercised. | fsl-semantics/tests/it/division_law_selection.rs:277-341 |
| FND-002 | medium | a_mod_node_is_typed_by_its_divisor_interval checks admission and refusal but never inspects the checked mod node's type, leaving AC-17's explicit Int[0,11] node-type contract unasserted. | fsl-semantics/tests/it/division_law_selection.rs:189-212 |

## Analysis

The computed matrix tags FR-495-AC-17/19/20, but those are broader than the dispatched mod-only scope. The excluded zero-divisor and rem subcases remain explicitly ignored and assigned to QSL-1962/QSL-1989. The rem split preserves both original assertions and trace tags. No in-scope behavior lacks an owning requirement, and no new production stub was found.

All active exact-interval examples pass parameter nodes to mod. The small interval enumeration varies operand domains and values but never changes the operand expression shape, and skips singleton result intervals. It therefore misses the literal/arithmetic range-loss failure recorded in the code review. Add real source cases for literal operands, derived ranges and exact singleton results, with positive exact consumers and rejecting narrowed consumers where those intervals remain valid.

The active AC-17 test discards the CheckedGraph after successful checking. Admission/refusal proves bounded-consumer behavior but cannot establish that the mod node itself carries the smallest interval independently of a consumer. Inspect the mod node under an Integer-returning consumer as well as the explicitly bounded case and assert both endpoints. Existing generic source tests outside this file do not supply an active AC-17 node-type assertion according to the matrix.

## Coverage

Reconciliation: quoin matrix (quoin 0.29.4), cross-checked against quire matrix (quire 0.38.3, engine 0.52.1). Whole-repository static counts: tagged 3715; untagged 1045; tagged-by-ignored-test 302; method-without-symbol 98; retired 13. These baseline counts are not a repository-wide PASS; findings are scoped to the dispatched PR. The changed criteria FR-495-AC-17/19/20 are tagged, with excluded rem/nonzero subcases still ignored. No run evidence is bound in the matrix for these criteria.

Plan completion: not assessed

Methods: code-review with Rust lane and gap-analysis. Spec-review does not apply because no spec/plan file changed. Optional gap semantic audit was not separately invoked; requirement/code/test faithfulness was checked by the required code-review method. No source edits, tests, builds or aggregate gate were run by this reviewer. The PR reports post-rebase fmt, workspace check, default/no-default Clippy, fsl-semantics tests (900 unit, 587 integration, 26 doctests) and qualified-core passing; these are coder evidence, not an independent rerun. Earlier /private/tmp logs report 581 integration tests and were not represented as this head's evidence.

## Examined scope and bindings

Publication: Linear comment `7d6ce8b1-fafa-470c-8fc9-5a5119b5ddb7`. Exact body read-back, finding-id/field checks and real YAML parsing via `quire parse` all passed. The existing QSL `has-review` label was applied idempotently. Quire validation passed with the installed inline-schema and duplicate-archetype warnings. Coverage also lists FSpec-prefixed external references and ignored-test ticket references as unmatched local tags; these were inspected against the actual source and were not treated as new local trace defects.

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
      "severity": "medium",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-495",
      "related_ids": [
        "FR-495-AC-19",
        "FSpec-FR-147-AC-14"
      ],
      "path": "fsl-semantics/tests/it/division_law_selection.rs",
      "lines": "277-341",
      "excerpt": "The checker SHALL give `a div b`, `a rem b`\n  and `a mod b` the smallest interval that contains the result of the operator\n  for every pair `(x, y)` with `x` in the range of `a`, `y` in the range of `b`\n  and `y` not zero, taking the retained division law for `div` and `rem`\n  (FSpec FR-147) and the Euclidean remainder for `mod`. When both ranges are\n  singletons the result range is the singleton of the exact result.",
      "finding": "The active exact-interval tests use parameter operands only; none covers a literal operand or an arithmetic/negation operand with a known finite range, so the ValueType-only range-loss defect is not exercised."
    },
    {
      "fnd": "FND-002",
      "severity": "medium",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-495-AC-17",
      "related_ids": [
        "FSpec-FR-147-AC-12"
      ],
      "path": "fsl-semantics/tests/it/division_law_selection.rs",
      "lines": "189-212",
      "excerpt": "`function m1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure { a mod b }` checks and the checked `mod` node has type `Int[0, 11]`",
      "finding": "a_mod_node_is_typed_by_its_divisor_interval checks admission and refusal but never inspects the checked mod node's type, leaving AC-17's explicit Int[0,11] node-type contract unasserted."
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


## Dispositions

Round 1 at `cf48c310c7f4f6ff80a09b56f3ee47a8171414a2`, reviewed through read-only inspection of the complete fix commit and its callers/tests. Original findings and their review-pass verdict remain unchanged above.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | modulo_uses_literal_and_derived_operand_ranges checks the -7 mod 3 singleton, rejects a consumer excluding 2, and checks arithmetic/negated operands against both narrowed endpoints. |
| FND-002 | fixed | The AC-17 test inspects the checked root type for m1/m2/m4. The literal test also asserts Int[2,2] under an Int[0,2] consumer, so the observed type is independent of the consumer's wider bounds; the existing coerce path retains a contained mod node unchanged. |

## Disposition round 1 verdict

Round 1 publication: Linear comment `1e75e72c-5cce-4e94-b478-4cfa467e84bc`; exact body read-back, original-finding coverage, nonempty fixed excerpts and real YAML parsing through quire all passed. Original comments and findings remain unchanged. Existing Quire inline-schema/duplicate-archetype warnings remain non-fatal.

**PASS** — every original gap-analysis finding is fixed; no new finding. PR #2738 is review-cleared and mergeable after the lead's required final aggregate gate. GitHub currently reports OPEN/MERGEABLE against main. No source edits, builds, tests or gate were run by this reviewer, as the disposition brief requires.

The fix preserves the excluded QSL-1962 nonzero behavior and the two QSL-1989 rem assertions. It reuses the existing iterative static_integer_range helper, and the quotient-run algorithm and its charge/refusal path are unchanged. The regression's singleton type under a wider consumer confirms that its type assertion is not merely the consumer's declared type. No spec/plan changes require a spec-review pass.

```yaml
{
  "head": "cf48c310c7f4f6ff80a09b56f3ee47a8171414a2",
  "round": 1,
  "verdict": "PASS",
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "after_excerpt": "    exact(VALUE, \"a: Int[0, 1], b: Int[5, 6]\", \"(a + 1) mod b\", 1, 2);\n    exact(VALUE, \"a: Int[1, 2], b: Int[5, 6]\", \"(-a) mod b\", 3, 5);",
      "reason": "modulo_uses_literal_and_derived_operand_ranges checks the -7 mod 3 singleton, rejects a consumer excluding 2, and checks arithmetic/negated operands against both narrowed endpoints.",
      "artifact_id": "FR-495",
      "original_finding": "The active exact-interval tests use parameter operands only; none covers a literal operand or an arithmetic/negation operand with a known finite range, so the ValueType-only range-loss defect is not exercised."
    },
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "after_excerpt": "        let body_type = graph\n            .function_state(0)\n            .expect(\"the function is checked\")\n            .body\n            .value_type();\n        assert_eq!(\n            body_type,\n            &ValueType::Int(\n                IntegerInterval::new(lower.into(), upper.into()).expect(\"valid interval\")\n            ),\n            \"checked mod node type for {source}\"\n        );",
      "reason": "The AC-17 test inspects the checked root type for m1/m2/m4. The literal test also asserts Int[2,2] under an Int[0,2] consumer, so the observed type is independent of the consumer's wider bounds; the existing coerce path retains a contained mod node unchanged.",
      "artifact_id": "FR-495-AC-17",
      "original_finding": "a_mod_node_is_typed_by_its_divisor_interval checks admission and refusal but never inspects the checked mod node's type, leaving AC-17's explicit Int[0,11] node-type contract unasserted."
    }
  ],
  "new_findings": [],
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
  ]
}
```

