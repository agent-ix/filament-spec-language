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

**PASS** — accepted the author’s implementation receipt at 818adf75bda50dbd049177ad524869196be0dbc8 for the QSL-2686 harness correction. False generic negative coverage and its criterion binding are removed; real-source positive/boundary checks and explicit grammar-aware located negatives remain active. This is a PR-scoped conclusion, not a full-component or repository-wide assurance claim.

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

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The legacy production-vector test mutates a word suffix but requires a diagnostic at that suffix even when the earlier remaining prefix already violates the exact-token grammar. Replace the whole final token with padded standalone `#` while keeping the strict boundary assertion. | fsl-cst/tests/it/complete_grammar.rs:587; SR-1467 FND-002 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | Source-grounded vector correction accepted by the lead; implementation and one changed-function check remain. Missing runtime diagnostic values were not invented. |
| FND-002 | still-open | Round 3: the one changed-function run refutes the proposed generic marker repair. Remove the generic negative generator under QSL-2686, retain active positive/boundary assertions, and represent missing grammar-aware negative coverage through existing owners. No incorrect location oracle becomes a strict-pending parser bug. |
| FND-002 | fixed | Accepted the author's in-lane implementation receipt at 818adf75bda50dbd049177ad524869196be0dbc8: both generic mutation loops and mutation-only bookkeeping removed; active function renamed for positive/exact-boundary assertions; negative-production criterion tag removed; explicit located negative fixtures unchanged. The only changed qualified function passed 1/0. |

## Final negative-coverage disposition (pass 3)

The supplied CaseArm `r#` failure is compatible with the actual ReferenceLiteral prefix grammar; it is not evidence of a parser-location defect. No generic marker substitution proves an invalid token at the intended production boundary across arbitrary surrounding grammar. The proposed padded-token repair is withdrawn. Remove the generator and its universal negative-coverage claim, while retaining all active positive production inventory, node boundaries/rendering, formatter properties and the real ReferenceLiteral vector. Existing explicit near-miss/code/cause/span tests remain strict; any still-missing production negatives are owned by the existing QSL-2075..2078/2107 program and must not be claimed as supplied by the positive inventory. QSL-2686 is the one actual harness defect, related to QSL-2107 and blocking QSL-2076 until the same-PR correction lands. No whole-positive-test ignore, speculative parser bug, another marker experiment or additional runtime was performed or recommended.

## Confined correction coverage

The four grounded stale-test corrections and tracked strict-owner annotations were inspected only in the supplied correction diff at 6540b9901112b0050ad51bf8b0914753e6cf1260. Four reported corrected passes are accepted; the reverted history-oracle pass is excluded. The ReferenceLiteral inventory and authored vector are grounded. Original required oracles under QSL-2679 and QSL-2680 through QSL-2685 remain pending, not passing coverage. The original gate failure remains visible; no test, build, baseline probe, owner audit or repository matrix was run for this disposition.

## Final record-only disposition (pass 4)

The retained active function still checks the independent production inventory, actual positive parsing, node rendering and exact byte spans, formatter significant-token/tree identity and idempotence, including the real ReferenceLiteral vector. No false generic negative-coverage claim remains. The original failed full gate and all attempts remain public; eight strict functions under QSL-2679 through QSL-2685 stay excluded from coverage. The seven new feature functions and four grounded corrections remain accepted. This disposition uses the author's receipt without another code read, review sweep, test, build or runtime probe. QSL-2686 can be closed when the same-PR correction merges.
