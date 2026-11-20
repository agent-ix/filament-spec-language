---
id: SR-1468
title: "Scoped acceptance gap analysis of QSL-2076 grammar near-misses"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language PR3146 at 406d58317fed97f48e981e95e728866cc4ca189f; fsl-cst/tests/it/{time_declaration,hybrid_declaration,probabilistic,synthesis_verify,main}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

Compared the changed tests with QSL-2076's seven grammar families and the current parse-only grammar contract. The seven functions supply 29 nonlexical negative inputs and 12 accepted declarations with code, cause, exact token/EOF byte spans, expected messages and lossless rendering assertions.

## Verdict

**PASS** — no missing acceptance branch or hollow assertion was found in the assigned diff. This is a PR-scoped conclusion, not a repository-wide assurance claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- Time declarations: dense unit, tick period/quantity and terminator negatives with dense/tick accepted controls.
- Time invariants and clock bounds: missing comparison, forbidden `or`, empty quantification and missing `in`, with combined ordinary/quantified valid control.
- Urgency: missing target, trailing word, missing guard braces and EOF terminator, with operation/guard accepted controls.
- Hybrid: empty equation list, missing equals, empty initial expression, duplicate initial block and incorrect member order, with an initial/two-mode control.
- Delay: unknown distribution, wrong separator, absent weight and empty list, with four admitted distribution forms.
- Synthesis/verify: missing mandatory members, ordering and terminators, with valid synthesis and dependent verification steps.
- Each new test carries `FSpec-FR-339-AC-1`; the helpers invoke the public real parser and assert non-admission plus located typed diagnostics. No `#` replacement, mock, skip or conditional assertion masks the intended path.
- Changed-code reverse-gap/stub inspection found only domain-specific test data and assertions; no new production behavior requires a separate specification.
- Repository matrix and existing corpus measurements were deliberately not repeated under the user's diff-only review and peer-evidence instructions. No repository completeness claim is made.
- Plan completion: not assessed
- Semantic agreement was assessed over the changed cases only; no optional repository-wide semantic audit ran.
