---
id: SR-967175026302197768900474181941935084792
title: "Scope-boundary review of QSL-2937 result texts (PR 3563)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/filament-spec-language; PR 3563 head cab8261ac57853eb726bf906c5ba6a42808c7b9c; spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md, spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md, spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md, spec/test-cases/TC-3197-text-form-names-the-stage-in-words.md, spec/test-cases/TC-3198-compile-success-text-says-what-was-produced-and-where.md, spec/test-cases/TC-3199-missing-import-hint-is-specific-to-its-cause-kind.md, spec/test-cases/TC-3200-unselected-model-message-names-the-selected-identities.md, spec/test-cases/TC-3201-outcome-details-admit-missing-and-accepted.md, spec/test-cases/TC-3202-export-request-that-is-not-an-object-is-refused-by-name.md"
review_set: subset
---

## Summary

Ticket: QSL-2937. Spec-only PR #3563, reviewed as `git diff origin/main...HEAD` at cab8261ac57853eb726bf906c5ba6a42808c7b9c (merge base e2b72b5f67). quire validate over the ten touched files exits 0 (EARS warnings only, all on pre-existing lines). No cargo was run; current-behaviour claims were checked by reading origin/main source. Checked the FSL/driver boundary against filament-driver origin/main (c4f1e26) FR-002-AC-13 and the ticket's four items.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Design question. Driver FR-002-AC-13 (filament-driver origin/main) requires the text to be 'the FR-865 text form of the document `--format json` writes', with the header naming the invoked CLI command (`compile`) instead of the operation. FR-865 has `CompileOutcome::render_text` return `package: success ...` and two lines that are not in the document. Separately, the non-serialized cause kind makes two documents with identical JSON bytes render different text (in-process vs read from JSON). That weakens 'keeps the two forms one document' (FR-865:46-47) and the driver's equality. Confirm with the architect and the driver lane which header the driver writes and whether FR-002-AC-13 gets a compile-success exception. | spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md:46-47,138-158,166-169 |
| FND-002 | low | Design question. An unselected `--library` identity keeps the bare message, while `--model` now lists what the source selects. Ticket item 3 names only models, but the same mistyped-identity problem applies to `--library`, using the unit's `import` identities. Confirm whether leaving it out is intended. | spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md:115-129 |
| FND-003 | low | Ticket item 2 says text renderings show stage labels 'and causes like missing-selection with no explanation'. The PR handles stages only, and `  cause: missing-selection` stays bare. Confirm that cause explanations are out of scope, or file the follow-up. | spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md:115-136 |

## Verdict

One medium design question and two low findings. Not a merge blocker on its own, but the architect should rule on FND-001 together with integrity FND-001/FND-002.

## Scope and evidence

Review set is the subset the dispatch named (base, ears, integrity, evidence, failure-domain, dependency, scope-boundary).

- FR-865#compile-success (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): ### The text of a compile success Today the text of a successful `compile` is the one line `package: success (exit 0) at S4`, which does not say that a package was produced or where it went. The `package` outcome document carries no package size and no path (FR-810 keeps `artifacts` empty), so the text of a successful compile is written from the typed `CompileOutcome` (FR-027), which holds the package bytes and, when the request carried `output` (FR-810) or the caller gave `RequestOverrides::output`, the path as the caller wrote it. `CompileOutcome::render_text` returns it, and the driver w...
- FR-865#description (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): ## Description Text is the driver's default output format, so the first command a reader types, `fsl check rule.fsl`, prints a result without a flag. The verbs `parse`, `check`, `compile`, `run`, `replay` and `export` SHALL write, in text mode, the text form of the FR-286 outcome document they write that FR-298's `render_outcome` returns, and SHALL exit with the same FR-285 code it exits with in machine mode. `analyze` and `monitor` keep the text forms FR-621 and FR-624 and the driver's FR-022 and FR-025 give them; this requirement does not change them. The text form is a display of the typ...
- FR-865#missing-import-hints (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): ### The missing_import hint of each cause kind Today every `missing_import` diagnostic of cause `missing-selection` carries the one catalog hint `Import the package that supplies the selected name.` (FR-286 "Fix hints"), beside a message that may name a model, a library, a byte-provision entry or a function owner, none of which is a package to import. Six producers write the pair; the typed diagnostic SHALL carry the kind of its producer as a member that is not serialized, so the JSON document, its `hint` included, is unchanged. In text mode the ` hint:` line of such a diagnostic is the tex...
- FR-866#unselected (spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md, examined): - A `--model` identity that no `model` declaration of the unit selects, or a `--library` identity that no `import` of the unit names, refuses `invalid-request`, exit 20, naming the option and the identity, after the source parses and before the file is read: an input that changes nothing is refused, as FR-640 refuses an unused clock selection. For a `--model` identity the refusal also names the identities the source does select. Today its message is `invalid clause request member --model: --model <identity> is not selected by the source`, which leaves a caller who mistyped one identity noth...
- FR-865#stage-words (spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md, examined): ### Stage words The text writes each stage in words, the word of this table, and never its label. `last_stage` in the JSON document is unchanged. | Label | Word | Stage (ADR-011) | | --- | --- | --- | | `S0` | `source` | source bytes | | `S1` | `syntax` | the lossless CST | | `S2` | `forms` | parsed semantic forms | | `I1` | `intake` | domain-package intake | | `S3` | `checking` | the checked semantic graph | | `S4` | `packaging` | the linked checked package | | `S5` | `contract` | contract IR | | `S6a` | `execution` | reference execution | | `S6b` | `proof` | bounded proof | | `S6c` | `mod...

## New findings (disposition pass 1)

Found at head 6b1b2799540a7015585d05a7aaf109ecf1501034 while checking the fix round.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | AC-16 requires `fsl compile` to write 'the bytes `render_outcome` returns' for the document. Driver FR-002-AC-13 (filament-driver origin/main) says the text header names the invoked CLI command (`compile`) instead of the document's operation (`package`). A byte-exact Driver test will fail against the driver's own contract. AC-3 says 'the text form', which leaves room for that. Use AC-3's wording plus the header exception, or have the driver lane drop the header substitution. | spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md: FR-865-AC-16; filament-driver FR-002-AC-13 |

## Dispositions

Round 1, head 6b1b2799540a7015585d05a7aaf109ecf1501034 (rebased on origin/main; fix commit 6b1b279954).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Answered by the document-only rule. Text renders FSL's document, and the size, path and selection kind are serialized members, so equal JSON gives equal text. The driver-header residue is new FND-004. |
| FND-002 | accepted-no-change | The author decided to leave `--library` bare. FR-866 records why, and FR-866-AC-10 pins the bare message, so the choice is explicit and tested. |
| FND-003 | accepted-no-change | The Rationale states that explaining `  cause: missing-selection` is outside QSL-2937's four texts, so it is a recorded scope decision. |
