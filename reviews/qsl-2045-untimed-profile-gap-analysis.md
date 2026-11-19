---
id: SR-2026111919340478491681412
title: "Diff-scoped gap analysis of QSL-2045 untimed-profile fixture"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2045; PR 3099; tests/corpus/probability/timed-profile-on-untimed-model.*"
review_set: subset
---

## Summary

Reviewed the three-file fixture diff at frozen head 3131fc49a0f3d6edcd654ad41f50ee2a2b9ba7f5 against FR-186-AC-13 and the existing corpus format. No findings.

## Verdict

**PASS for this fixture slice.** QSL-2045 remains partial under QSL-2463 and QSL-2158.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The new negative case covers the untimed-model refusal branch of FR-186-AC-13 through an actual compile request. It is discovered automatically by the existing probability corpus runner. Its source/request/expectation pairing is complete and the runner compares every populated expected field. The timed-model accepting branch, diagnostic text/location assertions and other probability behaviors are outside this small fixture slice; this review makes no full-criterion or whole-ticket coverage claim.

Accepted coder-reported single-case CLI validation without repeating it. No reviewer tests, builds, gates, matrix, baseline or wider review sweep ran. Rust and spec review are inapplicable to the changed fixture files. No applicable AssuranceProfile was identified in the established repository context. The lead owns the final changed-case CLI gate.

Plan completion: not assessed.

## Examined Scope

```json
{
  "clean": true,
  "scope": [
    {
      "id": "FR-186-AC-13",
      "path": "spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md",
      "role": "examined",
      "excerpt": "A claim under the timed profile over the untimed service model is refused `invalid_timed_model`/`model-time-on-untimed-model` naming `Service`, at the `using` alias, and not `timed-event-profile`; under the timed profile over `Retx`, `probability >= 0.99 [ eventually[0 ms, 4 ms] holds(c.late) ]` checks."
    },
    {
      "id": "timed-profile-refusal-fixture",
      "path": "tests/corpus/probability/timed-profile-on-untimed-model.native",
      "role": "examined",
      "excerpt": "Original probabilistic Delivered claim using the timed profile over the existing same-repository untimed Link model, with native-compile request and exact refusal expectation."
    }
  ],
  "findings": [],
  "bindings": []
}
```
