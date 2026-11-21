---
id: SR-202611210022442313144108526
title: Gap analysis of PR 3308 export admission
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; QSL-2667, QSL-2669, QSL-2670; fsl-command/src/command.rs, fsl-command/src/command/export.rs, fsl-command/src/command/export/evidence.rs, fsl-command/src/command/output.rs, fsl-command/src/command/wire.rs, fsl-command/tests/it/cli_contract/export.rs, fsl-command/tests/it/export_selection.rs, fsl-command/tests/it/main.rs, tests/it/plan_export_surface.rs
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-026
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1022
    type: references
---

## Summary

Frozen head: e615a0c4dad723a686254400f9a69bb8245c3fd2; base: 7bb12494. Primary ticket QSL-2667; batch order QSL-2667 → QSL-2669 → QSL-2670. Owner authorization is partial admission and one bounded export read. Genuine FR-1022 AC-1/2/3 and QSL-2667 AC-3 remain UNMET; package binding, proof naming, trace/config pairing and runner selection remain Peter's decisions. Both original ignored evidence tests retain their assertions. Driver argument conflicts and process acceptance are owned separately and are not claimed here.

## Verdict

**FAIL — not mergeable in the authorized partial scope.** Focused admission coverage misses a reproduced process abort and the catalog payload defect. Engine-backed repository assurance remains unavailable; this is not a full-repository PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Raw admission tests cover duplicate keys and an unknown envelope member but omit deep selected-member rejection; a 32,124-byte ordinary export request aborts instead of producing the FR-026 refusal. | fsl-command/tests/it/export_selection.rs:73 |
| FND-002 | medium | The pending-evidence oracle checks schema, code, cause and result but never the required capability payload; the closed lifecycle schema itself does not declare capability, so the passing oracle cannot establish catalog conformance. | fsl-command/tests/it/cli_contract/export.rs:107 |

## Evidence and required changes

FND-001: Existing raw-admission assertions have no nested selected-member oracle. The focused reproduction establishes an observable failure, not merely a missing trace tag. Add a subprocess oracle for typed rejection of deeply nested ordinary export selections and verify iterative cleanup remains effective after values leave the root tree.

FND-002: The lifecycle schema check at export.rs:107 is real and must stay. It passes because the current diagnostic schema has no capability field or conditional requirement for unimplemented-meaning. Add the required typed payload and a direct catalog assertion; admitting a genuine assembled package only demonstrates shape admission and cannot discharge FR-1022 genuine-evidence criteria.

## Coverage

The entry has the agreed public ExportSelection traits and function signature. with_request_limits performs one bounded file read and passes the same admitted bytes and original operand parent into selection execution. Required-member checks precede source, artifact and output I/O. Generic run shares the admission helper; its unrelated fallback reread is pre-existing and has not been broadened. Existing ordinary-export compile conversion remains unchanged. Duplicate detection reuses unique_json::read / quire-canonical rather than a second parser; numeric lexemes are decoded from original scalar text. Closed top-level envelope checks precede required selection. Evidence validates its four fields, closed member set, tool set and selection conflicts without reading artifacts. Output override, dependent limits, SpineLimits, target path and no-overwrite ownership remain in the ordinary route. No dependencies, pins, suppression, CI lanes or assertion bodies were changed. No applicable AssuranceProfile was found. New capability_document is a thin value constructor in the existing NativeResult owner; no separate protocol or lifecycle framework was added.

Inspected tmp/export-unit-final.log: 13 passed, 0 failed, 89 filtered; tmp/export-integration-final.log: 17 passed, 0 failed, 143 filtered. Inspected final scoped Clippy log: finished successfully; initial dependency lint stopped with four unchanged fsl-replay diagnostics (baseline, not suppressed). Final formatting log is empty; independently reran scoped rustfmt --check (edition 2021, skip_children=true), exit 0, and git diff --check, exit 0. No build, dependency, whole-crate, workspace, full qualification or aggregate gate was run by this reviewer. Existing built integration child used for the new resource-risk check: 32,124-byte JSON with 16,000 nested arrays under export.z gives SIGABRT/stack overflow; same nesting retained in the Deep-owned request with no required selection gives exit 20. Receipts: deep-repro.log and deep-control.log. The first exploratory pipe invocation could not open /dev/stdin (ENXIO) and established no behavior; the final reproduction uses a regular-file stdin descriptor and reaches the library through the existing child harness.

Plan completion: not assessed

Reconciliation: engine unavailable for this review. quire matrix and quire coverage expose no criterion filter. Each was attempted once with a 20-second bound and stopped without JSON, leaving only module-discovery diagnostics. Coverage exit 124 was captured. The matrix wrapper omitted its returned running-session metadata, so its final exit was not captured; its process ended at the timeout and produced no matrix. No repeat was made. No numeric coverage rollup, stale-tag result, engine-backed PASS or grep fallback is asserted. CLI quoin 0.29.1 is available; its matrix delegates to the same whole-tree extraction, so no repeated expensive attempt was made. These limitations prevent completing the whole-repository matrix, stale-tag and whole-source audit in this bounded review.

