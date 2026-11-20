---
id: SR-2026112017124277723988806
title: "Gap analysis of QSL-2237 in FSL PR 3127"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2237, PR 3127 at e39e97365f8ce2016587f3170fc30ed8fe44d7f9; spec/functional/FR-752-generate-model-based-tests-and-shrink-counterexamples.md; tests and production code in the PR diff"
review_set: subset
---

## Summary

Ticket: QSL-2237. Planless gap check of FR-752-AC-1..11 against `quoin matrix --json` (quoin 0.36.8, read-only, no build) and a manual reading of the tests the PR adds or changes. Plan completion: not assessed. Every FR-752 criterion has at least one binder. No run evidence is bound ("no evidence is bound"), because no test results were supplied.

## Verdict

Every criterion is bound. AC-4 includes three ignored stub binders (see FND-002 in the code-review artifact). The request-surface binders for AC-3 and AC-5 are partial (FND-001 and FND-007 there). One new gap below, beyond the code-review artifact. The stopped-state findings change in fsl-eval has no dedicated test of its own; it is exercised only through the request-surface refuted-initial-state tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The fsl-eval change that records findings at a stopped final state (sample, rebuild_model_trace, replay) has no focused test showing that a stopped trace's last-state finding is recorded and then matched on replay. The coverage is indirect, through the request-surface state-clause tests. | fsl-eval/src/simulation/sample.rs:148-162, fsl-eval/src/simulation/trace.rs:172-181, fsl-eval/src/simulation/model.rs:2471-2486 |
