---
id: SR-2026111414542500000012
title: "code-review of QSL-2002 and QSL-2004 numeric batch"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 2731; QSL-2002 and QSL-2004; fsl-semantics/src/check/check/typing.rs; fsl-semantics/tests/it/division_law_selection.rs; fsl-semantics/tests/it/native_scalar_equality.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: references
---

## Summary

Ticket: QSL-2002; combined PR #2731 also covers QSL-2004. Reviewed head `1cee0bd76cfe6b9b28fedca036c261cf30df89dd` against `origin/main` `7ef5979d3181f5757ae733e59f2c5bf4e1b1a505`. The complete three-file diff was reviewed in the assigned worktree. Scope excludes QSL-1962 zero-divisor obligations and div/rem implementation. No spec files changed, so no spec-review method applies.

Normative sources are current FSL FR-495 and FSpec FR-147/FR-149, with TC-1783, TC-1281 and TC-194. Ticket prose did not determine behavior. Scope excerpts are verbatim excerpts; AC-17 and FSpec AC-12 examination and bindings cover their mod range subcases, not excluded subcases.

## Verdict

**FAIL** — The required smallest mod interval is not implemented for non-singleton divisors, falsely rejecting valid bounded consumers. PR #2731 is not ready to merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | For a: Int[0,2] and b: Int[5,6], mod is assigned Int[0,5] instead of the required smallest interval Int[0,2], so a valid Int[0,2] consumer is refused. | fsl-semantics/src/check/check/typing.rs:130-150 |

## Analysis

The helper reads the dividend only when the divisor interval is a singleton. Otherwise it returns `[0, max(abs(lower), abs(upper)) - 1]`. With `a: Int[0,2]` and `b: Int[5,6]`, every pair has `0 <= a < b`, so every Euclidean remainder equals `a`, attaining exactly endpoints 0 and 2. Returning `[0,5]` is a sound enclosure but violates FR-495's smallest-interval requirement.

Static trace: `integer_modulo_result_type` -> IntegerDivision node's value_type -> IntervalWalk declared-type fallback at facts.rs:1719-1722 -> Coerce containment check at facts.rs:1235-1248. Thus `function narrow using v(a: Int[0,2], b: Int[5,6]): Int[0,2] pure { a mod b }` receives unproved-range under Linked. This was proved by code tracing, not by running a test. Both divisor values are positive; no zero-divisor behavior is involved. Implement the required smallest interval for non-singleton divisors and add source-level acceptance plus exact-node-type regression.

For QSL-2004 the float-target guard delegates to the pinned semantic-value admits_equality_conversion (Cargo.lock c943d9560f1c614ad6b8512bfe8b8f2963f8122d); Float32 and Float64 targets use the same closed table. Direct and one-sided exact-to-float conversions refuse before conversion-node construction or runtime charge. Existing ordinary conversion and float-source paths were inspected as context, not claimed as new implementation. No defect was found in the changed target-float behavior.

Rust lane: ownership, fallibility, fixed-count big-integer operations, refusal locations and node/coercion boundary examined. No new panic, unchecked narrowing, unsafe code or dependency duplication found. No equivalent general mod interval helper found in the existing checker or pinned exact-value source. Equality policy remains in semantic-value. No applicable AssuranceProfile was found under spec/.

## Coverage

Reconciliation: quoin matrix (quoin 0.29.4, quire 0.38.3), with quire matrix and coverage cross-checks. The commands scanned the whole repository; adjudication is restricted to the requested PR delta. Static criteria: tagged 3676; untagged 1043; tagged-by-ignored-test 259; method-without-symbol 98; retired 13. All 5089 criteria have `no run evidence` in the Quoin store. Separately inspected coder logs are identified below, not represented as store evidence.

FR-495-AC-17 is tagged by the newly enabled mod test alongside two ignored zero-divisor tests outside this batch. Tagged status does not establish delivery of all subcases. Coverage returned 914 diagnostics and 4397 untracked symbols repository-wide. The changed tests' documented external `FSpec-` tags do not resolve in the FSL-only model and were manually verified against the current FSpec checkout; they are not stale IDs. Broader baseline and selector diagnostics are retained in coverage.json and are outside this PR-delta verdict.

