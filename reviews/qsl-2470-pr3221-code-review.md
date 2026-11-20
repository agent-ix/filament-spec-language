---
id: SR-2026112021460200000000001
title: "code-review of PR 3221 L3 CLI contract tests (QSL-2470)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3221 head 4a142af165a5fc7caa3546780298b25972748624; fsl-replay/schemas/filament-outcome-1.schema.json, fsl-command/tests/it/cli_contract/*.rs, fsl-command/tests/it/main.rs, fsl-command/tests/fixtures/cli-contract/counter.semantic-ir.json"
review_set: subset
---

## Summary

Tickets: QSL-2470, QSL-2471, QSL-2472, QSL-2473, QSL-2474, QSL-2477, QSL-2478. PR filament-spec-language#3221. This is a diff-only code review with the rust-review lane folded in. Nothing was built or run. Quoin gap-analysis was not run. The one production change is a schema edit, so a manual check of each test's cited ACs (FR-621, FR-100, FR-501, FR-286, FR-285, FR-640, FR-644, FR-810, FR-869, FR-1015, FR-255, FR-661) was done in its place.

The product change is to the schema. Before it, every item had to carry `result`. Now an `if`/`then` requires `result` on the items of every non-`analyze` document. For `analyze` items this matches FR-286's "Published schema" table: they carry FR-621's members, and the result label sits inside `record`. The top-level `operation` is required, so the condition always applies, and no non-`analyze` document gets weaker. The problem is that the condition also covers `execute` documents. FR-963 requirement items are written in FR-621's spelling, so they have no top-level `result` (FND-001).

The tests drive each refusal class with an input that produces it, and they assert the exit code and the document's category, operation and format. Every outcome is validated against the published schema, except analyze refusals (FND-002). None of the expected exit codes or causes contradicts the FR it cites:

- FR-621-AC-10: an unknown claim gives a `disposition` record, and refusal exit 20.
- FR-100-AC-5: `nope` gives `missing_declaration` with `details.function`.
- FR-501-AC-4: an absent valuation is indeterminate, exit 11.
- FR-285-AC-2: {10, 22} gives 22.
- FR-644-AC-4: a refusal before the stream writes no event.

Several assertions are weaker than the AC they cite (FND-004). A dead closure would trip the clippy `-D warnings` gate (FND-003).

## Verdict

NOT MERGEABLE. One high (FND-001), three mediums (FND-002 to FND-004) and one low (FND-005).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new schema conditional requires `items[].result` on every non-analyze document, including `execute` documents whose FR-963 requirement items FR-286 says are in FR-621's item spelling (no top-level `result`). Valid execute documents still fail the published schema. | fsl-replay/schemas/filament-outcome-1.schema.json:20 |
| FND-002 | medium | The harness skips schema validation for every `analyze` document without `aggregate`, which is every whole-request analyze refusal. The suite hides the fact that these documents break FR-286's published schema ("an analyze document without aggregate fails") instead of failing on it. | fsl-command/tests/it/cli_contract/mod.rs:118-124 |
| FND-003 | medium | An unused closure statement (a leftover `SlowEnough` request builder) is dead code. rustc warns `unused closure that must be used`, so `cargo clippy --all-targets -- -D warnings` fails. | fsl-command/tests/it/cli_contract/machine_output.rs:179-186 |
| FND-004 | medium | Several tests assert less than the AC they cite: only the exit code, or a bare `contains`, where the AC names the code, member or reason. | fsl-command/tests/it/cli_contract/export.rs:104-118, export.rs:138-161, export.rs:166-177, run.rs:123-135, monitor.rs:119-146, monitor.rs:186-196, analyze.rs:139-158 |
| FND-005 | low | Some trace tags cite ACs the test does not exercise: FR-312 for request byte ceilings, FR-661-AC-1/AC-3 for `version` option parsing, FR-286-AC-19 and FR-286-AC-18 without the `details` they state. | fsl-command/tests/it/cli_contract/analyze.rs:176,189; run.rs:157,192; surface.rs:118 |

### FND-001

FR-286 "Published schema" table, `OutcomeDocument` row: `registry` "on an `execute` document of a call carrying `requirements`, whose `items` are FR-963's settled requirement items in FR-621's item spelling". `OutcomeDocument::with_requirement_items` (fsl-replay/src/outcome.rs:2493) writes those FR-621 `serde_json::Value` entries as `items`. They carry `request_index`, `claim_name`, `request`, `category` and `record`, with the result label at `record.result`. FR-286-AC-31 requires the golden `execute` document of FR-963-AC-1 to validate. Fix: widen the exemption to `"not": {"enum": ["analyze", "execute"]}` (or require `result` only for the `OutcomeItem` operations `prove` and `monitor`), and validate an FR-963-AC-1 execute document in the suite.

### FND-002

`RunError::outcome_document(Operation::Analyze)` returns `AnalyzeRefused(refusal).document()` with no `aggregate` or `registry`. FR-621 "Document members" says the analyze document carries `aggregate` and `registry`. FR-286 says an `analyze` document without `aggregate` fails the schema. The harness comment calls this an open spec question. Fix: make the analyze refusal writer emit FR-621's `aggregate` (status `incomplete`, no gaps) and `registry` (or settle the FR-621/FR-286 question in this PR), then delete the exemption. If the question stays open, assert the schema failure explicitly so it cannot pass unnoticed.

### FND-003

`|limits: Option<Value>| { ... };` sits between `let request = ...` and the `for` loop and is never called. Fix: delete it.

### FND-004

- export.rs:104: `--output` beside `export.output` asserts only exit 20, where the FR-810-AC-3 rule (stated for compile) refuses `invalid-request` naming `output` and `--output`.
- export.rs:138: the override run does not check that the document names `export.output_bytes`. The member-plus-override run asserts only exit 20, where FR-1015-AC-2 says `invalid-request` naming `limits` and `export.output_bytes`.
- export.rs:166: `contains("invalid-request")` stands in for the diagnostic code and the `output_bytes` member (FR-1015-AC-2).
- run.rs:123: the indeterminate run is checked with `contains("indeterminate")`. FR-285-AC-7 and FR-501-AC-4 also require the item's label and the `MissingValuation` reason.
- monitor.rs:119: FR-644-AC-2's claim that the good records' events and summary equal those of the stream without the bad record is not asserted.
- monitor.rs:186: either a refusal document or a summary passes as long as the exit is 22. FR-644 specifies a summary carrying `cancelled`.
- analyze.rs:139: FR-621-AC-10's "no `backend` member" on the disposition record is not asserted.

Fix: assert the AC's named code, member and reason (`refused(..)` plus pointer/member checks), and require `Stream::Lines` with `summary.cancelled` for the cancelled stream.

### FND-005

FR-312 is the clause-run request reader and says nothing about `request_bytes`. FR-869-AC-1 (compile) and FR-026 own the request and dependent byte ceilings. FR-661-AC-1/AC-3 are about `build_inventory` content and determinism. Fix: retag with the owning ACs, or drop the tag.
