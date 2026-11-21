---
id: SR-568
title: "QSL-2158 decimal retention counter gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2158; fsl-foundation/src/scalar_wire.rs; fsl-replay/src/probability_certificate/document/writer.rs; FR-201-AC-9 canonical integer transport portion; FR-255-AC-1 caller-budget portion"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Ticket: QSL-2158. PR: filament-spec-language#3382. Frozen head `1b366a59ae9bef53c1ad4067012af7284aafee94`, accepted base `5deeba7d06bfd9b99b9874a3766a302be9b0c3e2`. Independent Codex review, codebase read-only. No applicable frontmatter `type: AssuranceProfile` exists in spec/. Repository AGENTS.md/CLAUDE.md and Rust style/API/implementation/safety/boundary/testing rules were applied. No source, test, spec, manifest, pin or CI contract changed beyond the two assigned source files; the diff has no CI gate changes.

Tools: quoin 0.36.8; quire 0.52.0 (engine 0.62.2); spec-artifacts-process@v0.36.0. These installed versions are newer than the historical release notice; no old binary fallback was used. SR-567 and SR-568 were absent from both the frozen branch and actual main `85a8634c98aef7ae01d76ca722536ee5e51f559d` when checked, including quoted frontmatter IDs. Quoin authoring pack, installed skeleton and schema were read.

## Verdict

**PASS** for the assigned two-file traceability, reverse-gap and stub review subset. No actionable findings. This is not a whole-repository qualification verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review evidence

This is a bounded, planless gap analysis over the two changed owners and their three writer regressions. It makes no whole-repository assurance claim. The engine computed the repository matrix; only FR-201-AC-9's canonical integer transport portion and FR-255-AC-1's caller-budget refusal portion were selected and inspected. Each selected criterion is `tagged`; all three actual writer functions are engine-bound to both IDs (six bindings). Each test provides partial evidence for the larger criterion, not full discharge of every clause. The FR-255 scope excerpt records its caller-budget sentence; later explore/supplied-value/format clauses were not audited.

The complete matrix and coverage commands both exited 0. Existing broad collision diagnostics concern other source paths; no diagnostic, untracked symbol or unmatched tag names either changed source owner. The new test is a bound Rust test function, not a trace comment counted by grep. Broad untagged requirements and unrelated unmarked claims are outside this assigned review; this PASS must not be read as their resolution. No execution evidence store was read.

Reverse-gap inventory: three production behaviors (legacy quoted integer entry, shared bounded canonical decimal materialization/error mapping, replay admitted integer transport/observation) and three regression test behaviors were inspected. All map to the existing canonical transport/caller-limit contracts above; no new externally visible transport grammar or control was added. Untraced changed behaviors 0; source stubs 0; test stubs 0; coverage inflation cases 0. The new helper replaces the old conversion body rather than creating a second owner. The tests have exact bytes, typed refusal/count assertions and real execution seams, with no ignored/guarded-away assertions. No thresholds or gates were relaxed.

## Coverage

Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2) — no run evidence read. Selected criteria: tagged 2; untagged 0; tagged-by-ignored-test 0; method-without-symbol 0; qualified-record 0; retired 0. Six correct scoped writer-test bindings were examined. `quire coverage` supplied no changed-owner untracked/malformed/unmatched binding. Whole-repo diagnostics and coverage counts are not a scoped PASS claim.

Plan completion: not assessed

Optional separate semantic-review fan-out: skipped (not requested). Ordinary code/test intent inspection is included in the code/Rust method.

Execution evidence is attributed to the root's admitted Titan run, not rerun or independently measured by this reviewer: frozen-head `cargo check --locked --tests -p fsl-foundation -p fsl-replay` exit 0; each of the three exact writer tests (bounded kernel admission, existing bytes/storage/exact-output controls, and new positive/sink-refusal control) exit 0, aggregate 3 passed / 0 failed / 0 ignored. No local Cargo, clippy, broad suite, baseline or measurements were run per dispatch. `git diff --check` over the exact base/head passed locally. Broad `make ci`, all-target/all-feature clippy and whole-workspace qualification are not claimed.

## Examined units and bindings

