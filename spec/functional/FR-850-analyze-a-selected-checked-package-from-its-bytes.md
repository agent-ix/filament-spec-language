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
equality rows of its `equality.json`, admit the dependency artifact closure
of `dependencies.json` with each package's own witness files, and build from
those files alone the analysis input the engines take, and analyze
the request's items over it. No source is compiled, and the items, records,
counterexamples, replays and exit code SHALL equal those of the same request
over the source the package was compiled from.

### Decision: build the analysis input from the package bytes, never recompile

A caller who already holds a checked package, such as a lane that receives a
handoff directory or a CI step that compiled once and analyzes many times,
has the package and may not have the source. Recompiling a provisioned
source and comparing bytes would make the source mandatory, and would make
the package selection a second spelling of the program selection. The root v2
package carries its own analysis facts: the package identity,
the formula graph, the total source map, each named declaration's
`declaration.qualified_name`, the `temporal_clause_names` rows, the profile
and clock rows, the dependency selections and the identities of the domain
packages it was compiled against (FSpec FR-322). So the package is the
analysis input together with the admitted dependency artifacts and their own
witnesses from the handoff; dependency selections name those packages but do
not embed their bytes. The source is never consulted. This follows the rule
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
analysis reads the root and dependency packages and their own witness files
from the one handoff that was written together, and the rows still come only from the handoff files:
nothing is derived from the package bytes, a display name or source text. A
caller obtains the directory from a `native-compile/1` request's `handoff`
member (FR-665).

### Decision: a package-route request names a model by its package identity

