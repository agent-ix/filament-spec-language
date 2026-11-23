---
id: SR-3611002
title: "Integrity review of the compile request call_site member (filament-spec-language PR 3611)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@ce7b26509779e9bcdb14c50656ddf19938f4178a; spec/functional/FR-027-export-compiled-native-package.md; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md; spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md; spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md; spec/test-cases/TC-3199, TC-3210, TC-3211"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-121
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-3210
    type: references
---

# Integrity review of the compile request call_site member (PR 3611)

## Summary

Ticket: QSL-2962. Completeness, consistency and atomicity of the changed
text. New ids FR-027-AC-15..17, FR-286-AC-41, TC-3210 and TC-3211 are
unique. The TC ids are free on fresh origin/main. TC-3199 step 3 and
FR-865-AC-19 point at the same fixture. TC-3210 traces to FR-027, FR-121
and FR-286, and its scope lists exactly AC-15..17. `quire validate` passes.
The unbuildable AC-41 and the `call_site` entry mismatch are SR-3611001
FND-001..003.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-027 Inputs still say "The request contains only models, program and, for a `1-draft` program, `libraries`, ... `clocks` ..., `handoff` ... and ... `output`. It contains no runtime artifacts or execution selection." `call_site` is a selection member, and it is not in that list. The Outputs text now contradicts the Inputs section. Candidate: add "the optional `call_site` member (a located imported-function selection that changes nothing in the outcome)" to the list, and change "no ... execution selection" to "no execution selection other than `call_site`". | spec/functional/FR-027-export-compiled-native-package.md:54-59 |
| FND-002 | low | The dependency direction is inverted. FR-027's new text depends on FR-121 (the entry and `ImportedFunctionName`) and FR-286 ("Missing selections"). But FR-027's Dependencies and `relationships` gain neither, and FR-121's Dependencies gain FR-027, a dependent. Fix: add FR-121 and FR-286 to FR-027's Dependencies, with `depends_on` relationships. In FR-121, move the FR-027 line to a "Used by" note, or drop it. | spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:515; spec/functional/FR-027-export-compiled-native-package.md:161-170 |
| FND-003 | medium | The refusal of a malformed `call_site` is only partly stated. The text covers "A `kind` other than `imported-function`, or any other member". It does not cover a missing `library` or `function`, a non-string value, a `library` that is not a `LibraryName`, a `function` that is not an `Identifier`, or `call_site` `null` or not an object. Candidate: "`call_site` that is not an object with exactly `kind`, `library` and `function`, or whose `library` is not a library identity or whose `function` is not an identifier, refuses `invalid-request` naming `call_site`." | spec/functional/FR-027-export-compiled-native-package.md:96 |
| FND-004 | low | TC-3210 says "Through `fsl_command::command::compile_outcome` and the `fsl compile` driver". The `fsl` binary is in the filament-driver repo (FR-865-AC-16 says the driver test belongs there), so this repo cannot hold the driver half. TC-3210 and TC-3211 also name no test files (author question 7). Fix: say "through `compile_outcome` (fsl-command/tests/it/cli_contract/compile.rs) and its text rendering (fsl-command/tests/it/selection_text.rs)". Drop the driver, or name it as a filament-driver test. | spec/test-cases/TC-3210-a-compile-request-call-site-refusal-reaches-the-user-with-its-hint.md:22 |
| FND-005 | low | The FR-865-AC-19 binding is now wrong. fsl-command/tests/it/selection_text.rs:130-149 (`the_unimported_library_producer_text_holds_its_hint`, traced TC-3199, FR-865-AC-19) builds `CallSiteRefusal::UnknownImport` by hand with package `fixture`. Re-pointed AC-19 needs the compile refusal of FR-027-AC-15's request, so this test no longer verifies it. The code ticket must rewrite it through `compile_outcome`. Record this in the ticket so the matrix does not count the old test as coverage. | spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md:286 |

## Verdict

Not clean. FND-001 is a real Inputs and Outputs contradiction. FND-003 leaves
refusals that the wiring coder would have to decide. The rest are
bookkeeping. Each has candidate wording.
