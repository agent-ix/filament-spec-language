---
id: SR-967175026302197768900474181941935084789
title: "Evidence review of QSL-2937 result texts (PR 3563)"
type: SpecReview
analysis: evidence
scope: "agent-ix/filament-spec-language; PR 3563 head cab8261ac57853eb726bf906c5ba6a42808c7b9c; spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md, spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md, spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, spec/test-cases/TC-3197-text-form-names-the-stage-in-words.md, spec/test-cases/TC-3198-compile-success-text-says-what-was-produced-and-where.md, spec/test-cases/TC-3199-missing-import-hint-is-specific-to-its-cause-kind.md, spec/test-cases/TC-3200-unselected-model-message-names-the-selected-identities.md, spec/test-cases/TC-3201-outcome-details-admit-missing-and-accepted.md, spec/test-cases/TC-3202-export-request-that-is-not-an-object-is-refused-by-name.md"
review_set: subset
---

## Summary

Ticket: QSL-2937. Spec-only PR #3563, reviewed as `git diff origin/main...HEAD` at cab8261ac57853eb726bf906c5ba6a42808c7b9c (merge base e2b72b5f67). quire validate over the ten touched files exits 0 (EARS warnings only, all on pre-existing lines). No cargo was run; current-behaviour claims were checked by reading origin/main source. Checked that each new obligation has an AC with a verification method that can fail, and that each AC names a concrete input.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The driver obligations ('When `fsl compile` in text mode succeeds, the driver SHALL write the three lines', and the no-output third line) have no Driver test AC. AC-11 and AC-12 test only `CompileOutcome::render_text` in process. The one driver AC, AC-3, asserts the opposite (see integrity FND-001). Add a Driver test AC, with the TC delegated to filament-driver as AC-3 does, for `fsl compile` text with and without `--output`. | spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md:229-233,250 | correct-requirement-no-evidence |
| FND-002 | medium | FR-865-AC-10, FR-865-AC-20 and FR-286-AC-38 each require bytes 'byte-equal to the bytes written before this requirement' (or 'before this row'). No baseline is named, so a test needs a stored pre-change golden. That is a snapshot pin with no owner, which goes stale with the next unrelated document change. State the exact property instead: `last_stage` is the label string, `hint` equals the catalog text, and `details` holds no `missing` or `accepted` member. | spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md:257,267; spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:890 | wrong-requirement |
| FND-003 | medium | Three ACs give no input that a test can use. AC-15 ('a refusal of kind sibling-unit') names no input. AC-16 cites 'FR-862's refusal of a package reference with no matching byte-provision entry', but no FR-862 AC states such a refusal; the producer is fsl-replay `IncompleteByteProvision` (fsl-replay/src/request.rs:565), owned by FR-098/FSpec. AC-17 names no fixture, although FR-098-AC-33 gives exactly the `function_owner` `test/absent` refusal. Cite a concrete fixture or source AC for each. | spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md:262-264 | correct-requirement-no-evidence |

## Verdict