```json
{
  "clean": true,
  "scope": [
    {
      "id": "FR-201-AC-9",
      "path": "spec/functional/FR-201-check-a-probability-certificate.md",
      "role": "examined",
      "excerpt": "For an admitted acyclic eventual claim, writing and reading the closed profile preserves identity, exact rational values, complete keys, targets and required ranks byte for byte. Duplicate/out-of-order rows, unreduced 2/4, denominator 0, missing ranking, extra fields, padded base64url and a model-only key are rejected."
    },
    {
      "id": "FR-255-AC-1",
      "path": "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md",
      "role": "examined",
      "excerpt": "For each row of the setting table, driving its stage or operation past that limit at a configured bound `B` returns the code Behavior 1's table gives for the limit's carrying type, with the row's limit kind, bound `B`, the count the refused charge would have reached, and the row's setting name, and, for a `stage_limit_exceeded` row, its catalog record carries fields `kind`, `bound`, `actual` and `setting` holding those values; the caller-budget rows are as Behavior 1 states."
    },
    {
      "id": "scalar_wire::integer",
      "path": "fsl-foundation/src/scalar_wire.rs",
      "role": "examined",
      "excerpt": "writer.string(&integer_text(value, max_bytes, budget)?)"
    },
    {
      "id": "scalar_wire::integer_text",
      "path": "fsl-foundation/src/scalar_wire.rs",
      "role": "examined",
      "excerpt": "Some(budget) => value.try_to_decimal_string_with_budget(limit, budget),"
    },
    {
      "id": "Text::integer",
      "path": "fsl-replay/src/probability_certificate/document/writer.rs",
      "role": "examined",
      "excerpt": "fsl_foundation::scalar_wire::integer_text(integer, remaining, None)\n            .and_then(|rendered| {\n                self.text_reserved();\n                writer.string(&rendered)\n            })"
    },
    {
      "id": "tests::bounded_kernel_decimal_output_is_admitted_before_retaining_transport_text",
      "path": "fsl-replay/src/probability_certificate/document/writer.rs",
      "role": "examined",
      "excerpt": "\"oversized decimal transport was not retained\""
    },
    {
      "id": "tests::decimal_transport_retention_is_observed_before_sink_refusal",
      "path": "fsl-replay/src/probability_certificate/document/writer.rs",
      "role": "examined",
      "excerpt": "assert_eq!(calls.get(), 1, \"the actual sink refused its first write\");"
    },
    {
      "id": "tests::writer_admits_bytes_and_storage_before_transport_text_and_preserves_exact_budget_output",
      "path": "fsl-replay/src/probability_certificate/document/writer.rs",
      "role": "examined",
      "excerpt": "\"no base64 or decimal text was materialized before admission: {error:?}\""
    }
  ],
  "findings": [],
  "bindings": [
    {
      "test_id": "tests::bounded_kernel_decimal_output_is_admitted_before_retaining_transport_text",
      "ac_id": "FR-201-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "tests::decimal_transport_retention_is_observed_before_sink_refusal",
      "ac_id": "FR-201-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "tests::writer_admits_bytes_and_storage_before_transport_text_and_preserves_exact_budget_output",
      "ac_id": "FR-201-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "tests::bounded_kernel_decimal_output_is_admitted_before_retaining_transport_text",
      "ac_id": "FR-255-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "tests::decimal_transport_retention_is_observed_before_sink_refusal",
      "ac_id": "FR-255-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "tests::writer_admits_bytes_and_storage_before_transport_text_and_preserves_exact_budget_output",
      "ac_id": "FR-255-AC-1",
      "trace": "correct"
    }
  ]
}
```

## Publication status

Private Linear publication completed after the timed hold and explicit root release. Comment `887b03d7-df25-4b67-95a6-ea13961742f4` was published once after one latest-100 dedup batch. One latest-100 read-back batch returned the exact draft body; offline YAML parsing, finding/placeholder parity, full scope/data equality, clean flag and six binding checks passed. Existing has-review was supplied by dispatch and was not queried or changed. No GitHub findings or issue state changes were made. Runtime evidence remains attributed to reviewed source head 1b366a59ae9bef53c1ad4067012af7284aafee94, with admitted check exit 0 and three exact regressions passing; publication does not qualify a later rebase or merge head.
