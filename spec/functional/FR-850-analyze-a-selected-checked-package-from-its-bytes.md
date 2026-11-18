---
id: FR-850
title: "Analyze a selected checked package from its bytes without compiling source"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-517
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-665
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-531
    type: references
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-520
    type: depends_on
---
# FR-850: Analyze a selected checked package from its bytes without compiling source

## Description

A `native-analyze/1` request MAY select an existing
`filament.checked-package/v2` artifact through its `package` member instead
of a `program` source (FSpec FR-520). The selection names the FR-517 handoff
directory that holds the package. When it does, FSL SHALL admit the
directory's `checked-package.json` with the strict checked-package reader,
attach the identity rows of its `identities.json` and, when present, the
equality rows of its `equality.json`, build from those files
alone the analysis input the engines take, and analyze
the request's items over it. No source is compiled, and the items, records,
counterexamples, replays and exit code SHALL equal those of the same request
over the source the package was compiled from.

### Decision: build the analysis input from the package bytes, never recompile

A caller who already holds a checked package, such as a lane that receives a
handoff directory or a CI step that compiled once and analyzes many times,
has the package and may not have the source. Recompiling a provisioned
source and comparing bytes would make the source mandatory, and would make
the package selection a second spelling of the program selection. The v2
package already carries everything analysis reads: the package identity,
the formula graph, the total source map, each named declaration's
`declaration.qualified_name`, the `temporal_clause_names` rows, the profile
and clock rows, the dependency selections and the identities of the domain
packages it was compiled against (FSpec FR-322). So the package is the
analysis input, and the source is never consulted. This follows the rule
FR-531 states for evaluation from a package read from its bytes alone.

### Decision: the selection names the handoff directory, whose identity file supplies the witness

The package route's reader projection (`BarePackage::world()`, the bare
walker and observation admission) needs the identity witness: the selected
declaration and field identity rows and the final source rows that join the
package's nodes to their exact identities. FR-531 makes the package bytes
carry no such rows and admits them only from the handoff's `identities.json`
(and the equality rows only from its `equality.json`), by no other route. A
selection of the package file alone therefore cannot build the projection,
and the package forms of the source-checked evaluators (FR-940) could not
run. The selection names the directory FR-517 writes instead, so the
analysis reads the package and its witness files from the one artifact that
was written together, and the rows still come only from the handoff files:
nothing is derived from the package bytes, a display name or source text. A
caller obtains the directory from a `native-compile/1` request's `handoff`
member (FR-665).

## Inputs

- The request's `package` member `{directory}`: the path of an FR-517
  handoff directory, resolved against the request document's directory. The
  analysis reads from it `checked-package.json`, `identities.json` and, when
  present, `equality.json`, and no other file; each file read is counted
  toward `dependent_bytes` (FR-026).
- The request's `models`: the domain package documents the package records
  in its model selections, each matched to its recorded selection by
  identity.
- The request's `libraries`: absent or empty. A checked package already
  holds its dependency selections; a library selection beside `package` has
  no source to compile and refuses as below.
- Every other FR-620 request member (`documents`, `memory`, `subject`,
  `items`, `limits`, `deadline_ms`) with FR-620's meaning.

## Outputs

FR-621's `analyze` outcome document. Its `request` echo carries the
`package` selection where a source request carries `program`; every other
member is the same as the source route gives.

## Behavior

### Admission

- The bytes of the directory's `checked-package.json` SHALL be read by the
  strict checked-package reader FR-027 uses for a run request's package
  selection, with no fallback to source compilation.
