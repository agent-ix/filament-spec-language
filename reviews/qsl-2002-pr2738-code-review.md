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


## Dispositions

Round 1 at `cf48c310c7f4f6ff80a09b56f3ee47a8171414a2`, reviewed through read-only inspection of the complete fix commit and its callers/tests. Original findings and their review-pass verdict remain unchanged above.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The real typer now derives both operand ranges through the existing static_range walk; modulo_interval::bounded prioritizes those ranges over ValueType, preserving literals, arithmetic and negation. |


Round 2 at `1a094b06d23a89556ec9b20fb31971a7326071b7`:

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | The gate-discovered catalog mismatch is fixed: mod applications use catalog Integer, while direct bounded consumers receive explicit Coerce/narrow wrappers. This disposition covers that original mismatch, not the separately reported claim-record regression. |
| FND-003 | still-open | claims.rs:840-853 still uses the ValueType-only coerce_builds_narrow predicate, while lowering.rs:4448-4460 overrides it for direct mod. For m1(a:Int[0,100],b:Int[1,12]):Int[0,11]{a mod b}, the emitted direct narrow exists but the claim is SiteBound::Own(Int[0,11]); key_claims selects the mod application's Integer type node and no conversion_context. Wider bounded consumers also lose their actual target bound. fsl-route emits that wrong node/context, while replay reads the bounded ValueType, so proof and replay disagree. |

Round 3 at `b1f9c834839852ba46ce3feb9cfa96d57c7d84af`:

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | still-open | The shared predicate fixes direct equal/wider bounded consumers, but an inferred mod still produces an inconsistent own-bound record. For function f using v(a:Int[0,100],b:Int[1,12]):Integer pure { (a mod b) + 1 }, arithmetic infers its operands (typing.rs:2118-2131,3064-3090), retains the mod Int[0,11], and adds no Coerce around it. claims.rs:996-999 records Own(Int[0,11]); lowering.rs:4262-4277 emits Integer; key_claims chooses that Integer type node at claims.rs:1103-1125. The record and routed request still disagree with replay's bounded ValueType. |
| FND-004 | still-open | The new coerce arm overwrites a finite mod node's Int[0,11] type with Integer whenever its direct consumer expects Integer. The already-added integer(a:Int[0,100],b:Int[1,12]):Integer { a mod b } fixture therefore violates FR-495's explicit checked-node rule. Keep the exact checked interval and model any consumer/wire widening separately; do not erase the interval to align claim metadata. |

## Disposition round 1 verdict

Round 1 publication: Linear comment `ca818c3d-6686-49d5-8aa2-3a135af4bc76`; exact body read-back, original-finding coverage, nonempty fixed excerpts and real YAML parsing through quire all passed. Original comments and findings remain unchanged. Existing Quire inline-schema/duplicate-archetype warnings remain non-fatal.

**PASS** — every original code-review finding is fixed; no new finding. PR #2738 is review-cleared and mergeable after the lead's required final aggregate gate. GitHub currently reports OPEN/MERGEABLE against main. No source edits, builds, tests or gate were run by this reviewer, as the disposition brief requires.

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
      "after_excerpt": "                            let left_range = self.static_range(&left);\n                            let right_range = self.static_range(&right);\n                            self.modulo_result_type(\n                                &left.value_type,\n                                &right.value_type,\n                                left_range.as_ref(),\n                                right_range.as_ref(),\n                                &location,\n                            )?",
      "reason": "The real typer now derives both operand ranges through the existing static_range walk; modulo_interval::bounded prioritizes those ranges over ValueType, preserving literals, arithmetic and negation.",
      "artifact_id": "FR-495",
      "original_finding": "Passing only operand ValueTypes discards known ranges of literals, arithmetic and negation: a:Int[-7,-7] mod 3 is typed Integer instead of Int[2,2], and (a+1) mod b for a:Int[0,1], b:Int[5,6] is typed Int[0,5] instead of Int[1,2], refusing valid bounded consumers."
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


## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | Historical gate regression: exact checked mod types were emitted directly as the operation result type, violating the catalog Integer signature and causing the reported fsl-replay reader failures. Fixed by this head. | fsl-semantics/src/check/lowering.rs:4257-4277 |
| FND-003 | high | claims.rs:840-853 still uses the ValueType-only coerce_builds_narrow predicate, while lowering.rs:4448-4460 overrides it for direct mod. For m1(a:Int[0,100],b:Int[1,12]):Int[0,11]{a mod b}, the emitted direct narrow exists but the claim is SiteBound::Own(Int[0,11]); key_claims selects the mod application's Integer type node and no conversion_context. Wider bounded consumers also lose their actual target bound. fsl-route emits that wrong node/context, while replay reads the bounded ValueType, so proof and replay disagree. | fsl-semantics/src/check/lowering.rs:4448-4460 |

## Round 2 publication

Linear comment `86426eac-5960-4230-b8e9-229e28014f11`. Exact body read-back, finding/disposition field checks and real YAML parsing via quire parse all passed. Quire validation passed with nonfatal inline-data-schema and duplicate-archetype warnings. Original review and round 1 history remain unchanged.

## Disposition round 2 verdict

**FAIL — PR #2738 is not mergeable.** Historical gate mismatch is fixed; FND-003 remains open. All original round-1 outcomes remain fixed.

Read-only static review of the complete gate-fix delta and affected checker/lowering/claims/request/replay consumers. No source edits, builds, tests or aggregate gate. No new spec behavior is invented. The dispatch-upcast branch was checked against state_model::Form::Argument: it admits exact scalar types only, so it does not widen a bounded mod to Integer. Original rem assertions remain in the separate ignored QSL-1989 test. No zero-divisor or div/rem behavior was changed. Plan completion: not assessed. Prior matrix results remain historical; this pass inspected the added trace and assertions rather than rerunning execution evidence.

Concrete claim trace: coerce wraps a direct mod even for an equal/containing target; lowering unconditionally emits numeric.narrow for that wrapper. claims::leave still calls ir::coerce_builds_narrow(Int[0,11], Int[0,11]), which returns false. Classify::claim records Own(Int[0,11]); key_claims consequently uses the mod Integer node and None conversion context (claims.rs:1106-1128,1155-1159). fsl-route/src/request.rs:1084-1085 publishes those wire fields, while fsl-replay/src/value_check.rs:436-439 uses the Int ValueType. For target Int[0,20], the record also retains [0,11] instead of the emitted direct narrow's [0,20]. Fix the shared checked-to-lowered representation agreement and test both fields/context; do not erase the exact checked mod interval to hide the mismatch.

