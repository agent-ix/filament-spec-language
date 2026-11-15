---
id: FR-660
title: "Supply extension definitions through the request extensions member"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-006
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-354
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-111
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-275
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: references
  - target: ix://agent-ix/filament-specification/FR-467
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-562
    type: depends_on
---
# FR-660: Supply extension definitions through the request extensions member

## Description

A unit selects an extension by a header `profile <alias> = "<identity>";`
whose identity the supplied `DefinitionCatalog` holds as an extension
definition (FR-110 "Extension selection"), and the compiler admits the
extension's forms from that definition (FR-354). The catalog is the caller's
input. A `native-compile/1` request (FR-027) and a `native-run/1` request
(FR-100) SHALL supply it through one optional top-level member,
`extensions`, and the `parse` and `check` lifecycle operations (FR-275)
SHALL take the same catalog as an optional input, so that every path that
reaches S1 reads extension forms from the same caller-supplied definitions.

Definitions come from the request, not from an installed registry, a search
path or a configuration file: a result then depends only on the bytes the
request names (ADR-029 CB-5 "reads no configuration file the arguments do
not name"), and two machines given the same request and files compile the
same package. A definition is identified by the pair `{authority,
identity}` (FR-111 "Closure"), the same pair the unit's header profile
selects and the checked package's `definition_selections` row records
(FR-110); no other name, path or position identifies it.

## Inputs

The `extensions` member: a JSON array of closed records, each one extension
definition entry in FSpec FR-562's entry form:

- `authority`: the definition's authority, a non-blank string;
- `identity`: the definition's identity, a non-blank string;
- `file`: the path of the definition document, resolved against the
  request file's directory as every `file` member of the request is; the
  document holds FSpec FR-467's `grammar_schema` and `typed_node_schema`;
- `depends_on`: optional, an array of `{authority, identity}` records, the
  definition's dependency edges; absent is `[]`;
- `capabilities`: optional, an array of capability ids from the closed
  complete-V1 inventory; absent is `[]`.

## Outputs

The `DefinitionCatalog` passed to spine compile as `ProfileCatalogInput`
(FR-110, FR-354), in which every entry is a `Definition` with role
`DefinitionRole::Extension`, its exact document bytes, its dependency edges
and its capability ids; or a typed refusal.

## Behavior

- The request reader SHALL decode `extensions` under FR-100's closed-envelope
  rules: a `null` member, an entry that is not an object, an unknown entry
  member, a missing `authority`, `identity` or `file`, or a blank label
  refuses `invalid-request` at stage `request`, exit 20, naming the member,
  before any dependent file is read.
- The command SHALL read each entry's `file` as a dependent file of the
  category `extensions`, charged against the request's `dependent_bytes`
  ceiling and the selected-file count exactly as program, model and
  observation files are (FR-026), in entry order after the program and
  library sources.
- The command SHALL build every entry as a `Definition` with role
  `DefinitionRole::Extension` from the exact document bytes, and SHALL build
  the catalog from all entries. Two entries with one `{authority, identity}`
  refuse `invalid_package`/`conflicting-definition` naming both entry
  indices, before any compile.
- The catalog SHALL apply to every unit the request compiles, the program
  and each supplied library alike; a unit that selects no extension compiles
  exactly as it does without the member (FSpec FR-133).
- When a unit's header selects an identity that no entry supplies and no
  layer has, E3 SHALL refuse `unknown_profile`/`unsupported-selection` at the
  identity literal's span (FR-110). When an entry's `depends_on` names a
  definition no entry supplies, the catalog link SHALL refuse
  `missing_import`/`missing-selection` naming the edge (FR-111).
- Definition validation SHALL follow FSpec FR-467 "Validation" through
  FR-354 "Definition validation": the first violated rule of
  `missing-member`, `conflicting-definition` and `definition-cycle` refuses
  `invalid_package` naming the definitions involved, at stage `compile`,
  exit 20, and no later rule is reported.
- A unit that uses an extension form without selecting the extension SHALL
  keep the existing refusal of an unselected form (FR-354 "Admission"),
  whether or not the request supplies the definition.
- The `parse` and `check` operations SHALL take an optional catalog and SHALL
  hold and parse with its selected extensions exactly as spine compile does
  (FR-354 steps 1 to 3); without a catalog they behave as today.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-660-AC-1 | A `native-compile/1` request whose `extensions` member supplies `agent-ix`/`ext.retention` with FSpec FR-467-AC-1's `retention` document, over a `1-draft` unit whose header selects `profile r = "ext.retention";` and which declares `retention Audit { subject: "audit", days: 30 }`, compiles with exit 0, and the package holds one `extension` node of node kind `retention` and a `definition_selections` row `{agent-ix, ext.retention}`. The same unit run through a `native-run/1` clause or function request with the same member compiles the same package. | Test (TC-1740) |
| FR-660-AC-2 | The AC-1 unit compiled with no `extensions` member, and with an `extensions` member that supplies only `agent-ix`/`ext.other`, each refuse `unknown_profile`/`unsupported-selection` at the span of the literal `"ext.retention"`, exit 20. | Test (TC-1741) |
| FR-660-AC-3 | With AC-1's request, a definition document missing `typed_node_schema` refuses `invalid_package`/`missing-member` naming `ext.retention` and the member; a production whose leading keyword is `function` refuses `invalid_package`/`conflicting-definition` naming `ext.retention`, the core grammar and `function`; two entries `ext.a` and `ext.b` whose `depends_on` name each other, both selected, refuse `invalid_package`/`definition-cycle` naming both in path order; each exits 20 at stage `compile`. | Test (TC-1741) |
| FR-660-AC-4 | An `extensions` member that is `null`, an entry with an unknown member `version`, an entry missing `file`, an entry with a blank `identity`, and two entries with one `{authority, identity}` each refuse before any compile: the first four `invalid-request` at stage `request` naming the member, the last `invalid_package`/`conflicting-definition` naming both entry indices; with every program and model file absent, none of them reads a file other than the request. | Test (TC-1741) |
| FR-660-AC-5 | An entry whose `file` makes the request's dependent files one byte over `dependent_bytes` refuses naming `dependent_bytes`, and with the ceiling raised to the total the same request compiles. | Test (TC-1741) |
| FR-660-AC-6 | The `parse` operation over the AC-1 unit with a catalog holding `ext.retention` returns an admissible `ParsedSource` holding one `retention` form; without the catalog the form refuses as an unselected extension form. | Test (TC-1740) |

## Dependencies

- [FR-354](FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md): the compile path that reads held definitions.
- [FR-110](FR-110-resolve-header-profile-selections-at-e3.md): the header selection of an extension.
- [FR-111](FR-111-link-a-complete-v1-definition-bundle.md): the catalog and the definition identity.
- [FR-027](FR-027-export-compiled-native-package.md), [FR-100](FR-100-run-a-named-function-through-the-spine.md): the request envelopes that carry the member.
- FSpec FR-467: the definition document's members; FSpec FR-562: the entry form and the CLI option that names the same entries.
