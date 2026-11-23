---
id: SR-967175026302197768900474181941935084789
title: "code-review of QSL-2935 request diagnostics and request_example (PR 3561)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2935; PR3561 head 827911de23e3872e2cfe8eac18058ccfa99436f6; git diff origin/main...HEAD (base 1167ea0bbf); fsl-command/src/command.rs; fsl-command/src/command/{request_shape.rs,export.rs,analyze.rs,monitor.rs,output.rs,output/types.rs}; fsl-command/src/replay_request.rs; fsl-replay/schemas/filament-outcome-1.schema.json; fsl-command/tests/it/{request_diagnostics.rs,export_selection.rs,cli_contract/compile.rs,main.rs}; tests/it/{compile_command.rs,error_producer_inputs.rs}"
review_set: subset
---

## Summary

Ticket: QSL-2935, PR filament-spec-language#3561, frozen head 827911de23, scope `git diff origin/main...HEAD` over base 1167ea0bbf (merged spec PR #3548). Code-review with the rust-review lane folded in. Governing spec: FR-1047, TC-3170, with FR-644, FR-620, FR-027, FR-862, TC-105, TC-2443, and FR-286 for the outcome schema.

The PR adds `request_shape::admit`, called first by every entry family (run, run-reader, compile, analyze, monitor, export), which reports every missing member as an RFC 6901 pointer (one-of groups as arrays) and refuses a format the family does not read with `{path:"/format", accepted:[...]}`. It also adds `request_example` and `RequestExampleError`, and removes `RunCause::Format`, the `envelope` stage and the circular export text.

Cargo was not run: the host already had about ten concurrent cargo jobs (the one-heavy-job thermal rule applies). All findings are from static reading. The coder's test-pass claim was not re-verified.

## Verdict

NOT MERGEABLE until FND-001 is fixed. It becomes mergeable once FND-001 to FND-007 are fixed.

What holds:

