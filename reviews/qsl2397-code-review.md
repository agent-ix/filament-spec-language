---
id: SR-2026111612172400000031
title: "QSL-2397 conformance expectations code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2397; fsl-conformance/src/cases.rs; fsl-conformance/tests/it/execute_row.rs; fsl-conformance/tests/it/front_end_rows.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-286", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-564", type: references }
---

## Summary

Ticket QSL-2397; PR #2877. Frozen head ee489bddf1b0784fd73c20ef366f144409eb3ddc against c3a811816daaa9811c72a336495481eb7938c901. Reviewed only the requested conformance expectation corrections.

## Verdict

PASS for the bounded change. FR-286-AC-5 explicitly requires the integer member. Both trace additions correctly bind the integer-completion subcase, without claiming each test covers every compound AC clause. The package fixture is incomplete at 20,514 and successful at 20,515; the existing test checks both sides through the real package path. The only src edit is the conformance fixture constant, not production behavior. No unrelated scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

Rust lane checked assertions, trace bindings, fixture ownership, integer wire shape, inclusive output bound, diagnostic context, duplication, panic usage, and scope. No new helper, vendored content, production algorithm, API, dependency, CI change, warning suppression, or weakened gate. No applicable AssuranceProfile found in spec/.

Targeted verification: execute_row::execute_ (2 passed); front_end_rows::the_incomplete_cells_are_one_below_the_counter_their_input_reaches (1 passed); rustfmt --check on the three changed files and git diff --check passed. No aggregate gate or unfiltered suite was run, per direct owner order.

## Examined units and bindings

Compilation reported an existing dead-code warning for `Stage::QuireContext` at fsl-command/src/command/output/types.rs:14. That file is unchanged by this PR; the warning was not suppressed or repaired in this bounded review.

```yaml
{
  "bindings": [
    {
      "ac_id": "FR-286-AC-5",
      "test_id": "execute_returns_each_applicable_kind",
      "trace": "correct"
    },
    {
      "ac_id": "FR-564-AC-3",
      "test_id": "execute_returns_each_applicable_kind",
      "trace": "correct"
    },
    {
      "ac_id": "FR-286-AC-5",
      "test_id": "execute_preserves_the_cases_non_work_accounting_limit",
      "trace": "correct"
    },
    {
      "ac_id": "FR-564-AC-1",
      "test_id": "the_incomplete_cells_are_one_below_the_counter_their_input_reaches",
      "trace": "correct"
    },
    {
      "ac_id": "FR-564-AC-2",
      "test_id": "the_incomplete_cells_are_one_below_the_counter_their_input_reaches",
      "trace": "correct"
    }
  ],
  "clean": true,
  "findings": [],
  "scope": [
    {
      "excerpt": "The `execute` outcome of FR-100-AC-1's `seven` serializes with `category` success and `result` `{\"kind\": \"completed\", \"value\": {\"integer\": \"7\"}}`",
      "id": "FR-286-AC-5",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined"
    },
    {
      "excerpt": "Each of `check`'s six applicable cells returns its kind: success exit 0; refusal `ill_typed` exit 20; unsupported `unsupported_construct`/`declaration-form` exit 21; timeout `StageFailure::Cancelled(Deadline)` exit 22; cancellation `StageFailure::Cancelled(Requested)` exit 22; incomplete `LimitExceeded` naming `s3.nodes` exit 22.",
      "id": "FR-564-AC-1",
      "path": "spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md",
      "role": "examined"
    },
    {
      "excerpt": "Each of `package`'s five applicable cells returns its kind: success exit 0; unsupported `unsupported_construct` exit 21; timeout `Cancelled(Deadline)`, cancellation `Cancelled(Requested)` and incomplete `LimitExceeded` naming the `package` limit field, each exit 22.",
      "id": "FR-564-AC-2",
      "path": "spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md",
      "role": "examined"
    },
    {
      "excerpt": "Each of `execute`'s seven applicable cells returns its kind: `seven` completes 7, exit 0; `id(12)` `CallFailure::Input`, exit 20; `jit` not supplied `CallFailure::Unsupported` cause `NotSupplied`, exit 21; `CallFailure::Cancelled(Deadline)` and `CallFailure::Cancelled(Requested)`, exit 22; `Incomplete` naming `work_units`, exit 22; the fault-injecting backend's `CallFailure::Fault`, exit 30, no value.",
      "id": "FR-564-AC-3",
      "path": "spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md",
      "role": "examined"
    },
    {
      "excerpt": "const COMPILE_OUTPUT_BYTES_BELOW: u64 = 20_514;",
      "id": "fsl-conformance/src/cases.rs",
      "path": "fsl-conformance/src/cases.rs",
      "role": "examined"
    },
    {
      "excerpt": "assert_eq!(written_success[\"result\"][\"value\"][\"integer\"], \"7\");",
      "id": "fsl-conformance/tests/it/execute_row.rs",
      "path": "fsl-conformance/tests/it/execute_row.rs",
      "role": "examined"
    },
    {
      "excerpt": "limit: limit_at_counter(below + 1),",
      "id": "fsl-conformance/tests/it/front_end_rows.rs",
      "path": "fsl-conformance/tests/it/front_end_rows.rs",
      "role": "examined"
    }
  ]
}
```
