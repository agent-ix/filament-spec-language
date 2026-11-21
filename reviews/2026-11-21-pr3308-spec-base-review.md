---
id: SR-202611210022442313144108527
title: Base specification review of PR 3308 capability diagnostic details
type: SpecReview
analysis: base
scope: agent-ix/filament-spec-language; QSL-2667, QSL-2669, QSL-2670; FR-286 Diagnostic details addition and fsl-replay/schemas/filament-outcome-1.schema.json
review_set: base
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
---

## Summary

The owner-selected base review covers only the four-line FR-286 change and its matching schema member at frozen 3a4874b4b4c2b80cc7cf3c01b7c4e53df549c44f. It applies the existing DiagnosticDetails capability encoding already emitted by ordinary export to pending evidence; it introduces no evidence protocol, top-level diagnostic member, new type, package claim or proof execution. No applicable AssuranceProfile requires an enforced workflow; direct base review is appropriate.

## Verdict

**PASS for the scoped base specification review.** The approved FR-286 rule, closed vocabulary and schema string property are consistent; no new specification defect found. This does not claim repository-wide coverage or genuine FR-1022 acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The owner-selected base review covers only the four-line FR-286 change and its matching schema member at frozen 3a4874b4b4c2b80cc7cf3c01b7c4e53df549c44f. It applies the existing DiagnosticDetails capability encoding already emitted by ordinary export to pending evidence; it introduces no evidence protocol, top-level diagnostic member, new type, package claim or proof execution. No applicable AssuranceProfile requires an enforced workflow; direct base review is appropriate.

The new SHALL rule fixes when capability is required and exactly which value C means, and states equality with an export refusal when present. The adjacent closed member table gives capability string kind and the same FSpec FR-272 meaning. The JSON schema adds precisely that string property while retaining minProperties 1, null alternative and unevaluatedProperties false. The schema is a structural validator; the source and exact-C tests enforce the code/cause-specific requirement. Unchanged members retain their previous types. Normative and schema vocabularies agree.

IDs, existing relationships and acceptance-criterion identities are unchanged; no new duplicate artifact identity, dependency edge, hash/pin requirement or tracking ceremony is introduced. The FR-286→FSpec FR-272 catalog reference resolves in the existing pinned specification. The six coverage-rule checks are scoped to the change: both ordinary export families and pending evidence have exact-C positive oracles, unknown-details negative oracles, and pending evidence has a wrong-kind oracle. Closed/empty/null distinctions remain represented by the existing details schema and FR-286-AC-19. This stateless payload addition has no new transition or resource boundary. Existing FR-286-AC-31 provides schema/writer consistency context. No new AC or tag is claimed; broad FR-286 test tags establish only the inspected source association. Full computed matrix, full golden corpus completeness and repository-wide criterion coverage are not established in this bounded review. No source/test edits or full gates were performed by this reviewer.

Inspected tmp/export-fix1-unit.log: 14 passed, 0 failed, 91 filtered. Inspected tmp/export-fix1-integration.log: 18 passed, 0 failed, 157 filtered, including six deep-input subprocess cases. Scoped Clippy --lib --test it --no-deps finished successfully. Broader --lib --tests Clippy stops on the unchanged main/#3302 handoff.rs:593 result_large_err baseline; no suppression was added. Fix-file scoped rustfmt and git diff --check pass (reviewer verified exit 0). A broader formatting sweep including tests/it/main.rs detects the inherited dependency_handoff module ordering already present verbatim in main 7d9dd0ee; the PR only adds export_selection in its correct position. The empty coder formatting log alone is not treated as proof of broader baseline cleanliness. Focused FR-286 quire validation exits 0, with only unchanged module discovery and pre-existing EARS warnings at line 331. No new test build, aggregate gate, workspace tests or expensive matrix/coverage rerun was performed by the reviewer.

Genuine FR-1022 AC1–3 and QSL-2667 AC3 remain UNMET by authorized deferral. The two original evidence acceptance tests remain ignored with unchanged assertions. Driver process and argv-conflict cases remain separately owned. Prior matrix/coverage extraction was unavailable; no engine-backed repository assurance PASS is claimed. No new code or test defect was found in this disposition pass.

Examined and contextual units:

```yaml
scope:
  - id: FR-286
    path: spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md
    role: examined
    excerpt: 'An `unimplemented_capability` diagnostic with cause `unimplemented-meaning` SHALL carry `{"capability": C}`, where C is the capability name required by FSpec FR-272, matching the export refusal when one is present.'
  - id: FR-286
    path: spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md
    role: examined
    excerpt: 'The object''s members come from one closed vocabulary, each with one meaning and one JSON kind, and no other member is written:'
  - id: FR-286-AC-19
    path: spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md
    role: examined
    excerpt: 'The `execute` outcome document of a run whose unit S3 refuses `ill_typed` (`function bad using v(): Int[0, 9] pure { true }`) and the `parse` outcome document of FR-763-AC-2''s unit each hold diagnostics whose `details` is `null`; no diagnostic of any document holds a `details` member outside the closed vocabulary or an empty `details` object.'
  - id: FR-286-AC-31
    path: spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md
    role: examined
    excerpt: A Value-built writer that emits a key outside its `MEMBERS` fails its `MEMBERS` test; the corpus coverage test run against a copy declaring a diagnostic member `unused` fails naming `/diagnostics/*/unused`.
  - id: FSpec-FR-272
    path: spec/objects/foundation/FR-272-typed-diagnostic-cause.md (filament-specification@247a394)
    role: context_only
    excerpt: 'For `unimplemented-meaning`, `capability`: the name of the valid complete-V1 meaning the selected producer does not implement. Each is category unsupported.'
  - id: FR-1022-AC-1
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: context_only
    excerpt: A Lean proof of `Ob_Low` accepted by the Lean child process settles `proved`, `trusted`, extent `unbounded`, never `certified`; with `lean` absent the intake settles `unsupported`, `tool-unavailable`, exit `21`.
```