```yaml
{
  "head": "1a094b06d23a89556ec9b20fb31971a7326071b7",
  "base": "45705f21678355b93ba7492271c93ef00fe00b39",
  "round": 2,
  "verdict": "FAIL",
  "mergeable": false,
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
    },
    {
      "id": "FR-062/requirement-records",
      "path": "spec/functional/FR-062-implement-checked-family-contract.md",
      "role": "examined",
      "excerpt": "When a narrow directly wraps the scalar\n  application at this site, the record SHALL carry that direct (innermost)\n  narrow's canonical `OccurrenceKey` (its `expression` occurrence) as its\n  conversion context."
    }
  ],
  "dispositions": [
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "after_excerpt": "        let integer = self.type_ref(&ValueType::Integer, node.location())?;\n        self.typed_application(\n            node.location(),\n            integer,\n            Operator::Binary,\n            Operation::plain(\"filament.op.integer.mod\"),\n            arguments,\n        )",
      "reason": "The gate-discovered catalog mismatch is fixed: mod applications use catalog Integer, while direct bounded consumers receive explicit Coerce/narrow wrappers. This disposition covers that original mismatch, not the separately reported claim-record regression."
    },
    {
      "fnd": "FND-003",
      "outcome": "still-open",
      "reason": "claims.rs:840-853 still uses the ValueType-only coerce_builds_narrow predicate, while lowering.rs:4448-4460 overrides it for direct mod. For m1(a:Int[0,100],b:Int[1,12]):Int[0,11]{a mod b}, the emitted direct narrow exists but the claim is SiteBound::Own(Int[0,11]); key_claims selects the mod application's Integer type node and no conversion_context. Wider bounded consumers also lose their actual target bound. fsl-route emits that wrong node/context, while replay reads the bounded ValueType, so proof and replay disagree."
    }
  ],
  "reaffirmed": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "reason": "static_range operand extraction and prioritization remain present."
    }
  ],
  "new_findings": [
    {
      "fnd": "FND-002",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "code-bug",
      "artifact_id": "FR-495",
      "path": "fsl-semantics/src/check/lowering.rs",
      "lines": "4257-4277",
      "finding": "Historical gate regression: exact checked mod types were emitted directly as the operation result type, violating the catalog Integer signature and causing the reported fsl-replay reader failures. Fixed by this head.",
      "excerpt": "        let integer = self.type_ref(&ValueType::Integer, node.location())?;\n        self.typed_application(\n            node.location(),\n            integer,\n            Operator::Binary,\n            Operation::plain(\"filament.op.integer.mod\"),\n            arguments,\n        )"
    },
    {
      "fnd": "FND-003",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "code-bug",
      "artifact_id": "FR-062",
      "related_ids": [
        "FR-495",
        "FR-062-AC-13"
      ],
      "path": "fsl-semantics/src/check/lowering.rs",
      "lines": "4448-4460",
      "finding": "claims.rs:840-853 still uses the ValueType-only coerce_builds_narrow predicate, while lowering.rs:4448-4460 overrides it for direct mod. For m1(a:Int[0,100],b:Int[1,12]):Int[0,11]{a mod b}, the emitted direct narrow exists but the claim is SiteBound::Own(Int[0,11]); key_claims selects the mod application's Integer type node and no conversion_context. Wider bounded consumers also lose their actual target bound. fsl-route emits that wrong node/context, while replay reads the bounded ValueType, so proof and replay disagree.",
      "excerpt": "When a narrow directly wraps the scalar\n  application at this site, the record SHALL carry that direct (innermost)\n  narrow's canonical `OccurrenceKey` (its `expression` occurrence) as its\n  conversion context."
    }
  ]
}
```


## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | The new coerce arm overwrites a finite mod node's Int[0,11] type with Integer whenever its direct consumer expects Integer. The already-added integer(a:Int[0,100],b:Int[1,12]):Integer { a mod b } fixture therefore violates FR-495's explicit checked-node rule. Keep the exact checked interval and model any consumer/wire widening separately; do not erase the interval to align claim metadata. | fsl-semantics/src/check/check.rs:2743-2754 |

## Round 3 publication

Linear comment `e139c917-31f6-4030-bb92-0c4bdae0b62c`. Exact body read-back, finding/disposition checks and real YAML parsing via quire parse passed. Quire validation passed with nonfatal inline-data-schema and duplicate-archetype warnings. Original review and prior-round history are preserved.

## Disposition round 3 verdict

**FAIL — PR #2738 is not mergeable at b1f9c834839852ba46ce3feb9cfa96d57c7d84af.** FND-003 remains open; new FND-004 is open. Earlier closed findings remain fixed.

Reviewed the complete b1f9c8348 delta and its actual checker, claim classification, keying, routing, and replay consumers without executing tests. coerce_node_builds_narrow is now shared by claims and lowering, and the bounded cases correctly retain the narrow occurrence and target type node. The original static-range fix, bounded-node assertions, reader regression and ignored QSL-1989 rem assertions remain. No div/rem or zero-divisor change was introduced. No source edits, builds, tests, aggregate gate, or GitHub comments. Plan completion: not assessed. Prior matrix evidence remains historical; this pass inspected new trace bindings and assertions. GitHub reports OPEN/MERGEABLE mechanically, but the reviewer verdict is not mergeable.

