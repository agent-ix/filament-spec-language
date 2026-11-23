---
id: TC-3211
title: "A missing definition and a structural missing import carry their selection"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3211: A missing definition and a structural missing import carry their selection

## Description

`PackageError::MissingDefinition` reached through a compile request carries `extension-definition`; the checked-package reader's `LibraryRefusal::MissingImport` carries `package-reference`.

Scope: FR-286-AC-41.

## Test Procedure

Refuse a compile request whose profile closure names an extension definition absent from `extensions`; read a checked package whose structural read refuses `LibraryRefusal::MissingImport`. Serialize each outcome document.

Tag the tests `#[trace("TC-3211", "FR-286-AC-41")]`.

## Expected Results

- The first diagnostic: `code` `missing_import`, `cause` `missing-selection`, `selection` `extension-definition`, `hint` `Add the selected extension's definition to the request's extensions member, or remove the selection.`.
- The second: `selection` `package-reference`, never `library`.
