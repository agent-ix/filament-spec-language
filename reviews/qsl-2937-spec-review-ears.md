---
id: SR-967175026302197768900474181941935084787
title: "EARS review of QSL-2937 result texts (PR 3563)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; PR 3563 head cab8261ac57853eb726bf906c5ba6a42808c7b9c; spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md, spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md, spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, spec/test-cases/TC-3197-text-form-names-the-stage-in-words.md, spec/test-cases/TC-3198-compile-success-text-says-what-was-produced-and-where.md, spec/test-cases/TC-3199-missing-import-hint-is-specific-to-its-cause-kind.md, spec/test-cases/TC-3200-unselected-model-message-names-the-selected-identities.md, spec/test-cases/TC-3201-outcome-details-admit-missing-and-accepted.md, spec/test-cases/TC-3202-export-request-that-is-not-an-object-is-refused-by-name.md"
review_set: subset
---

## Summary

Ticket: QSL-2937. Spec-only PR #3563, reviewed as `git diff origin/main...HEAD` at cab8261ac57853eb726bf906c5ba6a42808c7b9c (merge base e2b72b5f67). quire validate over the ten touched files exits 0 (EARS warnings only, all on pre-existing lines). No cargo was run; current-behaviour claims were checked by reading origin/main source. Checked every new or edited SHALL statement in FR-865, FR-866, FR-286 and FR-1047 for EARS form and atomicity.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Three of the new FR-865 Behavior bullets carry two obligations each. The header bullet has `SHALL write` plus `SHALL NOT write`. The compile bullet has `SHALL write the three lines` plus `the second line SHALL state`. The missing_import bullet has `SHALL write as its hint line` plus `SHALL leave the ... JSON hint`. Split each into one statement per SHALL so each can be traced to its own AC (AC-9/AC-10, AC-11/AC-12, AC-13..19/AC-20). | spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md:226-237 |
| FND-002 | low | The obligation that the typed diagnostic SHALL carry its producer's kind as a member that is not serialized appears only in the Description prose (FR-865:166-169). No Behavior statement names it, and no AC tests that the member exists or that every producer sets it. Move it to Behavior as one statement whose subject is `OutcomeDiagnostic`. | spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md:166-169 |

## Verdict

Two low findings. The new FR-866 statements (an If/then refusal and a Where for no `model` declaration), the FR-1047 non-object bullet and the FR-286 details bullet are each one well-formed EARS statement. quire's EARS warnings on FR-865:241, FR-866:32/156/168/171 and FR-286:386 are on pre-existing lines. Mergeable once fixed.

## Scope and evidence

Review set is the subset the dispatch named (base, ears, integrity, evidence, failure-domain, dependency, scope-boundary).

- FR-865#behavior-new (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): - When `render_outcome` writes the header line, it SHALL write the stage as `at stage ` and the stage's word of "Stage words", and SHALL NOT write a stage label. - When `fsl compile` in text mode succeeds, the driver SHALL write the three lines of "The text of a compile success", and the second line SHALL state the length of the package bytes. - Where a compile success has no output path, the third line SHALL state that nothing was written and why. - When a `missing_import` diagnostic of cause `missing-selection` is rendered as text, `render_outcome` SHALL write as its hint line the text of...
- FR-865#missing-import-hints (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): ### The missing_import hint of each cause kind Today every `missing_import` diagnostic of cause `missing-selection` carries the one catalog hint `Import the package that supplies the selected name.` (FR-286 "Fix hints"), beside a message that may name a model, a library, a byte-provision entry or a function owner, none of which is a package to import. Six producers write the pair; the typed diagnostic SHALL carry the kind of its producer as a member that is not serialized, so the JSON document, its `hint` included, is unchanged. In text mode the ` hint:` line of such a diagnostic is the tex...
- FR-866#behavior-new (spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md, examined): - If a `--model` identity is selected by no `model` declaration of the unit, then `check_outcome` SHALL end the refusal message with `; the source selects: ` and the unit's model identities in source order, each once, joined by `, `. - Where the unit has no `model` declaration, the refusal message of an unselected `--model` identity SHALL end `; the source selects no models`.
- FR-286#details-new (spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md, examined): - An `invalid-request` diagnostic at stage `request` that FR-1047 writes SHALL carry that requirement's `details`: `{"missing": [...]}` for a request lacking members, and `{"path": "/format", "accepted": [...]}` for a `format` the entry does not read.
- FR-1047#non-object (spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, examined): ### A request that is not an object - If the export entry reads an envelope whose `request` member is not a JSON object, then the entry SHALL refuse `invalid-request` at stage `request`, exit 20, with message `invalid clause request member /request: expected an object`, which no member list can improve.
