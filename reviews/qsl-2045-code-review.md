---
id: SR-2026111919340478491681409
title: "Code and Rust review of QSL-2045 probability corpus"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2045; PR 3097; tests/corpus/probability/**; tests/it/probability_source_routes.rs; additive tests/it/main.rs"
review_set: subset
---

## Summary

Reviewed frozen head 5f6f8e363241ba03781af139ed8e5b23c96d0962: original probability declarations and claim forms through public source compilation. No findings in this explicitly partial corpus slice.

## Verdict

**PASS for the changed slice.** QSL-2045 remains incomplete under QSL-2463 and QSL-2158. Staged sources are not counted as runnable analysis coverage.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Includes the Rust/style lane. Reviewed repository AGENTS/CLAUDE/CONTRIBUTING, applicable Rust implementation/testing/boundary conventions, the complete 85-file PR diff and the existing public compile/corpus seams needed to interpret its assertions. No production code, API, dependency, CI, unsafe code, custom runner or permissive decoder is changed. Existing same-repository Link model/snapshot fixtures are referenced, not copied. Temporary request/source files are isolated and cleaned by TempDir. Literal replacement inputs are checked before mutation. Each live assertion can fail on a real admission, refusal or emitted-identity defect. No findings.

Accepted coder-reported focused evidence without repeating it: twenty source corpus programs passed; seven accepting claim forms passed emitted node/occurrence/sidecar identity assertions; nine refusal variants passed in the original focused run, whose overall exit 101 was caused by separately staged S4 cases. Formatter/whitespace checks were reported passing. No reviewer tests, builds, gates or matrix repeated. The lead owns one final applicable gate. No full CI, exact probability result, statistical interval, replay, certificate-checker or whole-repository assurance claim is made.

No applicable AssuranceProfile in the established repository context. Spec review is inapplicable because the PR changes no specification or plan.

## Examined Scope

```json
{
  "clean": true,
  "scope": [
    {
      "id": "FR-185-AC-2",
      "path": "spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md",
      "role": "examined",
      "excerpt": "Test (TC-620) |"
    },
    {
      "id": "FR-185-AC-3",
      "path": "spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md",
      "role": "examined",
      "excerpt": "Test (TC-620, TC-2060) |"
    },
    {
      "id": "FR-185-AC-4",
      "path": "spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md",
      "role": "examined",
      "excerpt": "Test (TC-620) |"
    },
    {
      "id": "FR-185-AC-10",
      "path": "spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md",
      "role": "examined",
      "excerpt": "Test (TC-2060) |"
    },
    {
      "id": "FR-185-AC-11",
      "path": "spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md",
      "role": "examined",
      "excerpt": "Test (TC-2061) |"
    },
    {
      "id": "FR-186-AC-1",
      "path": "spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md",
      "role": "examined",
      "excerpt": "Test (TC-621) |"
    },
    {
      "id": "FR-186-AC-2",
      "path": "spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md",
      "role": "examined",
      "excerpt": "Test (TC-621) |"
    },
    {
      "id": "FR-186-AC-3",
      "path": "spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md",
      "role": "examined",
      "excerpt": "Test (TC-621, TC-2063) |"
    },
    {
      "id": "FR-186-AC-4",
      "path": "spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md",
      "role": "examined",
      "excerpt": "Test (TC-621) |"
    },
    {
      "id": "FR-186-AC-5",
      "path": "spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md",
      "role": "examined",
      "excerpt": "Test (TC-621, TC-2062) |"
    },
    {
      "id": "FR-186-AC-8",
      "path": "spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md",
      "role": "examined",
      "excerpt": "Test (TC-2166) |"
    },
    {
      "id": "FR-186-AC-11",
      "path": "spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md",
      "role": "examined",
      "excerpt": "Test (TC-2168) |"
    },
    {
      "id": "FR-498-AC-4",
      "path": "spec/functional/FR-498-carry-a-probabilistic-claim-s-checked-package-identity.md",
      "role": "examined",
      "excerpt": "Test (TC-1288) |"
    },
    {
      "id": "probability-corpus",
      "path": "tests/corpus/probability/",
      "role": "examined",
      "excerpt": "Twenty runnable compile programs and their requests/expectations; original claim templates; six staged intended-success cases explicitly owned by QSL-2463 and QSL-2158, excluded by the existing nonrecursive corpus runner."
    },
    {
      "id": "probability-source-tests",
      "path": "tests/it/probability_source_routes.rs",
      "role": "examined",
      "excerpt": "Seven source claim forms compile through the public command, emit exactly one Available claim node and matching sidecar declaration/occurrence; nine malformed claim variants assert precise refusal codes and causes. Temporary files are scoped to a TempDir."
    },
    {
      "id": "probability-registration",
      "path": "tests/it/main.rs",
      "role": "examined",
      "excerpt": "The probability_source_routes integration module is registered under the existing checked-handoff feature."
    }
  ],
  "findings": [],
  "bindings": []
}
```