- Entry coverage: every entry in FR-1047's table reaches `admit`. The run variants go through `run_configured`, `prepare_run` through `prepare_run_with_overrides`, every compile entry through `compile_outcome_inner`, every analyze entry through `analyze::read`, plus `monitor` and `export_with_limits_and_overrides`. `RequestOverrides` supplies no required member, so calling `admit` before the overrides apply is correct.
- Member order and the first-absent rule: `/format`, then `/request` only when `format` is absent. AC-1 to AC-5 messages and details are asserted exactly. Monitor `records` is required under the monitor-run reading and not listed under the monitor reading (FR-644).
- AC-8: the formats list and error variants match exactly, and `code()` returns `invalid-request`.
- Changed assertions in `compile_command.rs`, `cli_contract/compile.rs` and `error_producer_inputs.rs` (`producer.future/99`) follow the amended FR-027-AC-2 and FR-1047-AC-5.
- RawValue probe: descent is bounded by the static shape (at most four levels). A deep or malformed document fails serde's recursion limit, and `admit` then defers to the owning reader without panicking. The input is bounded by `request_bytes`. The only `expect` calls are in `write_object`, over static leaf constants, not untrusted input.
- `clause.rs` is untouched by the PR, so its `cargo fmt --check` diff predates it.
- QSL-2937 is correctly out of scope. No spec pins the requested texts, and FR-865-AC-1 currently pins `package: success (exit 0) at S4` as the whole line and stage labels such as `S3` in text, so those texts need a spec amendment first.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Its own entry refuses `request_example("native-replay/1", None)`. The example's `replay_target` is `{"arguments":false,"members":{"function":"entry"}}` with no `occurrence_key`, so `run_with_providers` refuses `invalid_runtime_input`/`missing-member` at `/replay/occurrence_key`, stage request (fsl-replay request.rs:616/725), before any file is read. FR-1047 Behavior requires "Every value in the example, in scope or not, SHALL pass the value checks its reader applies before it reads a file". It only fixes `byte_provision` `[]` for replay, and nowhere allows the replay example to be refused. The AC-7 test pins the defect by asserting `code == invalid_runtime_input`. Fix: ship a complete, self-contained replay example (a real function record and occurrence key) that the entry admits, and assert that it is admitted. | fsl-command/src/command/request_shape.rs:319-343, fsl-command/src/command/request_shape.rs:697-700, fsl-command/tests/it/request_diagnostics.rs:329-334 |
| FND-002 | medium | `admit` deserializes the envelope and `request` into `BTreeMap<String,&RawValue>`, which silently keeps the last of any repeated key. `another_defect` re-serializes that deduplicated map and checks only `request`. So `{"format":"native-run/1","request":{"models":[],"models":[]}}`, a repeated `format`, or an unknown envelope member such as `{"format":"native-run/1","request":{},"x":1}` gets the FR-1047 missing-members refusal, not the repeated- or unknown-member refusal that FR-1047 says wins ("A request that also carries an unknown member, a member of the wrong JSON kind or a repeated member is refused for that defect"). This affects run, run-reader, compile, analyze and monitor. Export pre-checks with `unique_json`. Untested. | fsl-command/src/command/request_shape.rs:466-504, fsl-command/src/command/request_shape.rs:540-572 |
| FND-003 | medium | When the export or evidence member is absent, `admit` skips `another_defect`, so an unknown or wrong-kind member loses to the missing-members refusal. The changed `export_selection.rs` expectation pins this: a request with unknown member `z` now gets `missing required members /request/program \| /request/package, /request/export: ...`. This contradicts the same FR-1047 clause as FND-002. | fsl-command/src/command/request_shape.rs:528-533, fsl-command/tests/it/export_selection.rs:139-142 |
| FND-004 | medium | `run_selected_bytes` runs FR-1015's `request::read` on a present `export` member before `admit` checks the format. `{"format":"native-compile/1","request":{"export":{}}}` with `Ordinary` therefore gets FR-1015's refusal, not `format native-compile/1 is not read by export`. FR-1047: "When format is a string, the entry SHALL apply the format rule below before it checks request". | fsl-command/src/command/export.rs:142-156 |
| FND-005 | medium | The AC-7 test does not test what AC-7 requires. `if let Some(..) = call(..)` passes silently when the entry accepts. The non-replay branch expects `io-error` naming `program.fsl`, but the evidence example names no `program.fsl`, so the evidence case can only pass by being accepted, and "settles FR-1022's unimplemented-meaning" is never asserted. "The replay example reads no provisioned file" is not asserted. The one-member-removal check uses `contains`, where AC-7 says `details.missing` is exactly that pointer or group. | fsl-command/tests/it/request_diagnostics.rs:328-365 |
| FND-006 | medium | The FR-1047-AC-2 clause "A document carrying `argv` that FR-661's version adapter reads is not refused by this requirement" (TC-3170 step 1) has no test. `admit` returns early on any `argv` key, untested. | fsl-command/tests/it/request_diagnostics.rs:109-131, fsl-command/src/command/request_shape.rs:469-473 |
| FND-007 | medium | The FR-1047-AC-6 clause "A request whose `export` lacks `profile` is still refused as FR-1015 states" (TC-3170 step 3, third call) has no test in the tagged AC-6 test. This is the path the new pre-`admit` `request::read` call in export.rs exists for. | fsl-command/tests/it/request_diagnostics.rs:247-269, fsl-command/src/command/export.rs:142-155 |
| FND-008 | low | In `error_producer_inputs.rs`, adding `clause` to `base` also changes the `call` case, which now carries both `call` and `clause`. `ExtractionRequiresRun` is therefore exercised with two selectors instead of `call` alone. Add `clause` only to the cases that lack a selector (package, clause-count). | tests/it/error_producer_inputs.rs:1017-1024 |
| FND-009 | low | `another_defect` decides "missing member versus another defect" by matching serde's `Display` text (`starts_with("missing field \`")`). That is an unstable contract across serde versions and wrappers, so a wording change silently reorders refusals. Classify on a typed signal (for example a deny-unknown and duplicate pre-pass) instead of error text. | fsl-command/src/command/request_shape.rs:571 |
| FND-010 | low | The PR's tracker attachment is titled "QSL-2935, QSL-2937: ..." but delivers nothing for QSL-2937, which is deliberately left to a separate spec PR. Retitle so that the merge does not read as closing QSL-2937. | - |
