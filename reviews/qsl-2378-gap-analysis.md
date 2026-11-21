---
id: SR-202611210022442313144108529
title: "FSL repository traceability during QSL-2378 review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; spec/, all Rust source and tests; QSL-2378 changed behavior inspected directly"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Repository-driven traceability audit during review of the process-provider prerequisite. The repository has existing untagged and ignored-only criteria; the new tests bind to their declared criteria. Plan completion was not assessed.

## Verdict

FAIL for whole-repository assurance. This records baseline assurance debt, rather than asserting that the small process-provider change introduced it. No full build or test gate ran.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The engine reports 1119 untagged criteria and 83 tagged only by ignored tests in the repository. | quoin matrix; FR-287-AC-3, FR-287-AC-5 through FR-287-AC-9 |
| FND-002 | medium | Coverage reports 5785 untracked symbols, 6632 unmatched tags and extraction diagnostics, including missing requirement headings and duplicate fixture or cfg declarations. | quire coverage; spec/non-functional/NFR-001-bound-syntax-work.md |
| FND-003 | high | Whole-repository reverse ownership and semantic completeness cannot be established from the available extraction alone; direct code ownership inspection covered the changed behavior only. | 2159 Rust files; fsl-analyze/src/provider.rs |

## Coverage

Reconciliation: quoin matrix (quoin 0.36.8, quire 0.52.0, engine 0.62.2). Command exited 0. The separate `quire coverage --scope . --json` also exited 0 and emitted symbol collision diagnostics.

Plan completion: not assessed

Criteria reported by the engine: tagged 4817; untagged 1119; tagged-by-ignored-test 83; method-without-symbol 104; retired 13. All 6136 criteria have no run evidence in the evidence store. Static tagging does not establish that a test ran.

The new memory and CPU tests bind to FR-287-AC-12 and FR-291-AC-1. The two general failure and malformed-cause tests bind to FR-291-AC-1. Both criteria are tagged, but the code review found that FR-287-AC-12 does not describe memory or CPU limits and must be removed from these two tests. A bound tag alone does not demonstrate correct coverage. No added ignored test or unmatched criterion tag was found in this diff.

Source inventory found 2159 Rust files and 32 textual todo/unimplemented candidates. Inspection of the candidate lines found test-source strings, compile-fail documentation and an existing ignored hybrid-request test; none occurs in the two changed files. The new limit mapping and process-failure settlement are owned by FR-287 and FR-291. No new source or test stub, hollow assertion or duplicated failure vocabulary was found in the changed behavior. Whole-repository source ownership remains unestablished, as recorded above.

Semantic review: skipped. No plan was selected or authored. The raw command outputs are retained locally in `/tmp/terra-codex2-qsl-2378-fsl-matrix.json` and `/tmp/terra-codex2-qsl-2378-fsl-coverage.json`.

## Reviewed units

```yaml
{
  "scope": [
    {
      "id": "repository-matrix",
      "path": "spec/, Rust source and tests",
      "role": "examined",
      "excerpt": "Engine statuses: tagged 4817; untagged 1119; tagged-by-ignored-test 83; method-without-symbol 104; retired 13."
    },
    {
      "id": "repository-coverage",
      "path": "spec/, Rust source and tests",
      "role": "examined",
      "excerpt": "5785 untracked symbols; 6632 unmatched tags; binding census candidates 13528, tagged 12958, bound 12440."
    },
    {
      "id": "repository-reverse-ownership",
      "path": "2159 Rust files",
      "role": "examined",
      "excerpt": "Whole-repository source ownership remains unestablished; direct ownership inspection covered the changed behavior."
    },
    {
      "id": "ProviderLimit",
      "path": "fsl-analyze/src/provider.rs",
      "role": "examined",
      "excerpt": "MaxMemoryBytes,\n    /// `max_cpu_ms`, enforced by the process host.\n    MaxCpuMs,"
    },
    {
      "id": "ProcessFailed",
      "path": "fsl-analyze/src/provider.rs",
      "role": "examined",
      "excerpt": "ProcessFailed {\n        /// The observed process failure: `Crash` or `NonzeroExit`.\n        cause: ToolFailure,\n    },"
    },
    {
      "id": "finish",
      "path": "fsl-analyze/src/provider.rs",
      "role": "examined",
      "excerpt": "ProviderVerdict::ProcessFailed { cause } => process_failure(cause),"
    },
    {
      "id": "report",
      "path": "fsl-analyze/src/provider.rs",
      "role": "examined",
      "excerpt": "ProviderVerdict::ProcessFailed { cause } => return process_failure(cause),"
    },
    {
      "id": "process_memory_limits_settle_typed_failures_through_both_entries",
      "path": "fsl-analyze/tests/it/provider_external_contract.rs",
      "role": "examined",
      "excerpt": "#[trace(\"FR-287-AC-12\", \"FR-291-AC-1\")]\n#[test]\nfn process_memory_limits_settle_typed_failures_through_both_entries() {"
    },
    {
      "id": "process_cpu_limits_settle_typed_failures_through_both_entries",
      "path": "fsl-analyze/tests/it/provider_external_contract.rs",
      "role": "examined",
      "excerpt": "#[trace(\"FR-287-AC-12\", \"FR-291-AC-1\")]\n#[test]\nfn process_cpu_limits_settle_typed_failures_through_both_entries() {"
    },
    {
      "id": "process_deaths_without_limit_evidence_keep_only_the_observed_cause",
      "path": "fsl-analyze/tests/it/provider_external_contract.rs",
      "role": "examined",
      "excerpt": "#[trace(\"FR-291-AC-1\")]\n#[test]\nfn process_deaths_without_limit_evidence_keep_only_the_observed_cause() {"
    },
    {
      "id": "process_failure_verdicts_reject_nonprocess_and_mismatched_causes",
      "path": "fsl-analyze/tests/it/provider_external_contract.rs",
      "role": "examined",
      "excerpt": "#[trace(\"FR-291-AC-1\")]\n#[test]\nfn process_failure_verdicts_reject_nonprocess_and_mismatched_causes() {"
    },
    {
      "id": "FR-291",
      "path": "spec/functional/FR-291-bound-plugin-runs-and-settle-plugin-failures.md",
      "role": "context_only",
      "excerpt": "Only the two byte-limit variants carry a payload:"
    }
  ]
}
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Existing whole-repository traceability debt is outside the bounded process-provider prerequisite. |
| FND-002 | deferred | Existing extraction and unmatched-tag debt is outside the bounded prerequisite; raw diagnostics remain recorded. |
| FND-003 | deferred | Whole-repository ownership assurance remains unestablished; the owner requested focused delivery and no full gates. |

## Follow-up evidence

The separate code-review trace finding is fixed: both process-limit tests retain only their asserted FR-291-AC-1 criterion. Runtime behavior is unchanged. The repository-wide matrix snapshot above predates that trace-only correction and establishes no runtime pass.