Plan completion: not assessed

Reverse-gap inventory for the delta: two production behaviors (mod result typing; direct equality-conversion admission), both owned by reviewed requirements; zero untraced behaviors; zero source stubs; zero test stubs. Tests use real source assembly and checking, with exact types or located refusals. No mocks, copied implementation oracle, vacuous assertions, new dependency, wire/API change, unsafe code, I/O, concurrency or feature-selection changes were introduced. Optional whole-repository semantic fan-out was not run; targeted intent/test/code examination was part of the requested complete-diff review.

## Validation evidence and limitations

Read-only evidence inspection: qsl-2002-focused2.log records the intended red mod assertion after the initial compile issue was corrected; qsl-2002-focused3.log records green. qsl-2002-crate-test.log records 900 unit tests passed (3 ignored), 581 integration tests passed (23 ignored), and 26 doctests passed, including the mod regression and both float-conversion regressions. Workspace check, default/no-default clippy and architecture-check logs show success. No build, test, gate, source mutation, commit, push or merge was performed by this reviewer. These logs are coder evidence, not independent execution or proof against the counterexample. The lead-owned aggregate gate remains pending.

Tools on Draco: `/Users/peter/.local/bin/quoin` 0.29.4; `/Users/peter/.cargo/bin/quire` 0.38.3 (engine 0.52.1); spec-artifacts-process@v0.29.2. No review method was unavailable. Matrix and coverage commands exited 0. Quire stderr reported `semantic.inline-data-schema` and duplicate ADR/Plan/Review/SpecReview/Standard archetypes from spec-artifacts-process with first-wins selection. Reproduction: `quire coverage --scope /Users/peter/dev/qsl-2002-code --json`; expected one active archetype definition, observed duplicate-archetype diagnostics. Outputs are retained here.

## Examined scope and bindings

