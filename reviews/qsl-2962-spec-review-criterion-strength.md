---
id: SR-3611003
title: "Criterion-strength review of FR-027-AC-15..17 and FR-286-AC-41 (filament-spec-language PR 3611)"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language@ce7b26509779e9bcdb14c50656ddf19938f4178a; spec/functional/FR-027-export-compiled-native-package.md (FR-027-AC-15, AC-16, AC-17); spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md (FR-286-AC-41); spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md (FR-865-AC-19)"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
---

# Criterion-strength review of FR-027-AC-15..17 and FR-286-AC-41 (PR 3611)

## Summary

Ticket: QSL-2962. No Jev (typesafe.ai System One) client is on this host, so
the calibrated semantic judgment was skipped. This review was done by hand
over the spec text, with the code read to check each value.

AC-15 is falsifiable on every diagnostic member. Each value matches the code,
so a wrong code, cause, selection, hint, locus or details fails it. AC-16's
three libraries (supplied but unimported, absent, transitive-only) tell a
direct-import check apart from a supplied-set check. AC-17's byte equality
catches a located site that changes the outcome. FR-865-AC-19 pins the hint
line and selection. FR-286-AC-41 cannot be tested: neither clause has a
reachable producer (SR-3611001 FND-002, FND-003).

Weakness kind: `partial` for FR-027 (missing adverse cases, unpinned
message), `untestable` for FR-286-AC-41. Adverse-case coverage of FR-027's
`call_site` text: partial.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-15 is compound and partly not observable. Its last clause, "the item-level mapping of the same refusal is FR-121-AC-35's `Declined` with `InvalidInput` and `missing_import`", is about `TerminalValue::from_call_site_refusal`. A compile outcome has no items, so a test of the compile request cannot observe it, and FR-121-AC-35 already binds it. Drop the clause (author question 2). | spec/functional/FR-027-export-compiled-native-package.md:157 |
| FND-002 | medium | AC-15 pins the message only by prefix and suffix. It asserts no exit code, category or stage. A writer that gets the package name wrong, or that settles the refusal as internal-failure (30) or at the wrong stage, still passes. Candidate: "a `message` exactly `missing_import/missing-selection: package <the unit's identity label> imports no library test/other`, category `refusal`, exit 20, stage `<stage>`". The package name is `package_name(source.identity())`, fsl-command/src/command.rs:5083. | spec/functional/FR-027-export-compiled-native-package.md:157 |
| FND-003 | medium | The adverse cases of the general rule are missing. FR-027 says every `CallSiteRefusal` refuses the compile with `from_call_site_refusal`, but AC-15..17 cover only `UnknownImport`. No AC covers `{test/units, nope}` (`UnknownImportedFunction`: `missing_declaration`/`missing-name`, no `selection`, the catalog hint) or `NotImportable` (`ill_typed`/`operator-ineligible`). Both fixtures are already in FR-121-AC-36. A command that wired only `UnknownImport`, or that let a non-pair refusal carry a `selection`, passes. Add an AC with those two requests. | spec/functional/FR-027-export-compiled-native-package.md:157-159 |
| FND-004 | high | FR-286-AC-41 cannot fail by a correct implementation, and it cannot be satisfied either. No compile request reaches `PackageError::MissingDefinition`: `ExtensionRefusal::MissingDependency` refuses first, and the profile-closure path has only test callers. No checked package's reader refuses `LibraryRefusal::MissingImport`. A test can only build the refusal values by hand, which proves nothing about a command. Drop AC-41 and TC-3211 (SR-3611001 FND-002, FND-003). | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:979 |

## Verdict

Partial. AC-15..17's values are strong and grounded. FND-001..003 tighten
them. FR-286-AC-41 is untestable and should go.

## New findings (disposition pass 1)

Head cc626a9d4b451600aa8b511850b8655b34f2b21b.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The new AC-18 and AC-19 are weakly pinned. AC-18 says the `hint` is "present exactly when the hint catalog has a row for the pair". The catalog has both rows: `missing_declaration`/`missing-name`, "Declare the name or import the package that declares it." (fsl-replay/src/outcome/hint.rs:218-222), and `ill_typed`/`operator-ineligible`, "Use an operator that accepts this type." (hint.rs:146-150). `OutcomeDiagnostic::new` sets that hint with no selection (outcome.rs:2511). Pin both texts; AC-19 asserts no hint at all. Neither AC asserts `locus`, `details`, `message`, category, exit or stage, though FR-027:100 gives every selection refusal category `refusal`, exit 20, stage I1. The exact messages, from call_site.rs:492 and 504 with `ImportedFunctionName` displayed as `library::function`: `missing_declaration/missing-name: package test/units declares no function test/units::nope` and `ill_typed/operator-ineligible: package test/units function test/units::first has a package-dependent signature`. Add "`locus` `null`, `details` `null`, that `message`, category `refusal`, exit 20, stage I1" to each. | spec/functional/FR-027-export-compiled-native-package.md:164-165 |
| FND-006 | low | AC-21 leaves out two cases the text refuses. A `call_site` without `kind` is refused by "exactly those three members", and a `library` of `""` is not a library identity: `LibraryName::new` refuses only the empty string (fsl-semantics/src/library/mod.rs:53-59). Add "lacks `kind`" and "has `library` `\"\"`" to AC-21. | spec/functional/FR-027-export-compiled-native-package.md:167 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-15 no longer has the item-level `Declined`/`InvalidInput` clause. |
| FND-002 | fixed | AC-15 pins `message` exactly, as `missing_import/missing-selection: package <I> imports no library test/other` with `<I>` the program source's `identity` label. AC-16 asserts category `refusal`, exit 20 and stage I1. |
| FND-003 | fixed | AC-18 covers `{test/units, nope}` (`missing_declaration`/`missing-name`, no `selection`). AC-19 covers `{test/units, first}` (`ill_typed`/`operator-ineligible`, no `selection`). Their pinning is FND-005. |
| FND-004 | fixed | FR-286-AC-41 and TC-3211 are deleted. |
