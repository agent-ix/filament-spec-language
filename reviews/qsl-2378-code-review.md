---
id: SR-202611210022442313144108528
title: "QSL-2378 process provider verdict code review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/provider.rs, fsl-analyze/tests/it/provider_external_contract.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-287", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-291", type: references}
---

## Summary

Independent static review of FSL PR 3313's process limit verdicts and four integration tests. The reviewer did not author either changed file. One incorrect criterion binding was found; focused runtime verification remains pending.

## Verdict

CONDITIONAL: correct the two inaccurate criterion bindings. Merge also remains held for the four requested focused tests and the repository's direct owner authorization rule. This is not a whole-repository assurance signoff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The memory and CPU tests bind to FR-287-AC-12 without asserting its frame, deadline or invalid-option behavior; retain the real FR-291-AC-1 binding. | fsl-analyze/tests/it/provider_external_contract.rs:510 |

## Coverage

Reviewed the full two-file diff, callers of both settlement entries, closed verdict matching, existing failure serialization, FR-287 process budgets and FR-291's fourteen-defect vocabulary. Both the target-bearing and target-free settlement branches preserve the observed typed process cause. A mismatched limit/cause and a nonprocess cause on ProcessFailed remain internal faults. No unobserved byte or CPU measurements are added to the wire.

The added tests invoke the actual analyze and provider entry points through the existing external-provider seam. They assert exact terminal causes, provider identity, absent counterexamples, serialized cause members and exit 30; invalid cases assert rejection. No added code is vendored, unsafe, a placeholder or a replaced unit under test. No gate is weakened. No applicable AssuranceProfile was found under spec.

The author ran touched-file formatting and diff checks. Runtime request sent to draco:QSL:code: `cargo test --locked -j 4 -p fsl-analyze --test it -- provider_external_contract::process_ --test-threads=1`, expected four tests, with the owner's fixed debug/incremental environment. No runtime pass is claimed here. Full gates were not run, as instructed.

The separate gap-analysis artifact records whole-repository traceability debt and extraction limitations. Those existing findings are not introduced by this prerequisite.

## Reviewed units

```yaml
{
  "scope": [
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
| FND-001 | fixed | Both memory and CPU tests now retain only `#[trace("FR-291-AC-1")]`; their bodies are unchanged. |

## Final review state

PASS after the trace-only correction. Independently inspected the entire correction. The focused `cargo check --locked -j 4 -p fsl-analyze --test it` passed through the shared queue with the required environment. The four focused runtime tests remain queued with Draco; no runtime pass is claimed.

