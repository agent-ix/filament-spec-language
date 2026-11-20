---
id: SR-2026111911313600000014
title: "gap-analysis of QSL-2041 operations source corpus"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3065; tests/it/source_corpus.rs; tests/corpus/operations/**"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-107
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: references
---

## Summary

Ticket QSL-2041, PR 3065, frozen reviewed head ac7f1be44a00b1b2e72bb4337ddd3fe0e58026d0. Diff-scoped requirement/test/source review found no gaps in the delivered behavior. The explicit assignment limits this analysis to the diff and accepts the lead's saved matrix; no repository matrix or inventory was repeated.

## Verdict

**PASS for this reviewed slice.** No whole-repository, whole-criterion, or full-ticket completion is claimed. QSL-2041 remains partial; the lead owns final gate and delivery state.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The changed harness connects the original operations programs to actual public compile/run commands. It requires a nonempty operations directory, validates emitted native-run-result/2 through the authoritative reader, and compares concrete result values and refusal outcomes. No stub, doubled unit under test, ignored new test, manufactured coverage claim, or new unowned production path was introduced. Same-repository model references avoid a second fixture owner.

The supplied trace tags identify owning FR-106/107 behavior and the exercised healthy/violating clause-run route of FR-312-AC-1. Compile/runtime cases cover state rule typing and guarded references, pre/post/result use, frame validation, snapshots and invocation identity/shape, bounded fields, function selection, populations, and request envelopes. This establishes exercised portions, not all graph, provenance, metering, or field obligations of compound criteria. Six pending codec cases are explicitly excluded and linked to existing owning bugs; they are not tagged as successful verification. Richer operation-parameter coverage remains on the ticket rather than being represented as delivered.

Reported evidence is 77 runnable cases passing (31 compile, 46 runtime) and formatting passing. It is accepted without rerunning tests or builds. The saved structural matrix remains historical context, not a claim this diff closes all untagged or ignored-only criteria. No additional plan was supplied or assessed. Spec review is inapplicable because no specification artifact changed. Rust review is folded into the companion code-review artifact. No applicable AssuranceProfile was selected in the lead's established context.

Plan completion: not assessed

## Examined Scope

```json
{"scope":[{"id":"execute_request","path":"tests/it/source_corpus.rs","role":"examined","excerpt":"fsl_replay::spine::read_native_run_result(&bytes).map_err(|error| {"},{"id":"corpus_operations","path":"tests/it/source_corpus.rs","role":"examined","excerpt":"fn corpus_operations() {\n    let directory = corpus_root().join(\"operations\");\n    let report = run_directory(&directory).unwrap_or_else(|error| panic!(\"{error}\"));\n    println!(\"{report}\");\n}"},{"id":"p-healthy","path":"tests/corpus/operations/p-healthy.expected.json","role":"examined","excerpt":"{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"boolean\",\"value\":true}}"},{"id":"pending-fs-seven","path":"tests/corpus/operations/pending/fs-seven.expected.json","role":"examined","excerpt":"\"flips_with\": \"QSL-2184\""},{"id":"FR-107","path":"spec/functional/FR-107-evaluate-state-clauses-at-s6a.md","role":"context_only","excerpt":"S6a SHALL evaluate one checked state clause (FR-104) over one\n`AdmittedObservations` value (FR-106) and SHALL return FR-090's\n`Evaluation`: `FamilyOutcome::Evaluated` carrying the kernel\n`Outcome<Boolean>` (`Completed(true)`, `Completed(false)`, `Undefined`,\n`Refused` or `Incomplete`), or `FamilyOutcome::FamilyEvaluated` for a\nfamily-owned result, beside the location of the node that stopped it."},{"id":"FR-312-AC-1","path":"spec/functional/FR-312-read-a-clause-run-request-and-run-it.md","role":"context_only","excerpt":"Through `run`, FR-108's healthy-parent case as a clause-run request (`ParentOrder` over its current snapshot) returns one `native-run-result/2` result document with `success`, `truth: true` and `basis`, exit 0; violating-parent returns `violation`, exit 10."}]}
```
