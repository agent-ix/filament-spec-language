---
id: SR-2026111608570001
title: "Code review of filament-driver PR 160 fsl compile and run --clocks"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-driver; fsl-cli/src/cli.rs, fsl-cli/src/clocks.rs, fsl-cli/src/main.rs, fsl-cli/Cargo.toml, fsl-cli/tests/clocks_flag.rs, fsl-cli/tests/fixtures/clocks-stamped.fsl, Call/ClauseRunRequest literals in src/ and tests/, Cargo.lock at bbaf6586845972aa7825757e3917cd56a28222de; language fsl-command at 45705f2"
review_set: subset
evaluated_revision: "bbaf6586845972aa7825757e3917cd56a28222de"
---

## Summary

Ticket: QSL-2273. Code review with the rust-review lane (rust-style baseline) of the diff from 15db1ac (main) to bbaf6586845972aa7825757e3917cd56a28222de. Evidence: `cargo test -p fsl-cli --test clocks_flag` gave 6 passed. The built `fsl` binary was run by hand on a request carrying `clocks` with `--clocks`, on an unknown-kind file, on an unknown-kind request member, and on a file over `--request-bytes`. Clippy and `make ci` were not run; the lead runs the gate.

Question 1 (the both-given stage). FSL emits null. `RunError::outcome_document` puts `RunCause::ClocksGivenTwice` in its catch-all arm, which builds `OutcomeDocument::new(operation, None, ...)` with cause `None` (fsl-command/src/command.rs:498-513 at 45705f2, the cause at line 501). FSL's other projection, `RunError::value` through `output::error`, maps the same cause to `Stage::Request` (fsl-command/src/command/output.rs:114). FSL's own test `request_overrides.rs` checks only that projection, so it never sees the null. The driver passes FSL's document through unchanged (fsl-cli/src/cli.rs:840 and 1249) and drops nothing. Reproduced: `fsl run both.json --clocks c.json --format json` writes `"last_stage":null` and `"cause":null`. The same null-stage projection applies to a malformed clocks member read through `RunCause::Json`. FR-640 (lines 158-162) and the W5 batch FR-640-AC-13 and AC-14 require stage `request`.

Cargo.lock: only the ten fsl-* packages move, from dcf3165 to 45705f2. filament-contract-model stays at 80e3f67. The one manifest change adds serde_json's `raw_value` feature, with the version pin unchanged. There is no Python and no logic-bearing shell. The `python_path()` fixture helper in cli.rs is from before this PR; the diff only adds `clocks: Default::default()` to its literals.

## Verdict

REQUEST CHANGES, through FND-001. The driver code is sound. `clocks::read` caps the read at limit+1 bytes, refuses non-UTF-8, non-JSON, trailing content and non-objects through `RawValue`, and keeps the text as written so a repeated name reaches FSL's reader. Usage errors (missing or repeated operand) refuse before any file is read. The overrides reach `prepare_run_with_overrides` and `compile_outcome_with_overrides` unchanged. FND-001 is a language-side defect. It blocks the ticket AC "checking code, cause and stage" for the both-given refusal, and needs a language ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FSL's outcome document records the both-given refusal (`ClocksGivenTwice`) with `last_stage` null and `cause` null, while FR-640 and FR-640-AC-14 require stage `request`. FSL's `output::error` maps the same cause to `Stage::Request`, so FSL's two projections disagree, and only the `value()` one is tested. Language-side defect; the driver forwards the document unchanged. Fix (language): in `outcome_document`, give `ClocksGivenTwice` (and the other request-stage causes `output.rs` maps to `Stage::Request`, including clocks-member `Json` errors) `Some(Stage::Request)`, and assert `outcome_document` in `request_overrides.rs`. Then bump the driver lock and assert `last_stage == "request"` (see the gap-analysis SR, FND-001). | filament-spec-language fsl-command/src/command.rs:498-513, fsl-command/src/command/output.rs:114, fsl-cli/src/cli.rs:840 |
| FND-002 | medium | An oversize `--clocks` file refuses `invalid-request`, exit 20, cause `clocks-file-too-large`. An oversize request file refuses `resource_exhausted`, category `incomplete`, exit 22 (`request_bytes limit 100 exceeded; raise --request-bytes`). Both are bounded by the same `--request-bytes` value on `run`, so one limit gives two different categories. On `compile`, the driver uses `RunLimits::default().request_bytes` (FSL's fixed 1 MiB), and nothing tells the user what bound applies. Fix: refuse the oversize file as FSL refuses an oversize request (`resource_exhausted`, naming `--clocks` and the limit), or justify `invalid-request` in FR-023. Either way, state the bound per verb. | fsl-cli/src/clocks.rs:45-50, fsl-cli/src/cli.rs:826, fsl-cli/src/cli.rs:1238 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | rejected | Withdrawn: not a language defect. The outcome document's `last_stage` is an ADR-011 `OutcomeStage`, which has no `request` value; FSL FR-286 and FR-287-AC-11 require `last_stage` null for request-intake refusals, and fsl-command `tests/it/prepared_command.rs::intake_failures_have_their_original_codes_and_no_invented_stage` asserts it. FR-640's "stage `request`" is the refusal envelope's stage (`output::error`), which already emits `request`. QSL-2298 is cancelled; W5 is clarifying FR-640-AC-13/14. |
| FND-002 | fixed | At 8a514fa, `clocks::read` refuses an oversize file `Code::ResourceExhausted` (category incomplete, exit 22), cause `clocks-file-too-large`, with message "--clocks: <path> exceeds the <limit> byte limit; raise --request-bytes" on `run` and no raise hint on `compile`, whose bound is FSL's fixed 1,048,576 bytes. FR-023 states the bound per verb. |

## Verdict (disposition pass 1)

APPROVE at 8a514fa. FND-001 is withdrawn, and FND-002 is fixed. No new findings. `cargo test -p fsl-cli --test clocks_flag --test frontend_cli --test cli_contract` gave 7, 17 and 19 passed.