Manual scoped source inventory: four reviewed behaviors (required ordinary/evidence selection, duplicate/closed raw admission, existing ordinary export/output route, explicit unsupported evidence shape admission) map to FR-026/FR-1015/FR-1022. No new unowned behavior found in those four; repository-wide untraced behavior counts are not established. Source/test stub inspection covered all nine changed files and nearby route owners; the explicit unsupported settlement is owner-authorized partial delivery, not false completion. No source stub is misrepresented as completed FR-1022 acceptance. Optional whole-repository semantic review was not run; requested scoped requirement/test/source comparisons were performed.

Trace bindings inspected: the two retained ignored tests bind FR-1022-AC-1 and FR-1022-AC-3 and remain pending; new shape-admission tests bind broad FR-1022 only and do not claim those ACs or TC-2872. AC-2 remains unmet. No computed status counts were available.

All nine changed files examined:

- fsl-command/src/command.rs
- fsl-command/src/command/export.rs
- fsl-command/src/command/export/evidence.rs
- fsl-command/src/command/output.rs
- fsl-command/src/command/wire.rs
- fsl-command/tests/it/cli_contract/export.rs
- fsl-command/tests/it/export_selection.rs
- fsl-command/tests/it/main.rs
- tests/it/plan_export_surface.rs

Examined and contextual requirement units (roles and excerpts apply to this artifact and its tracker comment):

```yaml
scope:
  - id: FR-026
    path: spec/functional/FR-026-run-standalone-native-workflow.md
    role: examined
    excerpt: The command shall reject unknown/duplicate fields and positional record arrays.
  - id: FR-026-AC-3
    path: spec/functional/FR-026-run-standalone-native-workflow.md
    role: examined
    excerpt: Through `command` in process, a `1-draft` run request with a malformed request, an unknown format and an unavailable file each refuse at their actual stage with their catalogued code and the available source details, with no result document and exit code 20; each command-error envelope conforms to the native result schema.
  - id: FR-026-AC-4
    path: spec/functional/FR-026-run-standalone-native-workflow.md
    role: examined
    excerpt: Through `command` in process, a `1-draft` run request one byte over `request_bytes`, and dependent files one byte over `dependent_bytes`, each stop at intake naming the setting and its configured value; the same requests with each limit raised to their size run; a fresh default request then succeeds.
  - id: FR-026-AC-5
    path: spec/functional/FR-026-run-standalone-native-workflow.md
    role: examined
    excerpt: '`command` given one `1-draft` request file by its path from two different process working directories returns equal outcomes, the request''s relative, absolute and parent-relative file paths resolving against the request''s directory and as written; no call writes a byte to stdout or stderr.'
  - id: FR-1015
    path: spec/functional/FR-1015-request-an-export-and-write-the-export-outcome.md
    role: examined
    excerpt: FSL SHALL read a `native-run/1` request's `export` member as FSpec FR-817's export request and write FSpec FR-817's `filament.export-outcome/v1` document as the run outcome's `export` member, applying FSpec FR-811's fidelity floor and the `export.*` limits of FR-255, with FSpec FR-817's exits.
  - id: FR-1022
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: examined
    excerpt: 'The `export_evidence` member SHALL carry `package` (the `package` member of an assembled export''s outcome, FSpec FR-817, which with that export''s records is its FSpec FR-299 package key), `obligation`, `tool` (`tlc`, `apalache`, `tlaps` or `lean`) and `artifact`: for `lean` and `tlaps` the proof file, for `tlc` and `apalache` the model-checker configuration, and for a counterexample the tool''s trace file.'
  - id: FR-1022-AC-1
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: examined
    excerpt: A Lean proof of `Ob_Low` accepted by the Lean child process settles `proved`, `trusted`, extent `unbounded`, never `certified`; with `lean` absent the intake settles `unsupported`, `tool-unavailable`, exit `21`.
  - id: FR-1022-AC-2
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: examined
    excerpt: A TLC trace of a seeded violation reads back through the name map and settles `refuted` after native replay; the same trace edited so it no longer replays settles nothing.
  - id: FR-1022-AC-3
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: examined
    excerpt: Evidence naming an `unrepresented` record, and evidence whose `package` key names an export of a changed model, each refuse `invalid_runtime_input`/`invalid-value` at the member.
  - id: FSpec-FR-272
    path: spec/objects/foundation/FR-272-typed-diagnostic-cause.md (filament-specification@247a394)
    role: examined
    excerpt: 'For `unimplemented-meaning`, `capability`: the name of the valid complete-V1 meaning the selected producer does not implement. Each is category unsupported.'
  - id: FSpec-FR-817
    path: spec/functional/output-mapping/FR-817-request-and-write-a-tla-or-lean-export.md (filament-specification@247a394)
    role: context_only
    excerpt: FR-299's package key is this member together with the outcome's `records` in order.
  - id: FR-286
    path: spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md
    role: examined
    excerpt: A driver failure before any operation runs shall be one `OutcomeDocument::new(operation, None, category)` with one diagnostic.
```