The shared predicate fixes direct equal/wider bounded consumers, but an inferred mod still produces an inconsistent own-bound record. For function f using v(a:Int[0,100],b:Int[1,12]):Integer pure { (a mod b) + 1 }, arithmetic infers its operands (typing.rs:2118-2131,3064-3090), retains the mod Int[0,11], and adds no Coerce around it. claims.rs:996-999 records Own(Int[0,11]); lowering.rs:4262-4277 emits Integer; key_claims chooses that Integer type node at claims.rs:1103-1125. The record and routed request still disagree with replay's bounded ValueType.

The new coerce arm overwrites a finite mod node's Int[0,11] type with Integer whenever its direct consumer expects Integer. The already-added integer(a:Int[0,100],b:Int[1,12]):Integer { a mod b } fixture therefore violates FR-495's explicit checked-node rule. Keep the exact checked interval and model any consumer/wire widening separately; do not erase the interval to align claim metadata.

```yaml
{
  "head": "b1f9c834839852ba46ce3feb9cfa96d57c7d84af",
  "round": 3,
  "verdict": "FAIL",
  "mergeable": false,
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
    },
    {
      "id": "FR-062/requirement-records",
      "path": "spec/functional/FR-062-implement-checked-family-contract.md",
      "role": "examined",
      "excerpt": "When a narrow directly wraps the scalar\n  application at this site, the record SHALL carry that direct (innermost)\n  narrow's canonical `OccurrenceKey` (its `expression` occurrence) as its\n  conversion context."
    },
    {
      "id": "FR-495/checked-node-type",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "The checked node of `a div b`,\n  `a rem b` and `a mod b` SHALL have the type `Int[lo, hi]` of that interval\n  when the interval is finite, and `Integer` only when it is unbounded"
    }
  ],
  "dispositions": [
    {
      "fnd": "FND-003",
      "outcome": "still-open",
      "reason": "The shared predicate fixes direct equal/wider bounded consumers, but an inferred mod still produces an inconsistent own-bound record. For function f using v(a:Int[0,100],b:Int[1,12]):Integer pure { (a mod b) + 1 }, arithmetic infers its operands (typing.rs:2118-2131,3064-3090), retains the mod Int[0,11], and adds no Coerce around it. claims.rs:996-999 records Own(Int[0,11]); lowering.rs:4262-4277 emits Integer; key_claims chooses that Integer type node at claims.rs:1103-1125. The record and routed request still disagree with replay's bounded ValueType."
    },
    {
      "fnd": "FND-004",
      "outcome": "still-open",
      "reason": "The new coerce arm overwrites a finite mod node's Int[0,11] type with Integer whenever its direct consumer expects Integer. The already-added integer(a:Int[0,100],b:Int[1,12]):Integer { a mod b } fixture therefore violates FR-495's explicit checked-node rule. Keep the exact checked interval and model any consumer/wire widening separately; do not erase the interval to align claim metadata."
    }
  ],
  "reaffirmed": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "reason": "The static_range extraction and finite operand range priority remain, including literal/arithmetic/negation cases."
    },
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "reason": "Catalog mod applications still use Integer; direct bounded coercions retain explicit narrow."
    }
  ],
  "new_findings": [
    {
      "fnd": "FND-004",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "code-bug",
      "artifact_id": "FR-495",
      "path": "fsl-semantics/src/check/check.rs",
      "lines": "2743-2754",
      "excerpt": "The checked node of `a div b`,\n  `a rem b` and `a mod b` SHALL have the type `Int[lo, hi]` of that interval\n  when the interval is finite, and `Integer` only when it is unbounded",
      "finding": "The new coerce arm overwrites a finite mod node's Int[0,11] type with Integer whenever its direct consumer expects Integer. The already-added integer(a:Int[0,100],b:Int[1,12]):Integer { a mod b } fixture therefore violates FR-495's explicit checked-node rule. Keep the exact checked interval and model any consumer/wire widening separately; do not erase the interval to align claim metadata."
    }
  ]
}
```

