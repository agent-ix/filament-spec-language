---
id: SR-3611001
title: "Spec review of the compile request call_site member and missing-import selections (filament-spec-language PR 3611)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@ce7b26509779e9bcdb14c50656ddf19938f4178a; spec/functional/FR-027-export-compiled-native-package.md (Outputs call_site text, FR-027-AC-15..17); spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md (Dependencies); spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md (Missing selections, FR-286-AC-41); spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md (FR-865-AC-19); spec/test-cases/TC-3199, TC-3210, TC-3211"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-121
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-3210
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-3211
    type: references
---

# Spec review of the compile request call_site member (PR 3611)

## Summary

Ticket: QSL-2962. Scope `git diff origin/main...HEAD` at head
ce7b26509779e9bcdb14c50656ddf19938f4178a (one commit, 7 files). No code
review; every claim about the code was checked by reading it.

Checked and correct. `OutcomeDiagnostic::from_call_site_refusal`
(fsl-replay/src/outcome.rs:2676) builds `new(cause, code, None, to_string())`
with `with_selection`. For `CallSiteRefusal::UnknownImport` this gives code
`missing_import` (call_site.rs:540), cause `missing-selection` (:558),
selection `unimported-library` (:575), the catalog hint (outcome/hint.rs:185-186),
locus `null`, details `null` (never set), and the message
`missing_import/missing-selection: package {package} imports no library {L}`
(:482). AC-15's values all match. The direct-import-only lookup
(`dependencies().get`, call_site.rs:835-843) gives AC-16's transitive case.
The compile request is `deny_unknown_fields` (fsl-command/src/command/wire.rs:50),
so a closed `call_site` member fits FR-027's "closed format/request JSON
envelope". Category: FR-285 maps `refusal` to 20 and internal-failure to 30.
TC-3210 and TC-3211 are free on fresh origin/main (highest TC-3209). `quire
validate` passes on all 7 files; its 7 warnings are on unchanged lines.
Moving `LibraryRefusal::MissingImport` out of the `library` row is right.

Refuted claims. (1) `LibraryRefusal::MissingImport` has no production path at
all. The checked-package reader cannot carry it (FND-002). (2) No compile
request reaches `PackageError::MissingDefinition`. `CompileRefusal::Profile`
does not carry `ProfileClosureRefusal` (FND-003). (3) FR-121's `call_site`
entry cannot compile "the request's program, models and libraries" as the
command compiles them (FND-001).

Answers to the author's open questions:

1. A `check` surface is not needed. FR-865-AC-16 is `fsl compile` success
   text. FR-865-AC-17 is `check_outcome`'s unsupplied `--model` refusal and
   has nothing to do with call sites. The only `unimported-library` row,
   FR-865-AC-19, now reads through FR-027-AC-15. `check` has no request
   document, only `CheckInputs` or argv (fsl-command/src/command.rs:2306
   `check_argv`). A check surface would need a new driver option, and it can
   be its own FR. The compile half of the ruling is enough for every
   `unimported-library` AC.
2. Drop AC-15's item-level clause. A compile outcome has no items, and
   FR-121-AC-35 already binds `TerminalValue::from_call_site_refusal` (SR-3611003 FND-001).
3. The package name is the program source's `identity` label. That is
   `package_name(source.identity())` (command.rs:5083), which the compile's
   own `check` passes (command.rs:457, 467). For a unit array it is
   `canonical_unit_source` (command.rs:3753). Pin the full message in AC-15,
   for example `missing_import/missing-selection: package <unit identity> imports no library test/other`.
   `call_site` faults `package-name-is-a-library-name` when the label is not a
   valid `LibraryName` (call_site.rs:661-666). The spec should say that case
   settles internal-failure, exit 30, or refuses earlier.
