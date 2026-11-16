---
id: SR-2026111414542500000013
title: "gap-analysis of QSL-2002 and QSL-2004 numeric batch"
type: SpecReview
analysis: gap-analysis
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

**CONDITIONAL** — Traced source-backed tests miss the mod interval interaction exposed by the code review. This is not a whole-repository assurance verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The mod regressions do not cover a bounded dividend narrower than every member of a non-singleton divisor interval, allowing the incorrect Int[0,5] result for Int[0,2] mod Int[5,6] to pass. | fsl-semantics/tests/it/division_law_selection.rs:189-235 |

## Analysis

The modified mod test combines a wide dividend with a variable divisor (`[0,100]` by `[1,12]`), and a narrow dividend only with a singleton divisor (`[0,2]` by `[5,5]`). It misses the interaction where the new non-singleton branch discards the narrow dividend. The code-review counterexample therefore escapes all existing regression checks. Add `Int[0,2] mod Int[5,6]`, asserting both acceptance by an `Int[0,2]` consumer and exact node type `Int[0,2]`. This is a coverage defect supporting the separate implementation blocker, not an expansion into QSL-1962.

Both float regression bindings correctly name FSpec FR-149-AC-10. Their helper checks exactly one type-mismatch refusal at the converted operand. The target-float guard is shared across widths; existing ordinary IEEE conversion tests remain passing in the coder log. The gap verdict concerns the requested delta; whole-repository baseline gaps are not declared resolved.

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
      "severity": "medium",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-495",
      "related_ids": [
        "FR-495-AC-17",
        "FSpec-FR-147-AC-12"
      ],
      "path": "fsl-semantics/tests/it/division_law_selection.rs",
      "lines": "189-235",
      "excerpt": "- **Range of an integer division.** The checker SHALL give `a div b`, `a rem b`\n  and `a mod b` the smallest interval that contains the result of the operator\n  for every pair `(x, y)` with `x` in the range of `a`, `y` in the range of `b`\n  and `y` not zero, taking the retained division law for `div` and `rem`\n  (FSpec FR-147) and the Euclidean remainder for `mod`.",
      "finding": "The mod regressions do not cover a bounded dividend narrower than every member of a non-singleton divisor interval, allowing the incorrect Int[0,5] result for Int[0,2] mod Int[5,6] to pass."
    }
  ]
}
```

