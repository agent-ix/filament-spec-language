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
  reaches an equality application (FR-533).
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