```yaml
{
  "clean": false,
  "scope": [
    {
      "id": "FR-495",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "- **Range of an integer division.** The checker SHALL give `a div b`, `a rem b`\n  and `a mod b` the smallest interval that contains the result of the operator\n  for every pair `(x, y)` with `x` in the range of `a`, `y` in the range of `b`\n  and `y` not zero, taking the retained division law for `div` and `rem`\n  (FSpec FR-147) and the Euclidean remainder for `mod`."
    },
    {
      "id": "FR-495",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "- **Type of an integer division node.** The checked node of `a div b`,\n  `a rem b` and `a mod b` SHALL have the type `Int[lo, hi]` of that interval\n  when the interval is finite, and `Integer` only when it is unbounded, so a\n  consumer reads the proved range from the node's type as it does for `+`\n  and `*`."
    },
    {
      "id": "FR-495-AC-17",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "`function m1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure { a mod b }` checks and the checked `mod` node has type `Int[0, 11]`; `function m2 using v(a: Integer, b: Int[1, 12]): Int[0, 11] pure { a mod b }` checks (the remainder is bounded by the divisor alone); `function m3 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 10] pure { a mod b }` refuses `unproved-range` at the `mod`."
    },
    {
      "id": "FSpec-FR-147-AC-12",
      "path": "../filament-specification/spec/functional/expressions/FR-147-evaluate-integer-division-domains.md",
      "role": "examined",
      "excerpt": "For `a: Int[0, 100]` and `b: Int[1, 12]`, `a mod b` has result type `Int[0, 11]` and is admitted into an `Int[0, 11]` consumer, and is refused `unproved-range` into `Int[0, 10]`; with `a` of the unbounded integer type `a mod b` still has type `Int[0, 11]`."
    },
    {
      "id": "FSpec-FR-149-AC-10",
      "path": "../filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md",
      "role": "examined",
      "excerpt": "Exactly the tabled equality conversions are admitted from declared bounds, every other conversion operand of `=` or `!=` is `ill_typed` before any charge even when the value fits, and admitted decimal and rational conversions charge their decimal schedule."
    },
    {
      "id": "FSpec-FR-149",
      "path": "../filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md",
      "role": "examined",
      "excerpt": "An operand of `=` or `!=` whose expression, ignoring parentheses, is\n`convert<T>(e)` is an equality conversion from the static type `S` of `e`. It is\nadmitted only when (`S`, `T`) is in the closed table below and the other operand\nthen has type `T`. Every other equality conversion is\n`refused { code: ill_typed }` before any charge, even when the particular value\nwould fit."
    },
    {
      "id": "TC-1783",
      "path": "spec/test-cases/TC-1783-integer-division-nodes-carry-their-interval-and-nonzero-obligation.md",
      "role": "context_only",
      "excerpt": "Verify from source that `mod`, `div` and `rem` refuse a possibly-zero divisor\nunder `CheckMode::Linked`, that their checked node type is the proved\ninterval, that a remainder is bounded by its divisor even for an unbounded\ndividend, and that only `CheckMode::Kernel` reaches a run-time zero divisor.\n"
    },
    {
      "id": "FSpec-TC-1281",
      "path": "../filament-specification/spec/test-cases/TC-1281-integer-division-result-interval-and-nonzero-obligation.md",
      "role": "context_only",
      "excerpt": "1. Check `a mod b` over `a: Int[0, 100]`, `b: Int[1, 12]` into `Int[0, 11]`\n   and into `Int[0, 10]`; then with `a` unbounded into `Int[0, 11]`.\n"
    },
    {
      "id": "FSpec-TC-194",
      "path": "../filament-specification/spec/test-cases/TC-194-complete-equality-matrix.md",
      "role": "context_only",
      "excerpt": "| E19 | binary32 `1.0` and binary64 `1.0` without conversion, then after explicit exact conversion to binary64 | ill-typed; numeric equality true while both source widths remain unchanged |"
    },
    {
      "id": "integer_modulo_result_type",
      "path": "fsl-semantics/src/check/check/typing.rs",
      "role": "examined",
      "excerpt": "/// The smallest bounded result interval we can derive for Euclidean `mod`\n/// from the operands' declared integer intervals.\nfn integer_modulo_result_type(dividend: &ValueType, divisor: &ValueType) -> ValueType {\n    let ValueType::Int(divisor) = divisor else {\n        return ValueType::Integer;\n    };\n"
    },
    {
      "id": "a_mod_node_is_typed_by_its_divisor_interval",
      "path": "fsl-semantics/tests/it/division_law_selection.rs",
      "role": "examined",
      "excerpt": "#[trace(\"TC-1783\", \"FR-495-AC-17\", \"FSpec-TC-1281\", \"FSpec-FR-147-AC-12\")]\n#[test]\nfn a_mod_node_is_typed_by_its_divisor_interval() {\n"
    },
    {
      "id": "two_float_conversions_under_equality_refuse_type_mismatch",
      "path": "fsl-semantics/tests/it/native_scalar_equality.rs",
      "role": "examined",
      "excerpt": "fn two_float_conversions_under_equality_refuse_type_mismatch() {\n    refused_at_equality_conversion(\n        \"function bad using v(a: Int[0, 1]): Boolean pure { convert<Float64[nearest-even]>(a) = convert<Float64[nearest-even]>(a) }\",\n    );\n}\n\n"
    },
    {
      "id": "one_float_conversion_under_equality_refuses_type_mismatch",
      "path": "fsl-semantics/tests/it/native_scalar_equality.rs",
      "role": "examined",
      "excerpt": "fn one_float_conversion_under_equality_refuses_type_mismatch() {\n    refused_at_equality_conversion(\n        \"function bad using v(a: Int[0, 1], b: Float64[nearest-even]): Boolean pure { convert<Float64[nearest-even]>(a) = b }\",\n    );\n}\n\n"
    }
  ],
  "bindings": [
    {
      "test_id": "a_mod_node_is_typed_by_its_divisor_interval",
      "ac_id": "FR-495-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "a_mod_node_is_typed_by_its_divisor_interval",
      "ac_id": "FSpec-FR-147-AC-12",
      "trace": "correct"
    },
    {
      "test_id": "two_float_conversions_under_equality_refuse_type_mismatch",
      "ac_id": "FSpec-FR-149-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "one_float_conversion_under_equality_refuses_type_mismatch",
      "ac_id": "FSpec-FR-149-AC-10",
      "trace": "correct"
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
      "lines": "130-150",
      "excerpt": "- **Range of an integer division.** The checker SHALL give `a div b`, `a rem b`\n  and `a mod b` the smallest interval that contains the result of the operator\n  for every pair `(x, y)` with `x` in the range of `a`, `y` in the range of `b`\n  and `y` not zero, taking the retained division law for `div` and `rem`\n  (FSpec FR-147) and the Euclidean remainder for `mod`.",
      "finding": "For a: Int[0,2] and b: Int[5,6], mod is assigned Int[0,5] instead of the required smallest interval Int[0,2], so a valid Int[0,2] consumer is refused."
    }
  ]
}
```

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | For a: Int[6,6] and b: Int[4,5], mod still receives Int[0,4] instead of the smallest interval Int[1,2], so a valid Int[1,2] consumer is refused. | fsl-semantics/src/check/check/typing.rs:130-163 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: the exact [0,2] mod [5,6] case now returns [0,2], and m5 asserts bounded-consumer acceptance and exact node type. |
| FND-002 | still-open | Round 1: The new shortcut does not apply to dividend [6,6] and divisor [4,5]; the fallback still produces [0,4] instead of [1,2]. |

