---
id: FR-850
title: "Analyze a selected checked package from its bytes without compiling source"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-027
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
of a `program` source (FSpec FR-520). When it does, FSL SHALL admit the
package bytes with the strict checked-package reader, build from those bytes
alone the in-process checked package the analysis engines take, and analyze
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

## Inputs

- The request's `package` member `{file}`, resolved against the request
  document's directory and counted toward `dependent_bytes` (FR-026).
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

- The package bytes SHALL be read by the strict checked-package reader FR-027
  uses for a run request's package selection, with no fallback to source
  compilation.
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

- FSL SHALL build the in-process checked package from the admitted package
  value: its package identity, its formula graph, its source map, its
  declaration names, its clause names, its profile and clock rows, its
  dependency selections and its model selections, with the supplied domain
  packages bound to those selections. The built value is the one the source
  route's S4 hands analysis for the same program, so every engine, the
  default-item rule, the subject binding and the replay of a counterexample
  read the same facts on both routes.
- An item's `claim` SHALL resolve by name against the package: a temporal
  clause through its `temporal_clause_names` row, and any other claim through
  the `declaration.qualified_name` of its `claim` node (FSpec FR-322). The
  name lookup gives the occurrence key `ClaimItem::new` takes, as FR-620
  states; a name no row and no declaration carries settles the item
  `invalid-request` with `invalid_capability`/`unknown-claim`.
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
| FR-850-AC-1 | The Counter program with the claims `Bounded` and `BelowThree` is compiled to its v2 package bytes; a `native-analyze/1` request that selects those bytes as `package`, with the same `models`, `documents` and `items` `[{"claim": "Bounded"}, {"claim": "BelowThree"}]` and no `program`, gives an outcome equal to the source request's outcome except for the `request` selection member, and the same exit code; the same holds with no `items` member, the default items in the same order. | Test (TC-2410) |
| FR-850-AC-2 | Over the package of AC-1 with a claim `AtMostTwo` (`always holds(c.value <= 2)`) added, the package route refutes `AtMostTwo` with the same counterexample, the same replay target and `reproduced` `true` as the source route. | Test (TC-2410) |
| FR-850-AC-3 | A `package` file holding `{}`, one holding a `filament.checked-package/v1` document, one holding AC-1's bytes with one source-map entry removed, and one that is not JSON each refuse the whole request at the request stage with code `invalid_package`, the reader's cause and pointer, exit 20, `items` empty, and no engine runs. | Test (TC-2410) |
| FR-850-AC-4 | AC-1's package request with `program` also present, with a `libraries` entry, and with `integer_division` `floor` each refuse `invalid-request` at the request stage naming that member, exit 20; with `models` empty it refuses `missing_import`/`missing-selection` naming `test/counter`; with an extra `models` entry the package does not record it refuses `invalid_package`/`conflicting-definition` naming that entry. | Test (TC-2410) |
| FR-850-AC-5 | AC-1's package request with the item `{"claim": "Missing"}` settles that item `invalid-request` with `invalid_capability`/`unknown-claim` naming `Missing`, as the source route does. | Test (TC-2410) |

## Dependencies

- FR-620 owns the request document and its other members; this requirement
  states its `package` selection.
- FR-027 owns the strict checked-package reader; FSpec FR-322 owns the v2
  package and its rows.
- FR-281 owns analysis, negotiation, settlement and replay over the built
  package.
