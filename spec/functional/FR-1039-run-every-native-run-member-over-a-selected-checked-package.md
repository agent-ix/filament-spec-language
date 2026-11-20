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
  - target: ix://agent-ix/filament-specification/FR-825
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-961
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-531
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-533
    type: references
  - target: ix://agent-ix/filament-specification/FR-323
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

- `package`: exactly one of `{"file": P}` or `{"file": P, "equality": E}`,
  `P` the path of a checked-package v2 file (FR-945) and `E` the path of
  that package's equality witness file (an FR-531 equality file, the
  `equality.json` an FR-517 handoff writes), or `{"directory": D}`, the
  path of an FR-517 handoff directory holding `checked-package.json`,
  `identities.json` and, when present, `equality.json` (FR-850), each path
  resolved against the request file's directory, each file read counted
  toward `dependent_bytes`. `equality` is optional, a string, and admitted
  on the file form only. Beside `file`
  or `directory` the selection MAY carry `source_package`, the closed object
  `{"identity": I, "version": V}` with `I` and `V` non-empty strings: the
  source package reference the caller states for the package ("The source
  package reference of a package export" below). An `export` over a package
  requires it; every other member ignores it after its shape is read.
- `models`: entries in the run request's admitted formats,
  `semantic-ir/2.0.0` documents and `native-model-unit/1` units compiled to
  their documents (FR-710), each matched by identity to one of the package's
  recorded model selections.
- Exactly one run member: `call`, `clause`, `items`, `synthesis`,
  `verification_plan` (with its `document`, FSpec FR-824), `export` or
  `tl_lowering`. `tl_lowering` is FR-931's member: an object with exactly the
  members `clause` (required), `correspondence` (optional) and
  `cross_evaluate` (optional), decoded under FR-026's closed-envelope rules
  and refused as FR-931 "The member" states.

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
- A `tl_lowering` member over a package SHALL lower the temporal clause that
  `tl_lowering.clause` names, resolved through `temporal_clause_names`
  (FR-850), under the correspondence it names or the clause's default, and
  SHALL return the record the source route returns for the same unit and
  member (FR-931 "Atom text": both routes print byte-equal atom text). It
  SHALL NOT refuse the member as unimplemented, and SHALL NOT run another
  member in its place.
- A `{directory}` selection SHALL attach the handoff's identity and equality
  rows as FR-850 "Admission" states. A member whose run builds states over
  the package's model world (a `clause` run, a plan synthesis, a
  `verification_plan` with a step over a state model) needs those rows; with
  a `{file}` selection it SHALL refuse `invalid-request` at stage `request`
  with `details` `{"member": "<member>", "requires": "directory"}`, exit 20.
  A `call`, a value `synthesis` and an `export` run from either form.
- A `{file, equality}` selection SHALL read the equality rows from the
  `equality` file through `ReadEqualityRows::read(bytes, package)` against
  the admitted package, the same admission, with the same checks that the
  rows belong to that package, that a `{directory}` selection applies to its
  `equality.json` (FR-850 "Admission", FR-531), and SHALL attach them to the
  package. A `{file}` selection with no `equality` carries no equality rows.
  The rows come only from the producer's witness file and are never derived
  from the package bytes: FR-850, FR-531 and FR-533 are unchanged.
- A `{file}` selection with no `equality`, over a package that holds an
  equality application (its `checked_term_data.equality_decisions` is
  non-empty, FSpec FR-322), SHALL refuse the whole request at stage
  `request` with FR-850's code for the missing witness, `invalid_package`,
  cause `missing-file`, naming `/request/package/equality`, exit 20, and
  write no member outcome (the bytes-only `Equality(Missing)` refusal,
  FR-533). A package with no equality application runs from `{file}` alone.

### A package that cannot run

Each refusal is at stage `request`, exit 20, with no member outcome, and
none falls back to source:

| Defect | Code / cause | Locus or `details` |
| --- | --- | --- |
| the selected file or directory is missing or unreadable | `invalid-request` | names `package` and the path |
| the `equality` file is missing or unreadable | `invalid-request` | names `package.equality` and the path |
| `equality` beside `directory`, or not a string | `invalid-request` | names `package.equality`; refused before any selected file is opened |
| `ReadEqualityRows::read` refuses the `equality` file (`Malformed`, `Format` or `Unjoined`) | `invalid_package`, cause `refused-file`, retaining the `EqualityFileRefusal` variant and its first row (FR-850) | names `package.equality` and the path |
| no `equality` on the file form over a package that holds an equality application | `invalid_package`, cause `missing-file` (FR-850) | `/request/package/equality` |
| `contract_version` is not `filament.checked-package/v2` | `unknown_wire`/`unsupported-wire`, `actual` the version read, `expected` `filament.checked-package/v2` | `/request/package/contract_version` |
| the strict reader refuses the bytes | `invalid_package` with the reader's cause | the reader's pointer, under `/request/package` |
| the `lock` edition is not `1-draft` | `unknown_edition` (stage `profile`) | names the package and the edition |
| a recorded model selection has no matching `models` entry | `missing_import`/`missing-selection` | `/request/models`, naming the identity |
| a `models` entry matches no recorded model selection | `invalid-request` | `/request/models/<i>`, naming its identity |
| the member's target is absent from the package's name rows | the member's own refusal (FR-945, FR-850, FR-1037, FSpec FR-824) | the member's pointer |