- The directory's `identities.json` SHALL be read by
  `ReadIdentityRows::read(bytes, package)` against the admitted package, and,
  when the directory holds `equality.json`, that file by
  `ReadEqualityRows::read(bytes, package)` (FR-531). The rows SHALL be
  attached to the package by `BarePackage::with_supply`, under one
  `NativeJoinBudget` built from the `i2.work_units` and
  `i2.native_join_storage` settings (FR-255) at their published defaults,
  since an analyze request states no native join limits (FR-531 "The native
  join budget"). No row SHALL be derived from the package bytes or any other
  file.
- A `directory` that does not exist or holds no `checked-package.json`
  SHALL refuse the whole request at the request stage, `invalid_package`,
  cause `missing-file`, naming the missing path, exit 20.
- A directory that holds no `identities.json` SHALL refuse the whole request
  at the request stage, `invalid_package`, cause `missing-file`, naming
  `identities.json`, exit 20, before any item is resolved. An identity file
  that `ReadIdentityRows::read` refuses SHALL refuse the request
  `invalid_package` with the cause of its `IdentityFileRefusal`, naming
  `identities.json`. The refusal is a closed set of seven variants, and each
  maps to one cause:

  | `IdentityFileRefusal` | Cause | Retained |
  | --- | --- | --- |
  | `Malformed` (not JSON, not the identity file's shape, an unknown or repeated member) | `malformed-file` | the member the parse stopped at |
  | `Format` (another `format` string) | `unknown-format` | that string |
  | `Unjoined` (a row the package does not join) | `unjoined-row` | the member and index of the first such row |
  | `Native` (IR refused the producer rows at its typed structural locus) | `native-witness` | the native code and structural locus |
  | `SourceObject` (IR refused the source-object rows against the admitted graph) | `native-witness` | IR's source-object error |
  | `SourceOperation` (IR refused the source-operation rows against the admitted graph) | `native-witness` | IR's source-operation member error |
  | `Rows` (a producer adapter denial with its original locus) | `native-witness` | the adapter error and its locus |

  The last four each refuse the witness rows the file carries, by Contract IR
  or by the producer adapter, so they share the one cause and differ in what
  they retain. The
  causes name the file defect itself, so a reader tells a missing witness
  from a foreign, mis-joined or IR-refused one without reading a message.
- A present `equality.json` that `ReadEqualityRows::read` refuses SHALL
  refuse the request `invalid_package`, cause `refused-file`, naming
  `equality.json` and retaining the `EqualityFileRefusal` variant
  (`Malformed`, `Format` or `Unjoined`) and its first row. An absent
  `equality.json` is not a refusal: the supply carries no equality rows,
  and a protocol-slot call that needs them refuses as FR-533 states.
- A denied native join charge SHALL refuse the request
  `stage_limit_exceeded`, naming its setting `i2.work_units` or
  `i2.native_join_storage` (FR-531 "The native join budget"), exit 20,
  before any engine runs.
- Bytes that are not JSON, are not a `filament.checked-package/v2` document,
  or that the reader refuses (a foreign `format`, an unknown member, a broken
  graph or source-map invariant) SHALL refuse the whole request at the
  request stage with category refusal, code `invalid_package`, the reader's
  typed cause and the JSON pointer it names, exit 20, before any item is
  resolved and before any engine runs.
- A request carrying both `package` and `program`, or `package` with a
  non-empty `libraries`, `refinement_internals` or `integer_division`,
  SHALL refuse the whole request `invalid-request` at the request stage,
  exit 20, naming the second member. These members configure a compile, and
  the package was already compiled.
- Each domain package identity the package's model selections record SHALL
  be supplied by exactly one `models` entry whose identity equals it. A
  recorded identity with no entry refuses the request
  `missing_import`/`missing-selection` naming that identity; an entry whose
  identity the package does not record, or whose document identity matches
  but whose content the package's recorded selection does not admit, refuses
  `invalid_package`/`conflicting-definition` naming the entry; each refusal
  is at the request stage, exit 20, before any engine runs.

### Building the analysis input

- FSL SHALL build the analysis input from the admitted package value as
  FR-940 states: a `BarePackage` (FR-531) over the read `CheckedPackageV2`,
  with the supplied domain packages bound to its model selections and the
  handoff's identity and equality rows attached ("Admission"), read by the
  engines through the sealed `AnalysisView`. It is never a `CheckedPackage`:
  FR-087 R-10 keeps checked typestate to S3 and S4. The source route reads
  each fact of FR-531's table through the same view, as the
  `AdmittedPackageRead` of its own S4 emission, and reads the source-checked
  values of FR-531's inventory from its `CheckedGraph`, while the package route
  reads them through their package forms (FR-940), so every engine, the
  default-item rule, the subject binding and the replay of a counterexample
  read one value per fact on both routes.
- An item's `claim` SHALL resolve by name against the package's name tables
  only: a temporal clause through its `temporal_clause_names` row, and an
  invariant, precondition or postcondition state clause, a refinement and
  every non-temporal claim (state-graph, hyperproperty, probabilistic)
  through its `claim_names` row (FSpec FR-322 "Claim declaration names"), a
  bare name matching the last `qualified_name` segment of exactly one row and
  a qualified name the whole path. The row's (`declaration`, `occurrence`)
  gives the occurrence key `ClaimItem::new` takes, as FR-620 states. A name
  no row carries settles the item `invalid-request` with
  `invalid_capability`/`unknown-claim`, as the source route settles it. The
  graph's state-clause nodes are identified by structure and carry no name,
  so a name is never read from a node body, a source region or a `NodeRef`
  path.
- The default items of a request with no `items` member SHALL be FR-620's
  default items, in the order the package's source map gives each claim's
  declaration span, so the order equals the source route's order.
- Counterexample replay (FR-281 "Replay") SHALL run over the same built
  package. Its replay target names the claim by the same qualified name the
  source route names.

### Equality with the source route

- For a package `P` compiled from a request's program, models and libraries,
  the outcome of the request with `package` selecting `P`'s bytes SHALL equal
  the outcome of the request over the source, member for member, except for
  the `request` echo's selection member.
- Analysis of a package selection performs no parse, select or check stage:
  the outcome's `last_stage` for a whole-request refusal before any item is
  `null`, and an item's record never carries a source-stage diagnostic.

## Acceptance Criteria

The Counter fixture is FR-620's.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-850-AC-1 | The Counter program with the claims `Bounded` and `BelowThree` is compiled by a `native-compile/1` request carrying the `handoff` member (FR-665) into its handoff directory; a `native-analyze/1` request that selects that directory as `package` `{"directory": ...}`, with the same `models`, `documents` and `items` `[{"claim": "Bounded"}, {"claim": "BelowThree"}]` and no `program`, gives an outcome equal to the source request's outcome except for the `request` selection member, and the same exit code; the same holds with no `items` member, the default items in the same order. | Test (TC-2410) |
| FR-850-AC-2 | Over the handoff directory of AC-1 with a claim `AtMostTwo` (`always holds(c.value <= 2)`) added to the compiled program, the package route refutes `AtMostTwo` with the same counterexample, the same replay target and `reproduced` `true` as the source route. | Test (TC-2410) |
| FR-850-AC-3 | AC-1's handoff directory whose `checked-package.json` holds `{}`, one holding a `filament.checked-package/v1` document, one holding AC-1's bytes with one source-map entry removed, and one that is not JSON each refuse the whole request at the request stage with code `invalid_package`, the reader's cause and pointer, exit 20, `items` empty, and no engine runs. | Test (TC-2410) |
| FR-850-AC-4 | AC-1's package request with `program` also present, with a `libraries` entry, and with `integer_division` `floor` each refuse `invalid-request` at the request stage naming that member, exit 20; with `models` empty it refuses `missing_import`/`missing-selection` naming `test/counter`; with an extra `models` entry the package does not record it refuses `invalid_package`/`conflicting-definition` naming that entry. | Test (TC-2410) |
| FR-850-AC-5 | AC-1's package request with the item `{"claim": "Missing"}` settles that item `invalid-request` with `invalid_capability`/`unknown-claim` naming `Missing`, as the source route does. | Test (TC-2410) |
| FR-850-AC-6 | AC-1's handoff directory with `identities.json` removed refuses the whole request at the request stage, `invalid_package`, cause `missing-file`, naming `identities.json`, exit 20, `items` empty and no engine run; with `identities.json` replaced by `{}` it refuses `invalid_package`, `malformed-file`, naming `identities.json`; with its `format` changed it refuses `invalid_package`, `unknown-format`; with `equality.json` removed the request analyzes as AC-1 states; with `equality.json` replaced by `not json` it refuses `invalid_package`, `refused-file`, naming `equality.json`; a `package` `{"directory": "absent"}` refuses `invalid_package`, `missing-file`, naming the path. | Test (TC-2966) |
| FR-850-AC-7 | Over the handoff directory of FR-940-AC-4's Counter unit, a `package` request with the items `{"claim": "NotAboveThree"}` (an invariant state clause) and `{"claim": "ReachThree"}` (a `possible` state-graph claim) resolves each through its `claim_names` row and settles it with the source route's record; `{"claim": "Missing"}` settles `invalid-request` `unknown-claim`; the same directory whose `checked-package.json` has the `NotAboveThree` row removed refuses `invalid_package`/`invalid-value` at `/claim_names`, and one with two rows of one `qualified_name` refuses `ambiguous_declaration`/`ambiguous-name`, each before any engine runs. | Test (TC-2967) |

## Dependencies

- FR-620 owns the request document and its other members; this requirement
  states its `package` selection.
- FR-027 owns the strict checked-package reader; FSpec FR-322 owns the v2
  package and its rows.
- FR-281 owns analysis, negotiation, settlement and replay over the built
  package.
- FR-517 owns the handoff directory and FR-665 its `handoff` compile
  member; FR-531 owns the identity and equality files, their strict readers
  and the native join budget.
