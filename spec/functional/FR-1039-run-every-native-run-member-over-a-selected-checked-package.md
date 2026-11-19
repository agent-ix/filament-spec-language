---
id: FR-1039
title: "Run every native-run member over a selected checked package without source"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-945
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-767
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
---
# FR-1039: Run every native-run member over a selected checked package without source

## Description

A `native-run/1` request MAY select an existing `filament.checked-package/v2`
artifact through its `package` member in place of a `program`. FSL SHALL
then admit the package with the strict checked-package reader, take every
selection a source run reads from its header and bindings from the
package's own `lock` and name rows, run the request's member over the
admitted package through FR-940's read view, and never compile source or
fall back to it. The outcome SHALL equal the outcome of the same request over
the source the package was compiled from.

### Decision: the package's records replace the source and its bindings

A source run reads its edition from the program header, its profiles,
dependencies and integer-division law from its declarations and request
members, and its names from the checked source. A checked package records
all of these: its `lock` holds the edition, the ordered profiles, the
dependency selections and the law definitions (the integer-division law
included); its `model_selections` hold the domain packages it was compiled
against; `declaration.qualified_name`, `claim_names` and
`temporal_clause_names` hold every name a request selects. So the package
route reads each from the package, as FR-850 does for analysis, and a
request member that would restate one of them (a `program`, `libraries`,
`refinement_internals`, `integer_division`) is refused rather than reconciled.

## Inputs

- `package`: exactly one of `{"file": P}`, the path of a checked-package v2
  file (FR-945), or `{"directory": D}`, the path of an FR-517 handoff
  directory holding `checked-package.json`, `identities.json` and, when
  present, `equality.json` (FR-850), resolved against the request file's
  directory, each file read counted toward `dependent_bytes`.
- `models`: entries in the run request's admitted formats,
  `semantic-ir/2.0.0` documents and `native-model-unit/1` units compiled to
  their documents (FR-710), each matched by identity to one of the package's
  recorded model selections.
- Exactly one run member: `call`, `clause`, `items`, `synthesis`,
  `verification_plan` (with its `document`, FSpec FR-824) or `export`.

## Outputs

The member's own outcome document, equal to the source route's, with the
request echo carrying `package` where a source request carries `program`.

## Behavior

- The request reader SHALL refuse, before it opens any selected file, a
  request carrying neither `program` nor `package` (`invalid-request` naming
  `program`), both (naming `program`), or `package` beside `libraries`,
  `refinement_internals` or `integer_division` (naming that member), each at
  stage `request`, exit 20.
- FSL SHALL take the edition, profiles, dependency selections and laws from
  the admitted package's `lock`, so a call evaluates `div` and `rem` under
  the law the package recorded (FR-767), and SHALL resolve every name a
  member selects only through the package's name rows: a function through
  `declaration.qualified_name` (FR-945), a clause through
  `temporal_clause_names` or `claim_names` (FR-850), a synthesis declaration
  through `claim_names` (FR-1037), a plan through the supplied plan
  document (FSpec FR-824).
- A `{directory}` selection SHALL attach the handoff's identity and equality
  rows as FR-850 "Admission" states. A member whose run builds states over
  the package's model world (a `clause` run, a plan synthesis, a
  `verification_plan` with a step over a state model) needs those rows; with
  a `{file}` selection it SHALL refuse `invalid-request` at stage `request`
  with `details` `{"member": "<member>", "requires": "directory"}`, exit 20.
  A `call`, a value `synthesis` and an `export` run from either form.

### A package that cannot run

Each refusal is at stage `request`, exit 20, with no member outcome, and
none falls back to source:

| Defect | Code / cause | Locus or `details` |
| --- | --- | --- |
| the selected file or directory is missing or unreadable | `invalid-request` | names `package` and the path |
| `contract_version` is not `filament.checked-package/v2` | `unknown_wire`/`unsupported-wire`, `actual` the version read, `expected` `filament.checked-package/v2` | `/request/package/contract_version` |
| the strict reader refuses the bytes | `invalid_package` with the reader's cause | the reader's pointer, under `/request/package` |
| the `lock` edition is not `1-draft` | `unknown_edition` (stage `profile`) | names the package and the edition |
| a recorded model selection has no matching `models` entry | `missing_import`/`missing-selection` | `/request/models`, naming the identity |
| a `models` entry matches no recorded model selection | `invalid-request` | `/request/models/<i>`, naming its identity |
| the member's target is absent from the package's name rows | the member's own refusal (FR-945, FR-850, FR-1037, FSpec FR-824) | the member's pointer |

### Decision: a wrong document type keeps the I2 reader's code

The package route reads the selected bytes through the same I2 reader as
every other checked-package consumer, so a document of another type refuses
with the code FR-096 "The I2 reader locates its refusals in the artifact"
gives that defect: `unknown_wire`/`unsupported-wire` with `actual` and
`expected`. Contract IR's reader names the defect `unknown_contract_version`
(FSpec FR-271, FR-760); FSL's diagnostic catalogue carries it as
`unknown_wire`, and the package route does not add a second FSL code or an
`invalid_package` cause for the same defect. The locus is the reader's
pointer `/contract_version` under `/request/package`, as for every other
reader refusal in the table.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1039-AC-1 | A unit declaring `function half using v(): Integer pure { -7 div 2 }`, compiled with `integer_division` `floor` and selected as `package` `{file}` with the `call` `half`, completes with `-4`, the source route's value under the recorded law, with the source file removed before the run; the same request with `integer_division` refuses `invalid-request` naming `integer_division` before any selected file is opened; a request with neither `program` nor `package` refuses `invalid-request` naming `program`. | Test (TC-3019) |
| FR-1039-AC-2 | The FR-517 handoff of FR-754-AC-3's `Pick` unit selected as `package` `{directory}`, and its package file selected as `{file}`, each run the `synthesis` member `{"declaration": "Pick"}` to the source route's `candidate` 7; a `clause` member over a `{file}` selection refuses `invalid-request` with `details` `{"member": "clause", "requires": "directory"}`, exit 20. | Test (TC-3019) |
| FR-1039-AC-3 | Each defect of "A package that cannot run" refuses at stage `request`, exit 20, with the code and locus its row states: a missing file; a document whose `contract_version` is `filament.checked-package/v1` (`unknown_wire`/`unsupported-wire` with `actual` `filament.checked-package/v1` at `/request/package/contract_version`, never `invalid_package`); a package with an unknown top-level member (`invalid_package` at the reader's pointer); a spine-model package run with no `models` entry for its recorded selection (`missing_import`/`missing-selection` at `/request/models`); and the value package run with one `models` entry, the spine document (`invalid-request` at `/request/models/0`). | Test (TC-3019) |

## Dependencies

- FR-100 (the run request); FR-945 (the function call); FR-850 and FR-940
  (the handoff directory and read view); FR-767 (the recorded law); FSpec
  FR-322 (the lock and name rows).
