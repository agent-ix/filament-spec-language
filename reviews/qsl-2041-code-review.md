---
id: SR-2026111911313600000013
title: "code-review of QSL-2041 operations source corpus"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3065; tests/it/source_corpus.rs; tests/corpus/operations/**"
review_set: subset
---

## Summary

Ticket QSL-2041, PR 3065, frozen reviewed head ac7f1be44a00b1b2e72bb4337ddd3fe0e58026d0. Code review includes Rust review/style and test-oracle inspection. No defects found in this delivered slice.

## Verdict

**PASS for the reviewed diff.** The public compile/run routes execute original source programs with concrete success, violation, refusal, and incomplete expectations. The lead owns the final applicable gate. This is not a full-ticket or repository assurance completion claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed the changed harness and added operations corpus, including source/expectation pairs, request selections, snapshot and invocation inputs, and pending codec reproducers. The 77 runnable cases comprise 31 compile and 46 runtime requests; runtime includes intended refusals, not 46 successful evaluations. Concrete Boolean outcomes distinguish healthy, violating, absent-parent, self-loop, population-count/sum, enumeration, function-selection, pre/post, and frame cases. Boundary and malformed input cases check category, lifecycle stage, and named code/cause where supplied. The existing mismatch helper compares expected values and metadata; source/request pairing prevents a request silently executing another corpus program.

Native-run-result/2 values are passed through the published strict reader before their disposition is projected. The added disposition paths expose actual refusal codes/causes and completed values to the existing assertions. The reader is not replaced or relaxed. Compile requests use the public command entry point and the existing same-repository ConfigVersion model directly. No sibling source/schema/model copy, new framework, compatibility reader, production behavior change, CI edit, dependency change, unsafe code, or concurrent resource lifecycle is introduced. Test-only panic/reporting follows the existing harness convention.

The operations test now requires its directory and runs actual cases rather than silently skipping a missing area. Its trace tags identify FR-106/107 and the exercised public route in FR-312-AC-1; they do not establish every clause of those requirements. The six nested pending cases remain outside traversal and retain intended outcomes under QSL-2575 (snapshot/header refusal locus) and QSL-2184 (selection catalog fields). Their adjusted relative model/data paths preserve usable reproducers; invalid wire is not counted as passing coverage. Richer operation-parameter scenarios remain open on QSL-2041.

Reported focused evidence accepted without repetition: 77 runnable cases passed, formatting passed, log /home/peter/dev/.agent/qsl-2041-final-focused.log. No test, build, matrix, broad gate, or peer result was repeated. No applicable AssuranceProfile was selected in the lead's established context; no unrelated profile inventory was performed. Model runtime identifier is not exposed in this session; comment metadata records unavailable rather than inventing one.

Plan completion: not assessed

## Examined Scope

```json
{"scope":[{"id":"execute_request","path":"tests/it/source_corpus.rs","role":"examined","excerpt":"fsl_replay::spine::read_native_run_result(&bytes).map_err(|error| {"},{"id":"corpus_operations","path":"tests/it/source_corpus.rs","role":"examined","excerpt":"fn corpus_operations() {\n    let directory = corpus_root().join(\"operations\");\n    let report = run_directory(&directory).unwrap_or_else(|error| panic!(\"{error}\"));\n    println!(\"{report}\");\n}"},{"id":"p-healthy","path":"tests/corpus/operations/p-healthy.expected.json","role":"examined","excerpt":"{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"boolean\",\"value\":true}}"},{"id":"pending-fs-seven","path":"tests/corpus/operations/pending/fs-seven.expected.json","role":"examined","excerpt":"\"flips_with\": \"QSL-2184\""},{"id":"FR-107","path":"spec/functional/FR-107-evaluate-state-clauses-at-s6a.md","role":"context_only","excerpt":"S6a SHALL evaluate one checked state clause (FR-104) over one\n`AdmittedObservations` value (FR-106) and SHALL return FR-090's\n`Evaluation`: `FamilyOutcome::Evaluated` carrying the kernel\n`Outcome<Boolean>` (`Completed(true)`, `Completed(false)`, `Undefined`,\n`Refused` or `Incomplete`), or `FamilyOutcome::FamilyEvaluated` for a\nfamily-owned result, beside the location of the node that stopped it."},{"id":"FR-312-AC-1","path":"spec/functional/FR-312-read-a-clause-run-request-and-run-it.md","role":"context_only","excerpt":"Through `run`, FR-108's healthy-parent case as a clause-run request (`ParentOrder` over its current snapshot) returns one `native-run-result/2` result document with `success`, `truth: true` and `basis`, exit 0; violating-parent returns `violation`, exit 10."}]}
```