### The requirement items of a package call

A `call` carrying `requirements` (FR-960) over a package SHALL settle the
called function's requirement items by FR-961's steps 4 to 6, with FR-395's
items read from the package and not from a compile:

- The items of the call SHALL be the package's FSpec FR-825
  `value-validity` rows whose record's `owner` is the function node the call
  resolves through `declaration.qualified_name` (FR-945), in package order,
  each with its row's position in `requirement_items` as its request index
  (FR-531 "Requirement records in the bytes", `BarePackage::call_requirements`).
  A function that owns no row has no items, and its `requirements` member is
  `[]`, as on the source route.
- Each item's claim, kind, extent, result bound, path condition and the rest
  of its FR-331 request item SHALL be read from its row and record node
  (FR-940 "Requirement records"), and the item SHALL be negotiated, settled
  and replayed through the same path as on the source route, its process
  provider requests carrying the admitted bytes of the selected package as
  `package`. An `items` entry whose claim names no item of the call refuses
  `UnknownRequirementClaim` before any item is negotiated, as on the source
  route.
- The `requirements` and `registry` members of the `spine-run-result/1`
  document, the `RunResult.requirements` field and the `execute` outcome's
  `items` SHALL equal, byte for byte, the source route's for the same request
  over the source the package was compiled from. A package whose
  `requirement_items` the strict reader refuses (a row removed, a record node
  without its row) refuses at stage `request` as "A package that cannot run"
  states for a reader refusal, and never settles a shorter item list.

### Decision: the package carries the records, the route does not rebuild them

The source route takes a call's items from the checker's requirement
records: the owner it recorded, the package order of every record, each
record's capability and extent, and each claim site's bound, guards and
roots. None of these is a name row or a `state_clause` node: a
`value-validity` record belongs to a scalar application inside a function
body, and an operation contract's `state_clause` node has no function owner.
Rebuilding them from the graph would re-run the checker's ownership,
guard and extent classification on the package route, and answering with
the records that happen to be derivable would be a partial map. So the
package carries them (FSpec FR-825) and the package route reads them, as it
reads every other fact of FR-531's table.

### The source package reference of a package export

FSpec FR-817's export outcome, and FSpec FR-299's package key, carry
`package.source_package`, the identity and version of the source package
the export was made from. A source export takes it from the source: the
identity the program source declares and the program selection's
`formal_revision` (FR-1015). A checked package does not record the program
selection's `document` or `formal_revision` (FR-027), so the package route
cannot derive the version, and it SHALL NOT invent one, reread or compile
source to find one, or take one from any other member of the package.

- An `export` request over a package SHALL carry
  `package.source_package`. The package route SHALL write the outcome's
  `package.source_package` equal to that pair as given, byte for byte, and
  read nothing else for it. FSpec FR-299, FSpec FR-817 and the FR-027 wire
  are unchanged: the reference has the same shape on both routes, and only
  its origin differs.
- The request reader SHALL check `source_package` whenever it is present,
  before it opens any selected file, and refuse at stage `request`, exit
  20, disposition `invalid-request`, with code `invalid_runtime_input`, the
  cause given and `details` `{"path": P}`:

| Defect | Cause | `P` |
| --- | --- | --- |
| an `export` request over a package with no `source_package` member | `missing-member` | `/request/package/source_package` |
| `source_package` not an object, `null` included | `wrong-value-kind` | `/request/package/source_package` |
| `identity` or `version` absent | `missing-member` | `/request/package/source_package/identity` or `/version` |
| `identity` or `version` not a string | `wrong-value-kind` | that member's pointer |
| `identity` or `version` the empty string | `invalid-value` | that member's pointer |
| any other member | `unknown-member` | that member's pointer |

- A request over a package whose run member is not `export` runs as
  before whether or not it carries `source_package`; a well-formed one is
  echoed in the request echo and read by nothing else.

### Decision: the caller states the source reference of a package