## Disposition pass 1 verdict

**FAIL** — original FND-001 fixed; new FND-002 remains open. PR #2731 is blocked on the remaining smallest-interval defect.

Disposition pass round 1 reviewed head `829feed978d7aeb9011c36f9c69425f514501590` against current origin/main `9d832887eb83748422375474896fb102c33333ac` and the prior reviewed head `1cee0bd76cfe6b9b28fedca036c261cf30df89dd`. The original finding text and review-pass verdict above remain unchanged historical records.

The exact original counterexample is fixed: the new nonnegative/below-smallest-divisor branch returns the dividend interval, and m5 checks `Int[0,2] mod Int[5,6]` into `Int[0,2]`; function index 3 is explicitly compared with `Int[0,2]`. The appended positive-divisor counterexample does not enter that shortcut: 6 is not less than 4, and [4,5] is not a singleton. The fallback therefore gives [0,4]. Since 6 mod 4 = 2 and 6 mod 5 = 1, the exact interval is [1,2]; the existing declared-node-type/coercion path refuses that valid consumer. The analogous negative-dividend case -1 mod [4,5] has exact interval [3,4] but is also typed [0,4]. Both examples avoid zero divisors.

No new defect was found in QSL-2004's unchanged float-target guard, the existing singleton-divisor computation, or the new below-smallest-modulus shortcut itself. This fix closes one case, not the general smallest-interval obligation. The next fix must address attainable remainder extrema for non-singleton divisor intervals rather than add only the second displayed example.

Evidence: the existing `/private/tmp/qsl-2002-round1-focused.log` names the mod integration test as passing (1 passed, 603 filtered out). It also contains `nice: setpriority: Operation not permitted` and `lockf: /tmp/agent-e-heavy-build.lock: already locked`; this is coder evidence, not an independently executed gate. This reviewer performed no build, test or gate, and modified only scratch review records. Quire validation remains required at the updated explicit scratch paths. No matrix recomputation was needed for this disposition-only delta; the earlier matrix rollup describes the original review head. Repository copies were byte-identical to the scratch artifacts before this round. `git diff --check` flagged their inherited blank EOF lines; appending the disposition sections removes that EOF condition in the updated scratch copies.