Three medium findings. AC-9 (all 12 labels plus null), AC-11/AC-12 (both output cases), FR-866-AC-8/AC-9 (exact messages with a fixture each) and FR-1047-AC-9 (exact input and message, matching `export.rs:157-160` on PR #3561) are concrete and can fail. Mergeable once fixed.

## Scope and evidence

Review set is the subset the dispatch named (base, ears, integrity, evidence, failure-domain, dependency, scope-boundary).

- FR-865-AC-9 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): `render_outcome` with `RenderForm::Text` over an outcome document whose `last_stage` is each of `S0`, `S1`, `S2`, `I1`, `S3`, `S4`, `S5`, `S6a`, `S6b`, `S6c`, `S7` and `S8` writes a header line ending ` at stage source`, ` at stage syntax`, ` at stage forms`, ` at stage intake`, ` at stage checking`, ` at stage packaging`, ` at stage contract`, ` at stage execution`, ` at stage proof`, ` at stage model-checking`, ` at stage witness` and ` at stage replay` in that order, and no line of the text contains a stage label as a word; over a document whose `last_stage` is null the header has no ` a...
- FR-865-AC-10 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): The JSON bytes of each AC-9 document hold `last_stage` as its label (`"S4"`, `"I1"`) and are byte-equal to the bytes written before this requirement.
- FR-865-AC-11 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): `fsl_command::command::compile_outcome_with_overrides` over FR-810-AC-1's request, whose `output` is `{"package": "spine.package.json"}`, returns a `CompileOutcome` whose `render_text` is exactly the three lines `package: success (exit 0) at stage packaging`, `produced: checked package (<n> bytes)` with `<n>` the length of `package_bytes()`, and `written to: spine.package.json`.
- FR-865-AC-12 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): `compile_outcome_with_overrides` over FR-810-AC-1's request without `output` and without `RequestOverrides::output` returns a `CompileOutcome` whose `render_text` is exactly `package: success (exit 0) at stage packaging`, `produced: checked package (<n> bytes)` and `nothing written: no --output option or output member was given`, and the text contains no `written to:`.
- FR-865-AC-15 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): The text of a refusal of kind `sibling-unit` holds the line ` hint: Supply the imported unit as a library, not as a sibling unit.`.
- FR-865-AC-16 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): The text of FR-862's refusal of a package reference with no matching byte-provision entry holds the line ` hint: Add a byte_provision entry for the named source to the replay request.`.
- FR-865-AC-17 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): The text of a replay of a `function` request naming a `function_owner` the unit does not import holds the line ` hint: Name a function_owner that the program imports directly, or add the import.`.
- FR-865-AC-20 (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): The JSON document of each AC-13 to AC-18 refusal carries `hint` `Import the package that supplies the selected name.` and is byte-equal to the bytes written before this requirement.
- FR-865#behavior-new (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): - When `render_outcome` writes the header line, it SHALL write the stage as `at stage ` and the stage's word of "Stage words", and SHALL NOT write a stage label. - When `fsl compile` in text mode succeeds, the driver SHALL write the three lines of "The text of a compile success", and the second line SHALL state the length of the package bytes. - Where a compile success has no output path, the third line SHALL state that nothing was written and why. - When a `missing_import` diagnostic of cause `missing-selection` is rendered as text, `render_outcome` SHALL write as its hint line the text of...
- FR-866-AC-8 (spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md, examined): `check_outcome` over FR-108's unit extended with a second declaration `model Other = "example/other"` after `model Config = "example/config-version"`, with the model selection `example:shop/model`, refuses `invalid-request`, exit 20, with message exactly `invalid clause request member --model: --model example:shop/model is not selected by the source; the source selects: example/config-version, example/other`, in text and in machine mode alike.
- FR-866-AC-9 (spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md, examined): `check_outcome` over `tests/fixtures/spine-compile.native` (no `model` line) with the model selection `example:shop/model` refuses `invalid-request`, exit 20, with message exactly `invalid clause request member --model: --model example:shop/model is not selected by the source; the source selects no models`.
- FR-286-AC-38 (spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md, examined): The outcome document of FR-1047-AC-1 serializes with one diagnostic whose `details` is exactly `{"missing":["/request/models",["/request/program","/request/package"],["/request/call","/request/clause","/request/items"]]}`; the document of FR-1047-AC-5's `native-analyze/1` run refusal serializes with `details` exactly `{"accepted":["native-run/1","native-monitor/1","native-replay/1"],"path":"/format"}`; every other member of both documents is as FR-286 states, and a document whose `details` carries neither member is byte-equal to the bytes written before this row.
- FR-1047-AC-9 (spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, examined): `export_with_limits_and_overrides` with `ExportSelection::Ordinary` over `{"format":"native-run/1","request":3}` refuses `invalid-request`, stage `request`, exit 20, with message exactly `invalid clause request member /request: expected an object`; this requirement adds no `missing` list for it, since a `request` that is not an object has no members to list.

## New findings (disposition pass 1)

Found at head 6b1b2799540a7015585d05a7aaf109ecf1501034 while checking the fix round.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | `render_outcome` takes a typed `&OutcomeDocument` (fsl-inspect/src/render.rs:261-267) and reads `last_stage` from its serialization. `OutcomeStage` (fsl-replay/src/outcome.rs:141-166) has no `I2` variant and no way to hold an arbitrary string, and the published schema's `last_stage` enum excludes both. So AC-10's document with `last_stage` `X9`, and AC-9's `I2` case, cannot be built and passed to `render_outcome`. As written, no test can exercise those cases. Either state them over the stage-word map itself (a pub function from label string to word), or say that `OutcomeStage` gains `I2` and drop the unknown-label case. | spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md: FR-865-AC-9, FR-865-AC-10, Stage words |

## Dispositions

Round 1, head 6b1b2799540a7015585d05a7aaf109ecf1501034 (rebased on origin/main; fix commit 6b1b279954).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-16 is a Driver test of `fsl compile` with and without `--output`, delegated as AC-3 is. See scope-boundary FND-004 for its header. |
| FND-002 | fixed | The 'byte-equal to the bytes before' clauses are replaced by exact properties and schema validation. |
| FND-003 | fixed | Each kind now cites a concrete source AC or an input: FR-099-AC-3, FR-121-AC-35, FR-098-AC-33, FR-071-AC-5 (all verified to exist and to state those refusals), and an explicit sibling-unit input. |
