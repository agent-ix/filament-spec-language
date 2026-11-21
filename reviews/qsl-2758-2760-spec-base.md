---
id: SR-1549
title: "Scoped base specification review of the canonical command/resource additions and the one new FR-753 selector sentence found no specification defect."
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR3404; QSL-2758/QSL-2760; frozen 0d77f99a3895a12015850da6837e991e76c4a013; command/run, replay reference/certificate helpers, model owners, scoped FR-650/FR-753 additions"
review_set: subset
---

## Summary

Scoped base specification review of the canonical command/resource additions and the one new FR-753 selector sentence found no specification defect.

Frozen source head: `0d77f99a3895a12015850da6837e991e76c4a013`; PR: agent-ix/filament-spec-language#3404; integrated review base: `21edb2ead1c8b69768beedaab003e077ce4ed042`. Tickets: QSL-2758 and QSL-2760. The review is diff-scoped; accepted prior specification/reference work was not repeated. No production code or frozen-branch file was changed.

## Verdict

**PASS** — No findings in this scoped method.

The new FR-650 AC-18 through AC-23 state observable canonical values, strict legacy refusal, cumulative supplied admission, normalization-owner denial, genuine package reference reuse and the zero-model-work boundary. The native grammar is explicitly retained for separately exposed internal/spine/clause readers. The FR-753 sentence assigns the existing request selector/options to every negotiated step while the surrounding contract retains caller negotiation and prohibits choosing an engine. These are bounded feature obligations, not a new compatibility layer or tracking framework. Existing prior spec/reference reviews are accepted; no full-spec audit or enforced-run claim is made. The live defects identified by SR-1547 are implementation failures against retained obligations, not reasons to weaken the requirements.

## Assurance Context

Repository AGENTS.md/CLAUDE.md and code-review/Rust review/style rules govern this pass. No applicable AssuranceProfile was identified by the scoped filename lookup; no profile/enforced-run conformance is claimed. The explicit no-rerun, bounded-scope, unchanged outside-run behavior and admission-before-allocation constraints from the dispatch apply.

## Examined Scope

```yaml
{
  "scope": [
    {
      "id": "FR-650-AC-18",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "Through the public run command, canonical Integer, Boolean, Text, Sequence and Set arguments complete with canonical values; a 23-digit integer remains exact, `4/6` completes as `2/3`, an empty sequence and one at its maximum retain order, and a set supplied as 3 then 1 completes as 1 then 3. A call with no arguments also writes its result canonically."
    },
    {
      "id": "FR-650-AC-19",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "The public run command refuses an untagged integer, a bare integer, a tagged integer with noncanonical decimal spelling, an unknown member, and an out-of-domain integer, each with exit 20. Request and dependent-byte exhaustion remain incomplete with no completed value."
    },
    {
      "id": "FR-650-AC-20",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "Genuine canonical argument membership and execution each fit a supplied admission-work limit independently, but their combined work on the same supplied run exceeds it and returns the typed supplied-value limit refusal. Reference codec conversion retains its separately cumulative conversion-work counter on that same run."
    },
    {
      "id": "FR-650-AC-21",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "A genuine checked-call reference catalog exhausts its original normalization owner's remaining work allowance and returns `NormalizationIncomplete` at `reference.catalog`, retaining the configured limit, prior consumption and denied charge."
    },
    {
      "id": "FR-650-AC-22",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "A checked-package call over a genuine admitted snapshot accepts a canonical reference and returns that same canonical reference; feeding the result into another package call completes unchanged after the original program file is removed. The catalog and writer use the same snapshot admission's retained normalization owners."
    },
    {
      "id": "FR-650-AC-23",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "A call with no models and no snapshot completes with its canonical value when `model.work_units` is zero; canonical output does not introduce model work into that value-only call."
    },
    {
      "id": "FR-753",
      "path": "spec/functional/FR-753-build-check-emit-and-execute-a-verification-plan.md",
      "role": "examined",
      "excerpt": "A run request's existing `requirements.backend` selector and its options SHALL apply to that caller negotiation for every negotiated plan step."
    }
  ]
}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence and Limits

Accepted evidence: 27 focused named passes reported by the author, with the focused-receipts document and original logs retained beside the PR body. Actual selected-plan and package-reference pass receipts were read, as was the shallow canonical-values pass receipt. The original borrowed-writer compile failure remains retained and is not counted as passing. No Cargo, build, test, lint, full gate, whole-repository matrix or peer-check rerun occurred. Root owns the final affected-crate plus fsl-analyze/filament-spec-language check and exact-head integration. The known reused held-claim failure remains owned by QSL-2744 / PR3403; it is not duplicated here. QSL-2760 remains open for real filament-driver adoption.