```yaml
{
  "head": "829feed978d7aeb9011c36f9c69425f514501590",
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "after_excerpt": "        if dividend.lower() >= &zero && dividend.upper() < &smallest_nonzero_modulus {\n            return ValueType::Int(dividend.clone());\n        }"
    },
    {
      "fnd": "FND-002",
      "outcome": "still-open",
      "reason": "The new shortcut does not apply to dividend [6,6] and divisor [4,5]; the fallback still produces [0,4] instead of [1,2]."
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
      "related_ids": [
        "FR-495-AC-17",
        "FSpec-FR-147-AC-12"
      ],
      "path": "fsl-semantics/src/check/check/typing.rs",
      "lines": "130-163",
      "excerpt": "- **Range of an integer division.** The checker SHALL give `a div b`, `a rem b`\n  and `a mod b` the smallest interval that contains the result of the operator\n  for every pair `(x, y)` with `x` in the range of `a`, `y` in the range of `b`\n  and `y` not zero, taking the retained division law for `div` and `rem`\n  (FSpec FR-147) and the Euclidean remainder for `mod`.",
      "finding": "For a: Int[6,6] and b: Int[4,5], mod still receives Int[0,4] instead of the smallest interval Int[1,2], so a valid Int[1,2] consumer is refused."
    }
  ],
  "scope": [
    {
      "id": "FR-495",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "- **Range of an integer division.** The checker SHALL give `a div b`, `a rem b`\n  and `a mod b` the smallest interval that contains the result of the operator\n  for every pair `(x, y)` with `x` in the range of `a`, `y` in the range of `b`\n  and `y` not zero, taking the retained division law for `div` and `rem`\n  (FSpec FR-147) and the Euclidean remainder for `mod`."
    },
    {
      "id": "FR-495",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "- **Type of an integer division node.** The checked node of `a div b`,\n  `a rem b` and `a mod b` SHALL have the type `Int[lo, hi]` of that interval\n  when the interval is finite, and `Integer` only when it is unbounded, so a\n  consumer reads the proved range from the node's type as it does for `+`\n  and `*`."
    },
    {
      "id": "FR-495-AC-17",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "`function m1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure { a mod b }` checks and the checked `mod` node has type `Int[0, 11]`; `function m2 using v(a: Integer, b: Int[1, 12]): Int[0, 11] pure { a mod b }` checks (the remainder is bounded by the divisor alone); `function m3 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 10] pure { a mod b }` refuses `unproved-range` at the `mod`."
    },
    {
      "id": "FSpec-FR-147-AC-12",
      "path": "../filament-specification/spec/functional/expressions/FR-147-evaluate-integer-division-domains.md",
      "role": "examined",
      "excerpt": "For `a: Int[0, 100]` and `b: Int[1, 12]`, `a mod b` has result type `Int[0, 11]` and is admitted into an `Int[0, 11]` consumer, and is refused `unproved-range` into `Int[0, 10]`; with `a` of the unbounded integer type `a mod b` still has type `Int[0, 11]`."
    },
    {
      "id": "FSpec-FR-149-AC-10",
      "path": "../filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md",
      "role": "examined",
      "excerpt": "Exactly the tabled equality conversions are admitted from declared bounds, every other conversion operand of `=` or `!=` is `ill_typed` before any charge even when the value fits, and admitted decimal and rational conversions charge their decimal schedule."
    },
    {
      "id": "FSpec-FR-149",
      "path": "../filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md",
      "role": "examined",
      "excerpt": "An operand of `=` or `!=` whose expression, ignoring parentheses, is\n`convert<T>(e)` is an equality conversion from the static type `S` of `e`. It is\nadmitted only when (`S`, `T`) is in the closed table below and the other operand\nthen has type `T`. Every other equality conversion is\n`refused { code: ill_typed }` before any charge, even when the particular value\nwould fit."
    }
  ],
  "bindings": [
    {
      "test_id": "a_mod_node_is_typed_by_its_divisor_interval",
      "ac_id": "FR-495-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "a_mod_node_is_typed_by_its_divisor_interval",
      "ac_id": "FSpec-FR-147-AC-12",
      "trace": "correct"
    },
    {
      "test_id": "two_float_conversions_under_equality_refuse_type_mismatch",
      "ac_id": "FSpec-FR-149-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "one_float_conversion_under_equality_refuses_type_mismatch",
      "ac_id": "FSpec-FR-149-AC-10",
      "trace": "correct"
    }
  ]
}
```

