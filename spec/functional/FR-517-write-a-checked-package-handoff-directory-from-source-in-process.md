---
id: FR-517
title: "Write a checked-package handoff directory from source, in process"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-042
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-480
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-514
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-515
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-499
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-500
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-505
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-531
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-508
    type: references
---
# FR-517: Write a checked-package handoff directory from source, in process

## Description

FR-042-AC-10 and AC-15 (retired as historical in M-6d) required that real
accepted source pass the compiler stages and produce a handoff that filament-protocol
consumes unchanged, written in process by a library call into a new directory,
deterministically, behind a feature in the normal dependency surface. The writer
`protocol_artifact::handoff::{write_v1, write_v2}` did this for the
`filament.compiled-protocol/1` and `/2` wires. The retired deletion table (rows PA A20 to
A22, W4 to W7, AC-10, AC-15) found no successor when this requirement was
written: nothing in the checked-package path wrote a directory, a selection
sidecar or a mutation manifest, and filament-protocol never read a checked package
(GAP-10, GAP-15). This requirement specifies the successor writer. FR-518
specifies the mutation corpus it writes.

The legacy writer, its two examples, the `handoff-writer` feature and the
`expected*.json` files were deleted by QSL-840 once FR-517-AC-1 to AC-9, AC-11
to AC-14, AC-17 to AC-20 and FR-518 were green and QSL-1549 had repointed
filament-protocol (FR-499's keep row, FR-521 row 2). That change replaced nothing:
this writer was already its successor.

## Use case

A downstream crate's test compiles the recipe fresh and asks FSL to write a
handoff into a directory it names. It then reads `checked-package.json` with
Contract IR's `read_checked_package`, using evidence assembled from the files
beside it, and checks the package against `selection.json`, which was derived
from the source before any bytes were emitted.

## Inputs

`filament_spec_language::checked_handoff::write(&CheckedHandoffRequest, &Path)`,
behind the Cargo feature `checked-handoff` and adding no dependency:

- `source`: one complete-V1 (`1-draft`) unit, its library sources, and its
  domain package documents, as the bytes `emit_checked_v2::Request` takes (the
  `emit_checked_v2::emit` entry, not its command line);
- `package_name`, `execution_points` and `temporal_selections` as
  `emit_checked_v2` and `fsl_package::emit_checked_with_temporal_selections`
  take them (`temporal_selections` is a new optional member of the request and
  also selects a `compensate` template by its template name and an `await`
  by the selector defined in "Await clock selections");
- `corpus`: `None` or `Standard` (FR-518).
- `native_join`: the finite work and logical-storage ceilings of the identity file's native joins (FR-531 "The native join budget"); its work and storage are `i2.work_units` and `i2.native_join_storage`. The writer has no default for it; one budget of it serves the write, and a denied charge returns `Error::NativeJoinExhausted` with Contract IR's refusal and writes no directory.

The `emit_checked_v2` command line carries `temporal_selections` as
`--clock-selections <json-file>`: a JSON object from the exact authored name of
a `temporal` clause, `compensate` template, or `await` selector to its clock configuration in the
`TemporalBinding` `clock` wire form, for example `{"Due": {"kind":
"event_position", "sequence_authority": "events-authority"}}`. Each member is
one `ClockSelection`, in file order. An empty object, a repeated name or a
value that is not a well-formed clock configuration is a refusal naming the
file; a name that matches no clause, template or `await` selector is the request's
`UnknownClause` refusal. Without the option the request holds no selection.

## Outputs

A new directory containing exactly:

| Path | Content |
| --- | --- |
| `checked-package.json` | the exact bytes `emit_checked_v2::emit` returns |
| `dependencies.json` | the required dependency artifact manifest, `filament.checked-handoff.dependencies/1`, as "The dependency artifact closure" states; present with an empty list when the root imports no library |
| `dependencies/<n>/checked-package.json`, `dependencies/<n>/identities.json`, `dependencies/<n>/equality.json` | one checked package and its own FR-531 witness files per manifest row; no source is needed to admit these artifacts |
| `sources/<label>` | the unit and library sources, exact bytes; `<label>` is the `source:<path>` label the package embeds |
| `models/<n>.json` | each domain package document, exact bytes, in selection order |
| `inventory.json` | `fsl_replay::fsl_inventory` over the unit, with the formal identity the writer selects (FR-514), in the encoding "The inventory file" states; the writer runs the replay spine over the unit's source to obtain the `check` outcome `fsl_inventory` takes (FR-517-AC-11). |
| `selection.json` | the selection sidecar, `filament.checked-handoff.selection/1` |
| `identities.json` | the required identity file, `filament.checked-handoff.identities/7`: selected declaration/field rows and final-emission source-record, source-tuple, source-union, source-enum, source-operation and source-object rows, together with the model-enum, model-union and model-tuple channels, in the encoding FR-531 "The identity file" states; a bytes-only consumer reads it with the package (FR-531) |
| `equality.json` | the equality file, `filament.checked-handoff.equality/1`: the equality application rows, equality fact rows and selected source-unit rows of the in-process unit, in the encoding FR-531 "The equality file" states, read by a bytes-only consumer with the package to evaluate a protocol slot whose walk holds an `==` or `!=` application (FR-533) |
| `mutations/manifest.json`, `mutations/<case>.json` | present only for `corpus = Standard` (FR-518) |

The sidecar holds, per temporal clause: the clause `NodeId`, declared name, the
protocol requirement entries that name it (FR-449), its activation form
(`origin` or `each`, with the trigger binder name and whether a guard is
written), the
profile selection (identity and revision) of its law, the lock definition
authority, and its expected `TemporalBinding` row (`clock_name` and clock
configuration); per `compensate` template, in the `compensations` member:
its `node`, `name` and expected `binding` (`TemporalBinding` row with
`clock_name` and `clock`; `binding` is absent for a clause or template to which
the request gives no clock selection); and per package: the contract version, the
package name (`package_name`), the unit's file under `sources/` (`unit`), the
accepted execution points (`execution_points`, an object from authored state
clause name to execution point, empty when the request gives none), the read limits the
writer used (`CheckedPackageReadLimits`), the evidence rows (which `models/`
files are domain package documents and which `sources/` files are libraries) and
the counts of nodes by form. Every value is taken from the in-process checked
unit and the request, never from the emitted bytes.

The node counts cover every node of a counted form that the unit's emission
holds, whichever declaration mints it. Each count is taken from the
in-process semantic graph, never from the emitted bytes, and counts every
node of its form by distinct node key, leaving none out:

- **`temporal/temporal_clause`**: three declarations mint these nodes, and
  structurally equal ones share a node (FSpec FR-322), so the count is the
  number of distinct keys among:
  - the unit's checked temporal clauses;
  - its hybrid claims (FR-447);
  - the body wrappers of its hyper clauses (FR-488 "The body").

  A count of temporal clauses alone would refuse every handoff of a unit
  with a hybrid claim or a hyper clause on `package.nodes`. A declaration
  form that later mints a `temporal_clause` node joins this count when it is
  added.
- **`claim/hyperproperty`**: the number of distinct `hyperproperty` node
  keys. `Selection::check` counts that form in the package.

Per hyper clause, in source order, the sidecar holds a row in the `hypers`
member: `node` (the `hyperproperty` NodeId), `name`, `occurrence` (the
clause's `claim` ordinal of that node), and `traces`, a list in quantifier
order of `{trace, binding}`. `binding` is the expected `TemporalBinding`
`clock_name` and `clock` of that trace's row, and is absent when the request
gives the clause no clock selection or its profile is not clock-configured.
`hypers` is omitted when the unit declares no hyper clause. The format
identity stays `filament.checked-handoff.selection/1`.

The expected number of `temporal_bindings` rows adds one row per quantifier
for each hyper clause whose profile is clock-configured, keyed on its
`hyperproperty` node at its `claim` occurrence with the quantifier's `trace`
(FR-488 "Clause names and clock bindings"); the emitter writes exactly those
rows and none keyed on the body wrapper. `Selection::check` refuses on
`clause.binding` when a trace's row is missing or present against an absent
binding, and on the clock axes when a value differs. Per-trace clocks are
derived from each trace's model (FR-640-AC-12), and a handoff names no clock a
model derives. An explicit clause-name selection for a hyper clause whose
quantifiers name more than one model is refused
`unsupported_construct`/`not-yet-implemented` (`PerTraceClockConfigurations`);
a single-model hyper clause takes the selection for every trace.


### Await clock selections

For each checked `await`, the named clock-selection key SHALL be exactly
`await <Protocol>::<AwaitName>`, with the owning protocol's checked name and
`CheckedAwait.name` substituted without normalization. The writer SHALL join
the selection to that protocol's checked `await` node and whole-declaration
span. If a supplied key identifies more than one checked declaration, the
request SHALL be refused as ambiguous, naming that key and writing no
directory; the writer SHALL NOT select the first match. A repeated supplied
key retains `DuplicateClause`, and a key with no match retains `UnknownClause`.
This adds no profile use site: the Await admission rules of
[FR-508](./FR-508-admit-a-profile-only-at-the-use-sites-of-its-kind.md) remain in
force, including the refusal of InfiniteTrace at Await.

The writer SHALL add an `awaits` member to `selection.json` immediately after
`compensations` and before `hypers` when the unit declares an `await`.
`awaits` SHALL be omitted when the unit declares none, leaving those units'
sidecar bytes unchanged. The format remains
`filament.checked-handoff.selection/1`. Its rows SHALL follow the checked
Await declarations in source order, across their owning protocols. Each row
SHALL contain exactly these members, in order:

| Member | Content |
| --- | --- |
| `node` | the `CheckedNodeId` of this Await's own native control declaration, joined from its checked protocol node key by the existing node-id encoding rule |
| `name` | the exact `await <Protocol>::<AwaitName>` selection key defined above |
| `occurrence` | the unsigned ordinal of this Await's `Declaration` occurrence on that node, as held by the checked unit; not the row index or an inferred zero |
| `binding` | the expected `{clock_name, clock}` in the existing binding encoding, or `null` when the existing profile and request require no configured binding |

The writer SHALL derive these rows from the checked unit and the actual
clock selections supplied for its acquisition. It SHALL NOT recover expected
binding values by reading the emitted package. A configured binding SHALL
retain the authored clock name and every supplied clock-configuration value.
The writer SHALL NOT infer a sequence authority, substitute `protocol-events`,
or supply a clock configuration that the request did not carry. An absent
selection required by the selected profile SHALL retain the existing missing
clock refusal and write no directory; it SHALL NOT become an unconfigured
Await row.

`Selection::read` SHALL refuse an Await row with a missing, repeated or unknown
member, a malformed node or occurrence, or a duplicate `(node, occurrence)`
pair. `Selection::check` SHALL match each expected binding to exactly the
package row with that declaration node and `Declaration` occurrence, with
`source_unit` and `trace` both absent. It SHALL NOT accept the first row for
the node or a binding at another occurrence. A missing or unexpected binding,
wrong occurrence, extra binding for that Await, or non-declaration locus
SHALL refuse on `clause.binding`, with `clause = Some(name)` for the affected
Await. Clock-name, clock-kind and configuration differences SHALL use the
existing `clock.*` axes and the same Await name. An absent or non-Await node
SHALL refuse on `clause.node` with that name.

The expected global `temporal_bindings` count SHALL include every configured
Await row as well as all temporal-clause, compensation and hyper-trace rows.
The comparison SHALL continue counting every actual package binding; adding
Await rows SHALL NOT suppress an extra row or reduce any existing count.

`Selection::inputs` SHALL reconstruct each Await's exact named clock
selection from these rows for source recompilation. The command's handoff
acquisition and the source replay that re-emits a package SHALL carry the
actual selections used by the checked acquisition, including Await
selections, to their existing emission entry. These paths SHALL NOT replace
those selections with empty selections or model-derived defaults. Equal
checked inputs and selections SHALL produce equal sidecar and package bytes
on write and recompile.


`checked_handoff::Selection::read(bytes)` parses the sidecar and
`Selection::check(&self, &CheckedPackageV2) -> Result<(), SelectionRefusal>`
compares an admitted package with it, counting every `temporal_bindings` row,
those of clauses, compensation templates, hyper traces and Await declarations;
`SelectionRefusal` carries one
`SelectionAxis` (FR-518). Both are FSL-owned so a consumer parses and compares
with no copy of the format.

The writer builds `equality.json` from the in-process unit's `Emission` rows
(`equality_producer_identity_rows`, `equality_fact_witness_rows` and
`selected_source_unit_rows`). The emitted package bytes are used for one join only:
every node and key member is written as the wire value of that node's `CheckedNodeId`
(the node id object of FR-531 "The identity file"), which the
writer reads from `WireGraph::of_package_bytes` over the emitted bytes, as for the
`node` member of `inventory.json`. A unit that holds no
equality application writes the file with `applications`, `facts` and `source_units`
empty, so the directory has the same paths for every unit.

### The dependency artifact closure

The writer SHALL persist the unique recursive closure of the root's selected
library packages from the same checked acquisition as the root, including a
library reached by more than one import path only once. Each library's artifact
SHALL be its checked-package emission, and its `identities.json` and
`equality.json` SHALL be derived from that library's own final emission by the
same rules as the root's witness files. A file with no equality applications
still has the three empty equality lists. The writer SHALL NOT substitute
library source for these artifacts or recompile a library to recover a missing
artifact or witness. The existing `sources/` entries remain available to the
explicit source-compiling `compile` entry; bytes-only consumers never read them.

`dependencies.json` SHALL be UTF-8 JSON, pretty-printed with two-space indent,
members in the order below, ending in one newline. Equal checked inputs SHALL
produce byte-identical manifests and dependency trees. Its top-level object
and every row SHALL be closed: missing, repeated and unknown members are refused.

| Member | Content |
| --- | --- |
| `format` | the string `filament.checked-handoff.dependencies/1` |
| `dependencies` | an array of the entire unique recursive library artifact closure, ordered lexicographically by the UTF-8 bytes of each selected library identity |

Each row SHALL contain exactly `identity` and `directory`, in that order.
`identity` SHALL be the non-empty selected library identity used by
`lock.dependency_selections`, with no normalization, revision or version.
For the row at zero-based position `n`, `directory` SHALL be exactly
`dependencies/<n>`, where `<n>` is the unsigned decimal position with no leading
zero except `0`. It is relative to the handoff directory; an identity is never
used as a filesystem path. The only artifact paths in that directory are
`checked-package.json`, `identities.json` and `equality.json`. The manifest
SHALL NOT carry import edges, source paths, digests or version pins: every
package's existing lock supplies its direct dependency edges.

For example, a root importing `test/z` that imports `test/a` writes:

```json
{
  "format": "filament.checked-handoff.dependencies/1",
  "dependencies": [
    {
      "identity": "test/a",
      "directory": "dependencies/0"
    },
    {
      "identity": "test/z",
      "directory": "dependencies/1"
    }
  ]
}
```

This is manifest order, not admission order. A root with no library selections
SHALL write the same object with `"dependencies": []` and no `dependencies/`
subdirectories. A missing manifest is not an empty closure and SHALL be refused;
there is no source or legacy-format fallback. A repeated identity, a second
artifact for one selected identity, an artifact not reachable from the root's
lock, or an omitted reachable identity SHALL NOT be admitted. A diamond whose
branches select the same admitted library shares its one row; branches requiring
conflicting admitted content under one identity SHALL be refused, not resolved
by selecting whichever artifact is read first.

A bytes-only consumer SHALL load the closure and witnesses under
[FR-850](FR-850-analyze-a-selected-checked-package-from-its-bytes.md)
"Dependency artifact admission". The output directory's paths are confined
relative paths: no absolute path, `.` or `..` component, backslash, symbolic
link or non-regular artifact file is admitted. The writer's existing atomic
write and no-partial-directory rules apply to the complete dependency tree.

### Compiling a handoff back in process

A consumer that holds only a handoff directory needs the in-process checked
package to dispatch an analysis whose subject is one (FR-571 "The protocol
subject"). `checked_handoff::compile(&Path) -> Result<CompiledHandoff, Error>`,
behind the same feature, reads `selection.json`, reads the unit, library and
domain package files its rows name, and compiles them through the entry `write`
compiles through, with the sidecar's package name, execution points and the clock
configuration of each binding row, each source under the path its
`source:<path>` or `library:<path>` label names. No file is parsed except
through the sidecar's reader, the compiler and Contract IR's reader.

- The recompiled bytes SHALL equal `checked-package.json`; otherwise the call
  returns `Error::ReadBackDiffers` and no package.
- The package is read with `read_checked_package` and the evidence `write`
  assembles, and compared with the sidecar by `Selection::check`.
- A sidecar file path that is empty or leaves the directory, that passes
  through a symbolic link or names anything but a regular file (a symbolic
  link, a FIFO, a device), or a source label other than `source:` for the unit
  and `library:` for a library, is `Error::Label`; a file that cannot be read, or is over its ceiling (the
  reader's byte limit for `selection.json` and `checked-package.json`, the
  source ceiling for the others), is `Error::Read`; a sidecar that does not parse
  is `Error::Sidecar`.

`CompiledHandoff` gives the checked package (`checked`), the admitted package
(`admitted`), the package input of the domain package documents (`packages`)
and the emission ceiling the compile used (`emit_limits`).

### The inventory file

`inventory.json` is written once beside `checked-package.json`. Contract IR
publishes no wire form of a `SourceInventoryOutcome`, so the encoding is FSL's,
and it follows `selection.json` and `identities.json`: one JSON object,
pretty-printed with two-space indent, members in the order below, closed (an
unknown or repeated member is a refusal), UTF-8, ending in one newline,
byte-identical for equal inputs. The writer builds every row from the outcome
`fsl_inventory` returns for the in-process unit. The emitted package bytes are
used for one join only: a requirement row holds a structural `CheckedNodeId`
(`fsl_semantics::check::wire_id::structural_id` of the occurrence's node key),
and its `node` member is the wire value of that node's `CheckedNodeId` in the package (an object
with `package`, `path` and `ordinal`, as `selection.json` and `identities.json` spell node ids), which the writer
reads from `WireGraph::of_package_bytes` over the emitted bytes, as
`Selection::derive` does for `selection.json`. A node the package does not
name is the writer's `Error::Unnameable`, and nothing is written.

| Member | Content |
| --- | --- |
| `format` | the string `filament.checked-handoff.inventory/1` |
| `owner` | the string `fsl` (`SourceInventoryOwner::Fsl`) |
| `scope` | the string `fsl_checked_requirements_and_pending_obligations` |
| `source` | the formal identity as `{ "document": <text>, "revision": <integer> }`; every row's source is this value, so rows carry none |
| `completeness` | the string `complete`; the writer refuses a unit the compiler does not check (FR-517-AC-7), so it never writes an `Incomplete` inventory |
| `observed_count` | the number of rows, a JSON integer equal to the length of `rows` |
| `rows` | the outcome's rows in the order `fsl_inventory` yields them: the requirement records in report order, then the pending obligations in report order |

A row is a closed object with a `kind`. A requirement record is `{ "kind":
"requirement_record", "node": <node reference>, "role": <text>, "ordinal":
<integer> }`, with `node` the wire value of the package's `CheckedNodeId` for that node (the node
reference of FR-531 "The identity file"), `role` the requirement's role text and
`ordinal` its occurrence ordinal. A pending obligation is `{ "kind":
"pending_obligation", "declaration": <integer>, "unit": 0, "start": <integer>,
"end": <integer>, "capability_kind": <text> }`, with `declaration` the ordinal
of the requirement form that holds the obligation's span (FR-514), `unit` always
`0`, `start` and `end` the span's byte offsets in the unit's source, and
`capability_kind` the exact capability kind name. The inventory has no
`expression` member: the spine holds no expression handle (FR-514). Numbers are
JSON integers; no value is a float or a digest.

The formal identity is selected by one rule, so that equal requests give equal
files and no caller input is needed. `fsl_inventory` takes Contract IR's
`SourceIdentity`, whose `document` is a `SourceDocumentId`: it starts with an
ASCII letter and holds only ASCII letters, digits, `.`, `_` and `-` (`:` and
`/` are rejected), so the `source:<path>` label of `sources/<label>` cannot be
the document. `document` is the file name of the unit's source path (the last
path component, extension included) with every character outside that alphabet
replaced by `_`, and prefixed with `u` when the result is empty or does not
start with an ASCII letter; `revision` is `1`. The three packages of **R**
(`unit-event-position.native`, `unit-fixed-sample.native`,
`unit-timestamped-event.native`) therefore have the documents
`unit-event-position.native`, `unit-fixed-sample.native` and
`unit-timestamped-event.native`. The rule adds no disambiguator and no digest:
one handoff holds one unit, so its document is unique within it, and units whose
file names sanitize to the same text are told apart by the `sources/<label>`
beside the file, which keeps the full path label. The formal identity is a
parameter of `fsl_inventory` and not part of the checked unit (FR-514), so this
rule is the writer's.

`checked_handoff::Inventory::read(bytes)` parses the file, behind the same
feature as `Selection::read`, and returns `InventoryReadError`, a closed set:
`Malformed` (not JSON, not this shape, an unknown or repeated member or row
member, a `kind` that is neither row kind, a non-integer number, a `completeness`
other than `complete`, or an `observed_count` unequal to the length of `rows`)
and `Format` (another `format` string, naming it). It returns no partial value.

Vector. The unit is the file `unit.native` holding
`language "ix:native" edition "1-draft";`, `profile v = "filament.value.complete/v1";`
and `function f using v(x: Int[0, 9], p: Boolean): Integer pure { if p then x + 1 else x + 1 }`,
each on its own line, so its formal identity is document `unit.native`, revision
1. `fsl_inventory` over it (measured on the spine) returns four rows: two
requirement records with role `expression` and ordinals 0 and 1, then two
pending obligations of capability kind `value-validity` for declaration 0 at
byte spans 153 to 158 and 164 to 169. `N0` and `N1` stand for the `CheckedNodeId` wire values (a JSON object, written with
the same two-space indentation at the depth of the row's members) of the node
the two records name in the emitted package, which are not fixed here; both
records are occurrences of one node. The bytes are exactly these lines, each ending in a line feed,
with no trailing space:

```
{
  "format": "filament.checked-handoff.inventory/1",
  "owner": "fsl",
  "scope": "fsl_checked_requirements_and_pending_obligations",
  "source": {
    "document": "unit.native",
    "revision": 1
  },
  "completeness": "complete",
  "observed_count": 4,
  "rows": [
    {
      "kind": "requirement_record",
      "node": N0,
      "role": "expression",
      "ordinal": 0
    },
    {
      "kind": "requirement_record",
      "node": N1,
      "role": "expression",
      "ordinal": 1
    },
    {
      "kind": "pending_obligation",
      "declaration": 0,
      "unit": 0,
      "start": 153,
      "end": 158,
      "capability_kind": "value-validity"
    },
    {
      "kind": "pending_obligation",
      "declaration": 0,
      "unit": 0,
      "start": 164,
      "end": 169,
      "capability_kind": "value-validity"
    }
  ]
}
```

Refusal vectors for `Inventory::read`: the vector with `"observed_count": 3` (four rows) is
`Malformed`; with `"format": "filament.checked-handoff.inventory/2"` is `Format`
naming that string; with a row `"kind": "criterion"` is `Malformed`; with
`"completeness": "incomplete"` is `Malformed`; with `"start": 10.0` is
`Malformed`; with a second `"owner"` member is `Malformed`; the empty input is
`Malformed`.

## Behavior

- **In process, no side channel.** The writer calls the library entries only: no
  shell, no stdout parser, no alternate frontend and no hand-written fixture
  supplies the package (FR-042-AC-10's rule, kept).
- **Read back before writing.** The writer reads the emitted bytes with
  `filament_contract_model::read_checked_package` and evidence it assembled from the
  files it is about to write, and compares the admitted package with the
  in-process checked unit (node forms, node ids, temporal bindings). Any
  difference returns `Error::ReadBackDiffers` and writes nothing. With
  `corpus = Standard` it also replays every case (FR-518) before writing.
- **Deterministic.** Two calls with equal requests write byte-identical trees.
- **New directory only.** An existing path returns `Error::Exists` and its
  contents are untouched. The tree is built in a sibling temporary directory and
  moved into place, so a failure leaves no partial directory at the requested
  path (an improvement on the legacy writer, whose README allows a partial
  directory).
- **No fork.** The writer and `Selection::check` read packages only through
  Contract IR's reader and the borrowed `CheckedPackageV2` surface.

### The recipe maps to complete-V1 source

The IT-001 recipe (`examples/protocol-handoff/`) is a located rule model
(`model.json`) and five `0-draft` bodies (`predicates`, `state`, `temporal`,
`temporal-v2`, `workflow`). The checked-package producer takes complete-V1
source (QSL-1501 symbol map, row C), so the recipe is ported, not wrapped. The
ported recipe is **R**; its elements and their spine requirements:

| Recipe element | In **R** |
| --- | --- |
| `model.json` scalars `ObjectId` (text, 128), `Amount` 1..20, `Total` 0..100, `Tally` 0..5, `Wide` (signed 64-bit) | one domain package document with the same bounds, selected by a `model` declaration; `Amount` and `Total` are aliases of the unit `ix://filament/unit/U` that keep their bounds as member constraints (FR-517-AC-13) |
| scalars `Ratio` and `Exact` (bounded rationals) | two Rational value types of the model with no `unit` (FR-505: `unit` is optional, and the legacy scalars carry none): `Ratio` with numerator -1..1 and denominator 1..2, and `Exact` with numerator -9223372036854775808..9223372036854775807 and denominator 1..9223372036854775807; the compiled source reaches each, so the package carries it (FR-517-AC-12) |
| records `Workflow`, `Notice`, `Provider`; sequence `receipts` (maximum 5); the compensations' `attempts`, `retry` and `recover` binders typed by the object type `Workflow` | the object types `Workflow` and `Provider`, the record value type `Notice` and the bounded sequence `receipts`; `Workflow` stays the `over`, role and forward-binder type; the `attempts` type and the `retry` binders are the record value type `Attempt` (field `tally`), and the `recover` binder the record value type `Recovery` (field `receipts`) |
| `predicates` and `state` bodies: cross-unit calls, bounded `sum`/`size` queries, the extrema literals `-9223372036854775808` and `9223372036854775807` | predicates and functions in a library unit imported by the main unit, with the same literals |
| `temporal` bodies, three clock kinds: event position, fixed sample, timestamped event | four `temporal` clauses, each under a `profile` selection; clock configurations are given in the request (QSL-1557); `temporal_bindings` holds one row per temporal clause and per compensation template, in ascending `NodeRef` order |
| `workflow` body: roles `Service` and `Provider`, channel `Decisions` with two `Notice` receives and an all-branch join, `Outcome` choice with complementary Boolean guards, `Workflow::apply` with pre and post, and the compensations `Full` (commit `Main::Committed`) and `Partial` (commit `never`) | one `protocol` with the same roles, channel, join, choice, effect and two `compensate` templates (FR-383), whose `recover` conditions compare the receipt sum with the captured target (FR-517-AC-14) |

The shared library of **R** SHALL reach both unit-less Rational types through
typed predicate parameters and Rational literals. Each of the three packages
SHALL carry the two extreme literals with the emitted bounded Rational type of
`Exact`, not merely as source text or values of an unbounded Rational type.
`Ratio` keeps numerator bounds -1..1 and denominator bounds 1..2; `Exact` keeps
numerator bounds -9223372036854775808..9223372036854775807 and denominator bounds
1..9223372036854775807. Neither declaration gains a `unit`.

The legacy recipe's `Workflow` attempt and recovery binders are not portable,
and **R** replaces their type rather than the element. An attempt record is a
record or an event type (FR-381 "Activation, attempts and deadline"; FSpec
FR-466 reference check 1 on `attempts.record_type`), so `attempts 3 of
M::Workflow` is refused. The retry relation and the recovery predicate bar
model state, and a field read of an object-typed binder is a model-state read
(FR-381 "Names a stage may read", FR-381-AC-14; FSpec FR-464 "Slots that bar
model state"), so `earlier.tally` and `recovered.receipts` over `Workflow`
binders are refused. The compensations keep every stage, bound and retry
relation of the legacy recipe over the record value types `Attempt` and
`Recovery`, and their `recover` conditions are those of FR-517-AC-14: the
reader admits the `sum` query binder that a `recover` slot introduces itself. A `recover` condition that
reads its recovery binder is checked and emitted.

The unit `U` on `Amount` and `Total` is carried by the model. In the model it is
an alias of `ix://filament/unit/U` whose `min` and `max` name the alias (FSpec
FR-154 "Unit bounds"; FR-230-AC-26), and each source declares
`dimension Money;` and `unit U : Money = rational(1, 1);`. Unit-typed members
check against unit literals (`amount >= 1 U` in `BeforeApply`). A measurement
unit that only the selected model's types name is emitted with
its dimension and target closure (FR-094); the library and fixed-sample and
timestamped-event units no longer need an expression that reads a unit-typed member.
Model aliases are source type names: `M::Total` resolves to its admitted
unit-bearing type. Its model `min`/`max` constraints retain their original
alias authority as member bounds (FR-230), without widening them or bounding
unrelated values of `U`. `capture targetFull: M::Total` therefore checks.
`sum<N>` checks a summand of the numeric family of `N` (FR-517-AC-20): an
integer total takes any integer summand, a rational total any rational, a decimal
total any decimal, and a float or quantity total a summand of that very float
type or unit (a bounded quantity of the unit is a quantity of it), and a carried-quantity total a summand of that very carried type. It lowers
to the family's `filament.op.collection.sum.<family>` identity (FR-093) and
evaluates by the family's addition under the value-accounting collection sum
schedule: a total the checker cannot prove inside `N`'s domain is refused as an unproved range (FR-046, FR-100),
and the empty sum is the family's zero, which `N` must admit. So **R**'s
`Healthy` invariant sums its receipts over `Quantity<U>` without the earlier
`IllTyped(TypeMismatch)`.

**R** is three packages, one per clock kind, because the emitter admits one
temporal profile selection per unit and the checker refuses a unit that selects
three. `write` writes one handoff directory for each package, and FR-518's cases
run in the package of their clock kind, so the corpus is split by kind across
the three directories. The packages hold:

| Package | Temporal clauses | Protocol, compensations and effect | `temporal_bindings` rows |
| --- | --- | --- | --- |
| event position | `Due` and `DueLate`, so the row-ordering case can swap them | one `protocol_clause`, the `Full` and `Partial` compensations, the `Main::Applied` effect | four: two clauses and two compensation templates |
| fixed sample | `DueSample` | none | one |
| timestamped event | `DueTimestamp` | none | one |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-517-AC-1 | `write(&request_for_R, dir)` creates exactly the paths of the Outputs table, `inventory.json` included (with no `mutations/` for `corpus = None`), and `checked-package.json` equals the bytes `emit_checked_v2::emit` returns for the same request. | Test (TC-1312) |
| FR-517-AC-2 | The package read from `checked-package.json` with `read_checked_package` and evidence assembled from the written files is admitted, and `Selection::check` passes; the sidecar's clause ids, names, requirement entries, activation forms, profile selections, definition authority and `TemporalBinding` rows, those of clauses and those of compensation templates, equal the values read from the checked unit held in memory by the writer. | Test (TC-1312) |
| FR-517-AC-3 | The event-position package of **R** emits one `protocol_clause`, a `Full` and a `Partial` compensation node with different keys paired with the one `Main::Applied` effect, two temporal clauses and four `temporal_bindings` rows in ascending `NodeRef` order; the fixed-sample and timestamped-event packages each emit one temporal clause and one row; the rows cover the clock kinds `EventPosition`, `FixedSample` and `TimestampedEvent`. | Test (TC-1312) |
| FR-517-AC-4 | **R** carries the extrema `-9223372036854775808` and `9223372036854775807` as `Wide` values of the emitted package, and no declared bound is widened to admit them. | Test (TC-1312) |
| FR-517-AC-5 | Two `write` calls with equal requests give byte-identical trees (every file, in path order). | Test (TC-1312) |
| FR-517-AC-6 | `write` onto an existing directory returns `Error::Exists` and leaves every byte of the directory unchanged; a failure injected after the temporary tree is built (read-back difference, I/O error on the third file) leaves no directory or temporary directory at or beside the requested path. | Test (TC-1312) |
| FR-517-AC-7 | A request whose source the compiler refuses returns the compiler's refusal and writes nothing; a request whose emitted package the reader refuses, and a request for which `ReadBackDiffers` is injected, write nothing. | Test (TC-1312) |
| FR-517-AC-8 | `checked_handoff` is behind the feature `checked-handoff`, adds no entry to `Cargo.lock`, and a downstream crate with only that feature enabled compiles `write` and `Selection::check` without building this crate's examples or dev-dependencies. | Test (TC-1312) |
| FR-517-AC-9 | A package with one `temporal_bindings` clock value changed to another well-formed value is admitted by `read_checked_package` and refused by `Selection::check` naming its `SelectionAxis`; no function of the module parses a checked package except through `read_checked_package` (`git grep` finds no second reader). | Test (TC-1312) |
| FR-517-AC-10 | The legacy writer is absent: `protocol_artifact::handoff` (`write_v1`, `write_v2`, the five `PUBLISHED_*` constants), the `handoff-writer` feature, the two examples `native_protocol_handoff` and `native_protocol_v2_handoff`, `examples/protocol-handoff` and `tests/it/handoff_writer.rs` do not exist, and no Cargo manifest of the workspace names the feature. The gate that kept them (FR-517-AC-1 to AC-9, AC-11 to AC-14, AC-17 to AC-20 and FR-518-AC-1 to AC-12 green, FR-521 row 2) was met when they were deleted. | Test (TC-1312) |
| FR-517-AC-11 | Each handoff directory written for **R** contains `inventory.json`, and its bytes equal the encoding "The inventory file" states of the outcome `fsl_replay::fsl_inventory` returns over the unit, run through the replay spine, with the formal identity that section selects; the file's rows are non-empty, `observed_count` equals their number, and two writes of equal requests give equal files. | Test (TC-1312) |
| FR-517-AC-12 | **R**'s model declares the Rational value types `Ratio` and `Exact` with no unit, the compiled source reaches both, and the emitted package of **R** carries the rationals `9223372036854775807/9223372036854775806` and `-9223372036854775808/9223372036854775807` as `Exact` values, with no declared bound widened to admit them. | Test (TC-1312) |
| FR-517-AC-13 | **R**'s model declares `Amount` and `Total` as aliases of the unit `ix://filament/unit/U`, each keeping its `min` and `max` naming the alias (`Amount` 1 and 20, `Total` 0 and 100), and each of the three **R** sources declares `U`. The emitted package of **R** holds the node of `U`, types the `targetFull` and `targetPartial` captures by it, and lowers **R**'s receipt sums to `filament.op.collection.sum.quantity`. | Test (TC-1312) |
| FR-517-AC-14 | With a `sum` query binder admitted inside `recover`, the `recover` condition of `Full` is that the receipt sum equals the captured target, and that of `Partial` is that the receipt sum is positive and below the captured target. | Test (TC-1312) |
| FR-517-AC-15 | `emit_checked_v2` run with `--clock-selections` naming a file that holds the clock configuration of each temporal clause and compensation template of an **R** package emits the bytes `emit_checked_v2::emit` returns for the equal `Request`; an empty object, a repeated name, an unknown `kind`, a member the configuration does not define and a member it requires that is missing are each a refusal naming the file that writes no bytes, and a name matching no clause or template is the `UnknownClause` refusal that writes no bytes. | Test (TC-1312) |
| FR-517-AC-16 | Over **R**'s model with `Amount` and `Total` as aliases of `ix://filament/unit/U`, a source whose `pre` compares the `Amount` parameter with `1 U` is written and admitted. `sum<Cash>` over the receipts, with `type Cash = Quantity<U>;`, is written and admitted (FR-517-AC-20). | Test (TC-1312) |
| FR-517-AC-17 | `Inventory::read` over the bytes `write` produced for **R** returns rows equal to those of the in-process outcome, and over each refusal vector of "The inventory file" returns the stated `InventoryReadError` and no value; two reads of equal bytes return equal results. | Test (TC-1312) |
| FR-517-AC-18 | A measurement unit that the selected model's types name is part of the emitted package even when no expression of the compiled source reaches it: the package holds its `dimension` and `unit` nodes, and Contract IR's reader admits a package whose model `typeRef` names it. Over **R**'s model with `Amount` and `Total` as aliases of `ix://filament/unit/U`, a source that reads no unit-typed member is written and admitted. | Test (TC-1312) |
| FR-517-AC-19 | A model alias is a source type name: over **R**'s model, `sum<M::Total>` and `capture targetFull: M::Total` resolve to the alias's unit-bearing type, and neither refuses `UnresolvedTypeName`. The capture checks, and the sum checks as a quantity sum (FR-517-AC-20). The alias's original `min`/`max` bounds remain selected-model member constraints and enforce the same endpoint values as before, without widening them or imposing them on other aliases of `U`. | Test (TC-1312) |
| FR-517-AC-20 | `sum<N>` checks a summand of any numeric family FSpec FR-145 and FR-322 give a `filament.op.collection.sum` identity for (integer, rational, decimal, float, `Quantity<U>` and the carried-quantity family), and lowers to that family's identity; over **R**, the `Healthy` invariant's receipt sum over `Quantity<U>` checks and evaluates to the sum of the receipts' magnitudes in `U`, and a `sum<N>` whose `N` is a carried `Quantity<U, C>` over summands of that very type checks, lowers to `filament.op.collection.sum.carried_quantity` and evaluates (FSpec FR-142-AC-33 to AC-38). | Test (TC-1312) |
| FR-517-AC-21 | The handoff directory written for TC-1070 fixture E contains `equality.json`, whose bytes equal the encoding FR-531 states of the rows of the in-process unit, with the three row lists non-empty (the unit holds integer `=` applications) and each row's node members naming nodes of `checked-package.json`; the directory written for the unit of the inventory vector (`unit.native`, which holds no `==` or `!=` application) contains `equality.json` with the three lists empty; two writes of equal requests give byte-identical files. | Test (TC-1312) |
| FR-517-AC-22 | A writer given a unit whose equality rows name a node the package does not hold returns `Error::Unnameable` and writes nothing, as for `inventory.json`; the written file holds no source text, no digest and no member the encoding of FR-531 does not list. | Test (TC-1312) |
| FR-517-AC-23 | The writer emits the required `identities/7` source-record, source-tuple, source-union, source-enum, source-operation and source-object lists from the same final emission as the package: original opaque keys, final structural/type references, original member identities and order round-trip through native joins. The reader requires all six source lists and the model-enum, model-union and model-tuple channels, including empty arrays only for genuinely empty producer channels, and refuses any omitted channel or `identities/6` and older formats; no optional compatibility path or source fallback supplies them. Source enum shapes, ordered flags, complete canonical case populations and original variant identities round-trip as FR-531-AC-50 states. | Test |
| FR-517-AC-24 | `compile` over a directory `write` wrote returns a `CompiledHandoff` whose checked package emits the bytes of `checked-package.json` and whose admitted package passes `Selection::check`; the sidecar names the package name, the unit's file and the execution points the request gave. A directory whose `checked-package.json` is edited to other bytes the reader admits returns `Error::ReadBackDiffers`, one whose sidecar `unit` leaves the directory, and one whose `models/0.json` is a symbolic link to a file outside it, return `Error::Label`, and one whose sidecar is removed returns `Error::Read`; none returns a package. | Test (TC-1312) |
| FR-517-AC-25 | `write` over a unit declaring the temporal clause `Bounded` and the hyper clause `Equal` (`forall trace a of M forall trace b of M { always holds(c.value @ a <= 3 and c.value @ b <= 3) }`) writes a sidecar whose `temporal/temporal_clause` count is 2 and whose `claim/hyperproperty` count is 1; `compile` over that directory returns a `CompiledHandoff` whose admitted package passes `Selection::check`, and a `native-compile/1` request carrying the `handoff` member over the unit exits 0. A unit that adds a hybrid claim to `Bounded` and `Equal` writes a `temporal/temporal_clause` count of 3 and passes `Selection::check` the same way. The first sidecar with its `temporal/temporal_clause` count set to 1 refuses the package on `package.nodes`, so the comparison can fail. | Test (TC-3128) |
| FR-517-AC-26 | `write` over a unit declaring the hyper clause `Ordered` under the event-position false-extension profile, with the quantifiers `forall trace a of M` and `forall trace b of M`, emits a package whose `temporal_bindings` hold exactly two rows for it. Both rows are keyed on its `hyperproperty` node at its `claim` occurrence, one with `trace` `a` and one with `trace` `b`, in quantifier order. No row is keyed on its body wrapper. The strict reader admits the package, the sidecar's expected binding count includes the two rows, and `Selection::check` passes. A package with one row keyed on the wrapper with no `trace` is refused by the strict reader at that row's `declaration`, so the emission check can fail. | Test (TC-3128) |
| FR-517-AC-27 | A root importing `test/z`, which imports `test/a`, writes the manifest vector of "The dependency artifact closure" and exactly one artifact/witness triple in each indexed directory. Each triple is admitted with that package's own direct dependencies and witnesses, and the root is admitted with the retained closure. Reordering the caller's library provisions does not change any written byte. A unit with no imports writes the required manifest with an empty list and no dependency directory. | Test |
| FR-517-AC-28 | For a diamond whose root imports `test/left` and `test/right`, both importing `test/shared`, the manifest contains three unique rows ordered `test/left`, `test/right`, `test/shared`, with directories `dependencies/0`, `dependencies/1`, `dependencies/2`; the shared package and its witnesses occur once. Two writes give equal trees. A conflicting `test/shared` selection is refused under the existing dependency conflict rule and leaves no output directory, never a tree containing one arbitrarily chosen shared artifact. | Test |
| FR-517-AC-29 | A checked unit containing an Await `Wait` in protocol `Flow`, with an event-position clock selection whose sequence authority is `caller-events`, writes an `awaits` row naming `await Flow::Wait`, its own native node, its actual Declaration occurrence and the exact authored clock name and supplied configuration. The strict reader admits the package, `Selection::check` passes, and `compile` reconstructs the same selection and package bytes. The authority is never replaced with `protocol-events`. | Test |
| FR-517-AC-30 | A unit with two Await declarations writes their rows in source order and counts both configured bindings alongside temporal-clause, compensation and hyper-trace bindings. Equal writes give byte-identical sidecars. A unit with no Await omits `awaits` and retains its prior sidecar bytes. | Test |
| FR-517-AC-31 | For an admitted package with an Await binding, changing its clock name, kind or a configuration value to another reader-admitted value makes `Selection::check` refuse on the corresponding existing `clock.*` axis with `clause = Some` of the Await selector. A missing binding, a binding at another reader-admitted declaration occurrence, or an extra binding is refused on `clause.binding`; a missing or non-Await node is refused on `clause.node`. No mutation is accepted by matching another occurrence's row. | Test |
| FR-517-AC-32 | `Selection::read` refuses an Await row with a missing, repeated or unknown member, a malformed occurrence, or a repeated `(node, occurrence)` pair. A supplied Await selection key that matches two declarations is refused as ambiguous and writes no directory; an unknown key and a repeated supplied key retain `UnknownClause` and `DuplicateClause`, respectively. A missing clock configuration required by the Await profile retains its existing refusal and writes no directory. | Test |
| FR-517-AC-33 | An analyze request whose checked acquisition receives a caller-supplied Await clock selection preserves that exact selection in its source replay and handoff emission. Source recompilation and the package path use the same clock configuration and produce the same replay outcome; neither path silently substitutes empty selections or model-derived clocks for the supplied Await selection. | Test |

## Dependencies

- **Upstream:** the inventory writer (FR-517-AC-11, AC-17) runs the replay spine over the unit and calls `fsl_inventory` over it (FR-514). FR-517-AC-12 needs no spine change: FR-505 admits a Rational value type without a unit, and the criterion fixes only what **R**'s model declares and the source reaches. FR-517-AC-13 is gated on the three spine capabilities of FR-517-AC-18 to AC-20 (the unit a model type names is emitted, a model alias is a source type name, `sum` over a quantity), all delivered. FR-517-AC-14 holds: the reader admits a binder that a protocol slot introduces itself, such as the `sum` query binder inside `recover`. FR-042-AC-10 and AC-15 (retired), FR-337, FR-383, FR-480,
  FR-505, FR-514, FR-515, `emit_checked_v2`, `fsl_package::emit_checked_with_temporal_selections`.
- **Downstream:** filament-protocol IT-001 (QSL-1549).