The export outcome needs a source version that the package does not hold.
Adding the program selection's labels to the checked package would change
the FR-027 wire, which FR-027 keeps free of them so that two compiles that
differ only in those labels write identical bytes. Deriving a version from
a digest or from the package identity would put a value in the outcome that
no one stated. The caller who selects a package knows which source it came
from, so the request carries that pair, and the outcome repeats exactly what
was stated.

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
| FR-1039-AC-2 | The FR-517 handoff of FR-754-AC-3's `Pick` unit (whose objective `Pick * Pick = 49` is an equality application) selected as `package` `{directory}`, and its `checked-package.json` and `equality.json` selected as `{"file": "handoff/checked-package.json", "equality": "handoff/equality.json"}`, each run the `synthesis` member `{"declaration": "Pick"}` to the source route's `candidate` 7, exit 0; the same package selected as `{"file": "handoff/checked-package.json"}` alone refuses at stage `request` `invalid_package`/`missing-file` naming `/request/package/equality`, exit 20, and writes no synthesis outcome; `{file, equality}` naming the `equality.json` of another unit's handoff refuses `invalid_package`/`refused-file` with the `Unjoined` variant; `{"directory": "handoff", "equality": "handoff/equality.json"}` refuses `invalid-request` naming `package.equality` before any file is opened; a `clause` member over a `{file}` selection refuses `invalid-request` with `details` `{"member": "clause", "requires": "directory"}`, exit 20. | Test (TC-3019) |
| FR-1039-AC-3 | Each defect of "A package that cannot run" refuses at stage `request`, exit 20, with the code and locus its row states: a missing file; a document whose `contract_version` is `filament.checked-package/v1` (`unknown_wire`/`unsupported-wire` with `actual` `filament.checked-package/v1` at `/request/package/contract_version`, never `invalid_package`); a package with an unknown top-level member (`invalid_package` at the reader's pointer); a spine-model package run with no `models` entry for its recorded selection (`missing_import`/`missing-selection` at `/request/models`); and the value package run with one `models` entry, the spine document (`invalid-request` at `/request/models/0`). | Test (TC-3019) |
| FR-1039-AC-4 | FR-1015-AC-4's counter compiled to a package file and exported over `package` `{"file": P, "source_package": {"identity": "example/counter-unit", "version": "7"}}`, with the source file removed, writes `package.source_package` `{"identity": "example/counter-unit", "version": "7"}` and otherwise the source export's `records` and `target_text`; the same request without `source_package` refuses `invalid-request` with `invalid_runtime_input`/`missing-member` at `/request/package/source_package`, exit 20, before the package file is opened (a request naming a missing file gives this refusal, not the missing-file one); `"source_package": null` and `"source_package": "x"` each refuse `wrong-value-kind` at `/request/package/source_package`, `{"identity": "a"}` `missing-member` at `/request/package/source_package/version`, `{"identity": 7, "version": "1"}` `wrong-value-kind` at `/request/package/source_package/identity`, `{"identity": "", "version": "1"}` `invalid-value` there, and an extra member `digest` `unknown-member` at `/request/package/source_package/digest`, each exit 20; FR-1039-AC-1's `call` request over the package runs to `-4` with and without a well-formed `source_package`. | Test (TC-3019) |
| FR-1039-AC-5 | FR-961-AC-2's `Even` request with `call.requirements` `{}` and no providers, and FR-062's RR-12 unit called as `f` with `n = 4` and `requirements` `{}`, each compiled to a package file and run over `package` `{"file": P}` with the source file removed, write `requirements` and `registry` members equal byte for byte to the source route's (the same request indices, claims, extents, path conditions, records and categories), and the `execute` outcome's `items` equal the source route's; RR-12's `g` called over the package writes `requirements` `[]`; an `items` entry naming a claim of `f` in a call of `g` refuses `UnknownRequirementClaim` before any item is negotiated, as on the source route; the package with the row of `f`'s `+` record removed refuses at stage `request`, exit 20, `invalid_package`/`invalid-value` at `/request/package/semantic_graph/nodes/{n}/body` (FSpec FR-825 rule 14), and settles no item; no request over the package refuses `call.requirements` as not implemented. | Test (TC-3019) |
| FR-1039-AC-6 | FR-931-AC-1's `Low` unit compiled to a package file and selected as `package` `{"file": P}`, with the source file removed, runs `tl_lowering` `{"clause": "Low"}` and `{"clause": "Low", "correspondence": "mltl.online-prefix/v1"}` and returns the records the source route returns for the same members, byte-equal, exit 0; `{"clause": "Low", "correspondence": "mltl.origin-complete-history/v1"}` returns the source route's `unsupported` record, exit 21; `{"clause": "Nothing"}` refuses `missing_declaration`/`missing-name` naming `tl_lowering.clause`, exit 20; `{"clause": "Low", "formula": 1}` refuses `invalid-request` at stage `request` naming `tl_lowering.formula`, exit 20; and `tl_lowering` beside `call` refuses `invalid-request` naming `tl_lowering`, exit 20. No response is the not-implemented refusal, and none is a `call` or `clause` outcome. | Test (TC-3019) |

## Dependencies

- FR-100 (the run request); FR-945 (the function call); FR-850 and FR-940
  (the handoff directory and read view); FR-767 (the recorded law); FSpec
  FR-322 (the lock and name rows).