On the source route a request may name a model by the alias its unit
imports it under (`M` for the Counter's model) or by its package identity
(FR-626, its Inputs). An alias belongs to the source that declares it: two units that
import the same domain package as `M` and as `N` record the same model
selection by identity, and an untimed unit compiles to package bytes that
are otherwise equal, so its bytes carry no alias. A timed model's qualified
name carries its alias (`[alias, "Timed"]`), but a model selection never
does, so no byte row names the alias as the model's name, and a package route that accepted one would
have to read it from source text the selection does not provide or guess
it. The package route therefore names a model by the identity the package
records, which is the one name both routes accept, and a request written
with identities settles the same records on both routes. A name the
package cannot resolve is told apart from an unknown claim by the cause
`model-alias`, which names the identity to write instead, so a caller
moving a request from source to package fixes it from the diagnostic
alone. The item is settled, not the request refused, because the defect
is that one item's spelling, as with `unknown-claim`.

## Inputs

- The request's `package` member `{directory}`: the path of an FR-517
  handoff directory, resolved against the request document's directory. The
  analysis reads from it `checked-package.json`, `identities.json`, when
  present `equality.json`, the required `dependencies.json`, and the package
  and witness triple of each manifest row (FR-517 "The dependency artifact
  closure"). It reads no source, selection sidecar or other handoff file.
  Each file read is counted toward aggregate `dependent_bytes` (FR-026).
- The request's `models`: the domain package documents the root or any
  dependency artifact records in its model selections, each matched to its
  recorded selection by identity. A document selected only by a dependency
  is not an extraneous root model.
- The request's `libraries`: absent or empty. A checked package already
  holds its dependency selections and obtains the selected admitted artifacts
  from its handoff manifest. A request-level `libraries` entry supplies
  library SOURCE, not a dependency artifact; it remains refused as below.
- Every other FR-620 request member (`documents`, `memory`, `subject`,
  `items`, `limits`, `deadline_ms`) with FR-620's meaning.

## Outputs

FR-621's `analyze` outcome document. Its `request` echo carries the
`package` selection where a source request carries `program`; every other
member is the same as the source route gives.

## Behavior

### Dependency artifact admission

The directory loader shared by `native-analyze/1`, `native-run/1` directory
selections ([FR-1039](FR-1039-run-every-native-run-member-over-a-selected-checked-package.md))
and their bytes-only counterexample replays SHALL consume FR-517's dependency
manifest and artifact closure. This changes no `{file}` or `{file, equality}`
request shape and adds no request-level dependency member. A bare-file selection
with no supplied dependency evidence SHALL keep its existing strict-reader
`MissingImport` refusal; it SHALL NOT search neighboring handoff files, sources
or artifact directories to repair the missing supply. The directory loader
SHALL NOT read library source, recompile a provision, or construct source `CheckedPackage` or `CheckedGraph` typestate
from artifact bytes (FR-940).

- The loader SHALL require `dependencies.json`, even when the root selects
  no library. It SHALL parse the closed `filament.checked-handoff.dependencies/1`
  format of FR-517 before admitting any item. Rows SHALL have unique identities
  in the specified order and their exact indexed directory. No additional
  path, edge, source, digest or version member is admitted.
- The loader SHALL admit each library through the existing strict checked-package
  reader with its selected domain documents and already admitted direct library
  dependencies. It SHALL perform admission bottom-up, with independent ready
  libraries visited in manifest order, then admit the root with the retained
  dependency evidence. The manifest order need not be topological. Package
  locks alone determine import edges. The reader SHALL validate each supplied
  package against its selected identity and the existing cross-package and
  diamond rules before `CheckedPackageEvidence::insert_dependency_package`
  makes it available to an importing package. No missing import is fabricated.
- For each dependency, the loader SHALL require both witness files, including
  an empty `equality.json` when the dependency has no equality application.
  It SHALL read its `identities.json` and `equality.json` against THAT admitted
  dependency, using `ReadIdentityRows::read` and `ReadEqualityRows::read`, and
  attach the resulting supply only to that dependency's package view. It SHALL
  NOT join a dependency's witness against the root, reuse the root's supply for
  it, or derive a missing witness from package bytes. The root's equality-file
  absence rule in "Admission" is unchanged.
- After the root is admitted, the manifest's identities SHALL equal the unique
  recursive closure reachable through its admitted dependency selections,
  excluding the root. A diamond shares one admitted package and its supply;
  two different packages claimed under the same library identity SHALL refuse
  the request. An extra unreachable artifact is not an alternative supply.
- The loader SHALL retain the admitted closure, original artifact bytes and
  each package's witness supply through run, analysis and counterexample replay.
  Evaluation of a dependency-owned function or premise SHALL use that owner's
  admitted package and witnesses. Replay SHALL consume the same retained
  closure without reopening source; a replay read still applies the existing
  `replay.input_bytes` bound to all package artifact bytes it consumes, and
  replay refusals keep FR-940's item-local `replay_refused` behavior.

Every manifest and artifact path SHALL be resolved beneath the selected handoff
using FR-517's confined-file rules. The loader SHALL reject absolute paths,
noncanonical indexed paths, `.` or `..` components, backslashes, symbolic links
at any component and non-regular artifact files before reading their content.
It SHALL NOT traverse an unlisted dependency directory to discover artifacts.

The following failures SHALL refuse the whole request before any item or engine
runs, with category refusal, exit `20`, `items` empty and `last_stage` `null`:

| Failure | Code / cause | Retained location |
| --- | --- | --- |
| Missing manifest or selected dependency package/witness file | `invalid_package` / `missing-file` | exact relative file path |
| Non-JSON manifest, missing/unknown/repeated member or wrong member kind | `invalid_package` / `malformed-file` | `dependencies.json` and first defective JSON pointer |
| Another manifest `format` | `invalid_package` / `unknown-format` | `dependencies.json`, `/format`, and the supplied string |
| Repeated identity, wrong order, noncanonical/unsafe path or unreachable manifest row | `invalid_package` / `invalid-value` | `dependencies.json` and first defective row/member pointer; an unsafe actual file names its relative path |
| A library selection not supplied by the manifest | `missing_import` / `missing-selection` | importing package's relative file path and the reader's dependency-selection pointer and identity |
| Conflicting admitted content under one identity or a foreign artifact supplied for a selected identity | `invalid_package` / `conflicting-definition` | relative artifact file path, selected identity and the reader's original locus when present |
| Cyclic dependency selections | `invalid_package` / `definition-cycle` | identities on the cycle and the first back-edge's dependency-selection pointer, visiting root edges and each package's edges in their recorded order |
| Refused dependency package or foreign/unjoined dependency witness | the strict reader's code/cause, or the identity/equality mapping in "Admission" | dependency-relative file path plus original reader/witness refusal and locus; no root-relative substitution |

The loader SHALL use one invocation-wide intake budget: every root, manifest,
dependency package and witness file read counts once by its full size toward
`dependent_bytes`, in addition to request-selected model and runtime documents.
It SHALL count every such selected file once toward FR-026's shared
64-selected-file ceiling, including the manifest and required dependency witness
files; a diamond's shared triple is counted once. It SHALL check a prospective
file count before opening the next file and cap each read by the remaining byte
allowance. These intake stops retain FR-026's `resource_exhausted`, category
`incomplete`, exit `22`, configured bound and option hint, rather than being
translated into a malformed-artifact refusal.

All strict package reads and native joins for the root and closure SHALL consume
one finite remaining work allowance for the invocation, without resetting it per
dependency. Native identity joins SHALL share the `NativeJoinBudget` and logical
storage allowance of `i2.work_units` and `i2.native_join_storage` (FR-531,
FR-255). Strict-reader work is charged once by its existing read schedule and
native-join work once by its join schedule; this rule adds no duplicate charge
for an operation already accounted by that schedule. A denied charge retains
its original setting, bound, attempted count and typed refusal. No item runs
on a partially admitted closure. The writer's one `native_join` budget likewise
serves the complete write, not one fresh budget per dependency.

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
  (`Malformed`, `Format` or `Unjoined`) and its first row.
- An absent `equality.json` SHALL be admitted, with a supply that carries
  no equality rows, only when the admitted package holds no equality
  application: its `checked_term_data` is absent or its
  `equality_decisions` array is empty (FSpec FR-322).
- When the admitted package holds an equality application, an absent
  `equality.json` SHALL refuse the whole request at the request stage,
  `invalid_package`, cause `missing-file`, naming `equality.json`, exit 20,
  `items` empty and no engine run, as an absent `identities.json` does.

  The equality rows are the producer's witness for every `=`, `==` and `!=`
  application (FR-533 "Equality slots"). That includes the `=` of a pre- or
  postcondition, which the package form of a state clause reads into the
  model-check input (FR-940 "Package forms of the source-checked
  evaluators"). The bare route derives no witness from the package and
  guesses no decision, so without the file it cannot build the state form
  of any operation whose state clauses hold an equality. Settling the items
  anyway would give a verdict the source route does not give. The source
  route always holds the rows of its own emission. The refusal is made once
  for the request, not per item, because the model-check input of every item
  over a subject reads all of that subject's state clauses. If it were made
  per item, which items settle would depend on which items happen to be
  requested, not on the package. FR-517 writes `equality.json` into every
  handoff directory, with empty lists when the unit holds no equality
  application. A complete handoff is therefore never refused, and the
  equality with the source route holds over every directory the writer
  produces. The bytes-only library entry keeps FR-531's per-call rule: a
  `BareSupply` whose `equality` is `None` is refused only by a slot call that
  reaches an equality application (FR-533). A `native-run/1` request that
  selects a package file rather than a directory names its witness file in
  the `package` member's `equality` (FR-1039); that file is admitted by the
  same `ReadEqualityRows::read`, and without it a package that holds an
  equality application refuses with this code and exit.
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
- Each domain package identity the root or any retained dependency's model
  selections record SHALL be supplied by exactly one `models` entry whose
  identity equals it. Each package SHALL bind that document against its own
  recorded selection. A recorded identity with no entry refuses the request
  `missing_import`/`missing-selection` naming that identity; an entry whose
  identity no package in the admitted closure records, or whose document
  identity matches but whose content a selecting package's recorded selection
  does not admit, refuses
  `invalid_package`/`conflicting-definition` naming the entry; each refusal
  is at the request stage, exit 20, before any engine runs.

### Building the analysis input

- FSL SHALL build the analysis input from the admitted package value as
  FR-940 states: a `BarePackage` (FR-531) over the read `CheckedPackageV2`,
  with the supplied domain packages bound to its model selections and the
  handoff's own identity/equality rows attached, retaining each dependency's
  admitted package and its separate supply ("Dependency artifact admission"
  and "Admission"), read by the
  engines through the sealed `AnalysisView`. It is never a `CheckedPackage`:
  FR-087 R-10 keeps checked typestate to S3 and S4. The source route reads
  each fact of FR-531's table through the same view, as the
  `AdmittedPackageRead` of its own S4 emission, and reads the source-checked
  values of FR-531's inventory from its `CheckedGraph`, while the package route
  reads them through their package forms (FR-940), so every engine, the
  default-item rule, the subject binding and the replay of a counterexample
  read one value per fact on both routes.
- An item's `claim` SHALL resolve by name against the package's name tables,
  and a derived item's model through `model_selections`, only: a temporal clause through its `temporal_clause_names` row, and an
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
- A derived item over a model subject (FSpec FR-368 "Claim and anchor of a
  derived or refinement item"), such as a deadlock-freedom item, SHALL name
  its model in `claim` by the package identity its model selection records,
  for the Counter `test/counter`. Its `claim` SHALL resolve when it equals
  the identity of exactly one of the package's model selections, and the
  item then settles with the source route's record for the same item.
- A derived item over a model subject whose `claim` names no claim row and
  no model identity the package records SHALL settle `invalid-request`
  with `invalid_capability`/`model-alias`, naming the claim and every model
  identity the package records, in model-selection order, as the name to
  use; the request's other items settle as they would without it, and the
  item never refuses the whole request, settles `unsupported` or
  falls back to source.

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
  the `request` echo's selection member, when every item that names a model
  names it by its package identity ("Building the analysis input").
- The equality holds over every member of a clause's record, its `span`
  included, with no member normalized. A package clause's span SHALL be read
  from the `declaration_region` of its `temporal_clause_names` or
  `claim_names` row (FSpec FR-322 "Clause declaration regions"), the whole
  declaration the source route's checked clause spans, and never from the
  occurrence's `source_map` region, which covers only the name token.
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
| FR-850-AC-6 | AC-1's handoff directory with `identities.json` removed refuses the whole request at the request stage, `invalid_package`, cause `missing-file`, naming `identities.json`, exit 20, `items` empty and no engine run; with `identities.json` replaced by `{}` it refuses `invalid_package`, `malformed-file`, naming `identities.json`; with its `format` changed it refuses `invalid_package`, `unknown-format`; with `equality.json` removed it refuses as AC-10 states; with `equality.json` replaced by `not json` it refuses `invalid_package`, `refused-file`, naming `equality.json`; a `package` `{"directory": "absent"}` refuses `invalid_package`, `missing-file`, naming the path. | Test (TC-2966) |
| FR-850-AC-7 | Over the handoff directory of FR-940-AC-4's Counter unit, a `package` request with the items `{"claim": "NotAboveThree"}` (an invariant state clause) and `{"claim": "ReachThree"}` (a `possible` state-graph claim) resolves each through its `claim_names` row and settles it with the source route's record; `{"claim": "Missing"}` settles `invalid-request` `unknown-claim`; the same directory whose `checked-package.json` has the `NotAboveThree` row removed refuses `invalid_package`/`invalid-value` at `/claim_names`, and one with two rows of one `qualified_name` refuses `ambiguous_declaration`/`ambiguous-name`, each before any engine runs. | Test (TC-2967) |
| FR-850-AC-8 | For every temporal clause and every state clause of FR-940-AC-4's Counter handoff, the clause the package route builds from the admitted bytes equals the source route's checked clause in every member, `span` included, and its `span` equals the `declaration_region` of the clause's name row; a handoff whose `checked-package.json` has one name row's `declaration_region` removed refuses the whole request `invalid_package` at the request stage naming that member, exit 20, before any engine runs. | Test (TC-2983) |
| FR-850-AC-9 | Over the handoff directory of the untimed Counter unit that imports its model as `M`, the `package` request with the items `{"claim": "Bounded"}` and `{"claim": "test/counter", "temporal": {"item_kind": "deadlock-freedom"}}` settles both with the records the source request with the same items gives; the same `package` request with the second item's `claim` `M` settles `Bounded` with the same record and settles that item `invalid-request` with `invalid_capability`/`model-alias`, naming `M` and `test/counter`, with no `not-yet-implemented` diagnostic and no item `unsupported`, while the source request with `M` settles it as with `test/counter`; the handoff directory of the same untimed unit importing its model as `N` holds a `checked-package.json` byte-equal to the `M` unit's. | Test (TC-2987) |
| FR-850-AC-10 | AC-1's Counter package holds the integer `=` application of the `Inc` postcondition (`self.value - pre(self.value) = 1`) in `checked_term_data.equality_decisions`. With `equality.json` removed from its handoff directory, the request with the item `{"claim": "Bounded"}` refuses the whole request at the request stage, `invalid_package`, cause `missing-file`, naming `equality.json`, exit 20, `items` empty and no engine run. With `equality.json` present, the same request settles `Bounded` `proved` with the record the source request gives. The handoff directory of the Counter unit whose `Inc` postcondition is written `self.value > pre(self.value)` holds no equality application (`checked_term_data` absent or `equality_decisions` empty); with its `equality.json` removed, the same request settles `Bounded` with the record the source request over that unit gives, exit 0. | Test (TC-3025) |
| FR-850-AC-11 | FR-1033-AC-1's `SB` unit is compiled by a `native-compile/1` request carrying the `handoff` member into its handoff directory, and FR-1033-AC-2's racing variant into its own. A `package` request over each directory, with `models` the `test/litmus` domain package document the package selects (FR-850 "Inputs") and the same `subject.protocol`, `memory` `[{"parallel": "SB::Both", "model": "tso"}]` and items as the source request, gives item records equal to the source route's: `NoBothZero` `refuted` with the same counterexample (both pending stores), replay target and `replay.reproduced` `true`, and over the variant the item `{"claim": "SB::Both", "temporal": {"item_kind": "race-freedom"}}` `refuted` with the same `Race` counterexample naming both writes, each echoing the same `temporal.subject` and effective model; with no `memory`, `NoBothZero` settles `proved` on both routes. No package-route record is `unsupported` and no request refuses for carrying `memory`. | Test (TC-3050) |
| FR-850-AC-12 | A compiled root calls `test/z::answer`, which calls `test/a::answer` returning Integer `42`. Running the root function from source and from its FR-517 directory returns `42` on both routes. Its Counter temporal claim `always holds(c.value <= z::answer() - 40)` is analyzed with the same models/documents/items on both routes and gives equal item records, counterexamples and reproduced replays except the request selection. Deleting all unit/library source files and the handoff's `sources/` before the package run and analyze/replay changes neither result. Removing `dependencies.json` refuses `invalid_package`/`missing-file` even for an import-free unit; no compile fallback occurs. | Test |
| FR-850-AC-13 | FR-517-AC-28's diamond is admitted once per unique library and settles the source route's result. A missing transitive artifact refuses before an item runs; a manifest entry for a foreign or conflicting library refuses `conflicting-definition`; an unreachable row, a duplicate identity, reversed rows, an unknown row member, a nonindexed path, `../escape`, an absolute path and a selected symlink/non-regular file each refuse by the dependency admission table at their own file or pointer. A lock cycle refuses `definition-cycle`; none reaches source or an engine. | Test |
| FR-850-AC-14 | For AC-12's transitive handoff, removal of either dependency's `identities.json` or `equality.json` refuses `missing-file` naming that indexed file even when the equality lists would be empty. Swapping non-empty witness files between distinct admitted libraries refuses their original identity/equality join error at the foreign file, before any item runs; no dependency receives the root's witness. The unchanged handoff completes. | Test |
| FR-850-AC-15 | For AC-12's handoff, compute `B` as the bytes of exactly the root, manifest, unique dependency package/witness files and selected model/runtime files read. At `dependent_bytes = B` intake completes; at `B - 1` it settles `incomplete`, `resource_exhausted`, exit `22`, naming `dependent_bytes` and its configured bound. A handoff whose manifest expands the selected-file total from 64 to 65 stops at the same phase naming the shared file limit before opening file 65. A diamond charges its shared triple once. For strict-read/native-join work and storage, an allowance equal to the completed closure pass succeeds and one below its next charge refuses under the original `i2.work_units` or `i2.native_join_storage` setting without restarting an allowance at a dependency boundary. | Test |
| FR-850-AC-16 | A library in AC-12's closure selects domain document `test/library-domain`, which the root does not select, and the request supplies both that document and the root's domain documents. Admission binds each document to the selecting package and completes without an extraneous-model refusal. Removing the library-only document refuses `missing_import`/`missing-selection` naming its identity; replacing it by content the library selection refuses preserves that reader refusal at the dependency package. No witness from the root is substituted. | Test |

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