4. Category `refusal`, exit 20 (FR-285 table; FR-027 Outputs "exit code is
   20 for refusal"). `CallSiteRefusal::Fault` maps to code
   `runtime_invariant` (call_site.rs:544): category internal-failure, exit 30.
   FR-027 Outputs also need a stage on every refusal envelope, and the text
   names none (FND-005).
5. Accepting only `imported-function` is fine. It is the only selection kind
   with a `missing_import` selection. State that the other FR-121 selection
   kinds are not accepted, as the text does.
6. No fixture exists, and none can: no command reaches
   `PackageError::MissingDefinition`. A compile whose extension dependency is
   missing from `extensions` refuses earlier, with
   `ExtensionRefusal::MissingDependency`
   (fsl-semantics/src/library/extension.rs:497). That is
   `CompileRefusal::Extension`, and fsl-replay/src/spine.rs:586 already maps
   it to `extension-definition`. Drop AC-41 (FND-002, FND-003).
7. Name the test files. TC-3210: fsl-command/tests/it/cli_contract/compile.rs
   for the compile outcome, and fsl-command/tests/it/selection_text.rs for the
   text line. TC-3210's "and the `fsl compile` driver" belongs to the
   filament-driver repo, as FR-865-AC-16 says (SR-3611002 FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-027 says the command "SHALL ... locate that selection over the request's program, models and libraries with FR-121's `call_site` entry". That entry (fsl-replay/src/call_site.rs:651) takes one unit's `bytes`, the package documents, a `DependencyInput` and clocks. It runs its own parse, select, check and package under `SpineLimits::default()`. It takes no `integer_division` (FR-767), `refinement_internals`, `extensions` or profile catalog (FR-660), `check_mode` (FR-952), FR-869 stage limits, or FR-480 unit array. So for a request that uses any of these, the entry compiles a different package. It can refuse `CallSiteRefusal::Compile` where the compile succeeded, or locate in another package. It is also the second compile pass FR-027's Description forbids ("without a second decoder or compile pass"). Fix: require a located site over the command's own checked package. That needs a new fsl-replay entry over a `CheckedUnit`/`CheckedPackage` (the sealed `Locate` impl at call_site.rs:828 already does exactly this). Or limit `call_site` to requests without those members, refuse the rest `invalid-request` naming `call_site`, and state it. | spec/functional/FR-027-export-compiled-native-package.md:96 |
| FND-002 | high | The new `package-reference` producer text and FR-286-AC-41's second clause say the checked-package reader's `V2ReadRefusal::Structural` carries `LibraryRefusal::MissingImport`. It cannot. fsl-package/src/checked_v2.rs:880-893 builds a `LibraryPackage` with `imports: Vec::new()` and calls `ImportView::of`, which runs only `verify_package` (InvalidExportName, AmbiguousDeclaration; fsl-semantics/src/library/mod.rs:275-296, 329-337). `MissingImport` is written only at mod.rs:429 in `resolve_libraries`, and that has only test callers (fsl-semantics/tests/it/library_resolution.rs, fsl-eval/tests/it/composite_values.rs). `PackageAdmissionRefusal`'s `missing_import` comes from `CheckedPackageRefusalCode::MissingImport` (fsl-package/src/package_admission.rs:239), not from `LibraryRefusal`. So AC-41's clause has no producer and no fixture. Fix: remove ", which includes ... carries it" from the row. Replace the note's first sentence with "`LibraryRefusal::MissingImport` is written only by `resolve_libraries`, which no production entry calls; it writes no outcome diagnostic and has no row." Drop AC-41's second clause. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:589-591 |
| FND-003 | high | The note says `PackageError::MissingDefinition` "is written inside a profile-closure refusal (`ProfileClosureRefusal::Dependency`) that a compile reports as a `Profile` refusal", and AC-41 makes a compile write it with `extension-definition`. Both claims are false. `ProfileClosureRefusal` reaches callers only through `check_with_profile_catalog(_and_inputs)` as `ProfileCheckFailure::Closure` (fsl-replay/src/spine/lifecycle.rs:1858-1878). Only tests call that: spine/lifecycle/tests.rs:1225-1255 and tests/it/extension_forms.rs:564. No `OutcomeDocument` conversion exists. `link_bundle` has only test callers too. `CompileRefusal::Profile` carries `ProfileRefusal` from `resolve_profiles` (lifecycle.rs:1554, 4273), a different type. A compile with an extension dependency missing from `extensions` refuses first with `ExtensionRefusal::MissingDependency` (extension.rs:497), which spine.rs:586 already maps to `extension-definition`. A root-level `MissingDefinition` is code `unknown_profile`, cause `unsupported-selection` (bundle.rs:768-772, 918), not this pair. So AC-41's first clause cannot be reached or tested through a compile request. The wiring coder would have to invent a production path the spec does not require. Fix: drop AC-41 and TC-3211. In the note, say `PackageError::MissingDefinition` is a library-entry cause no command reaches. Keep its row only for library callers, or move it to the "no outcome diagnostic" prose. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:591, 979 |
| FND-004 | medium | `call_site` is not defined for two program shapes. (a) A model-unit program. The command refuses every non-program member in a closed list (fsl-command/src/command.rs:2908-2928; FR-710:351), and `call_site` is not in that list, so the outcome is undefined. (b) An FR-480 unit array. The entry takes one unit's bytes (see FND-001). Candidate: "A `call_site` on a model-unit program refuses `invalid-request` naming `call_site` (FR-710's member list gains `call_site`)". For arrays, state that the selection is located over the array's one checked package, or that it refuses `invalid-request`. | spec/functional/FR-027-export-compiled-native-package.md:96 |
| FND-005 | medium | The text says only "in the category and exit code FR-285 gives a refused compile of that code". FR-285 maps categories to exits, not codes. The text names no stage, but FR-027 Outputs require "the stage and code" on the refusal envelope. No AC asserts exit, category or stage. Candidate: "...refuses the compile, category `refusal`, exit 20, at stage `<stage>`; a `CallSiteRefusal::Fault` settles internal-failure, exit 30". Add "exit 20" and the stage to AC-15. | spec/functional/FR-027-export-compiled-native-package.md:96, 157 |

## Verdict

Not mergeable. The FR-027 diagnostic values are right to the byte, and the
`library` row correction is right. But FR-286-AC-41 cannot be built: neither
clause has a production producer (FND-002, FND-003). FR-027's "with FR-121's
`call_site` entry" cannot compile the request as the command does (FND-001).
FND-004 and FND-005 are gaps the wiring coder would otherwise have to decide.

## New findings (disposition pass 1)

Head cc626a9d4b451600aa8b511850b8655b34f2b21b (rebased on origin/main 3f0154a5).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The locate runs only after "its compile succeeds", but FR-027 does not say what is written when the locate refuses. The command writes the `output` file (fsl-command/src/command.rs:2992) and the `handoff` directory (command.rs:2979, `write_compile_handoff`) after `complete()` returns. A wiring that locates after those writes would refuse the compile and still leave a package file or a handoff on disk. FR-027 Outputs say only that intake and static failures carry no package bytes. Candidate: "...refuse the compile with no package bytes, writing no `output` file and no `handoff` directory". Add to AC-15: "no package bytes, and with `output` set no file is written". | spec/functional/FR-027-export-compiled-native-package.md:100; spec/functional/FR-027-export-compiled-native-package.md:161 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-027:100: "the command shall locate that selection over the request's own checked package, as FR-121's `call_site` does over a checked unit, with no second compile pass and no stage limits other than the request's own". |
| FND-002 | fixed | The `package-reference` row is back to its origin/main text. The new "Producers no command reaches (current)" paragraph says `LibraryRefusal::MissingImport` "is written only by `resolve_libraries`, which no production entry calls; it writes no outcome diagnostic and has no row". This matches fsl-semantics/src/library/mod.rs:429 and checked_v2.rs:880-893. |
| FND-003 | fixed | FR-286-AC-41 and TC-3211 are deleted, and no reference to them is left. The paragraph says `PackageError::MissingDefinition` "is a library-entry cause no command reaches today": `ExtensionRefusal::MissingDependency` refuses first, and `check_with_profile_catalog` "has no outcome-document conversion". Both match the code. |
| FND-004 | fixed | FR-027:100: "or the request's `program` is a model unit or an FR-480 array of units, then the command shall refuse `invalid-request` naming `call_site`". FR-027-AC-21 tests both. |
| FND-005 | fixed | FR-027:100 gives "category `refusal`, exit 20 (FR-285), at stage I1", and a `CallSiteRefusal::Fault` "settles internal-failure, exit 30". FR-027-AC-16 asserts category, exit and stage. |
