---
id: FR-620
title: "Read a native-analyze/1 request document and analyze its items"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-368
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-507
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-623
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-624
    type: traces_to
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-520
    type: depends_on
---
# FR-620: Read a native-analyze/1 request document and analyze its items

## Description

When a caller hands FSL's request-file owner a request document whose
`format` is `native-analyze/1`, FSL SHALL read the document, compile or admit
its program, resolve each requested analysis item against the checked
package, and run the items through `analyze` (FR-281). The document is the
file encoding of `analyze`, as `native-run/1` is the file encoding of
`execute` (FR-100).

### Decision: a separate analyze envelope whose items are FR-331 request items

The analyze request is its own document, `native-analyze/1`, inside FR-026's
shared envelope `{"format": ..., "request": {...}}`. It is not the clause-run
request of FR-312. A clause-run request binds one clause to observations a
caller supplies and evaluates that one trace; an analyze request decides
claims over every behaviour of a subject, carries many items, and gives each
item its own engine, fairness, reduction and budget settings. Folding both
into one envelope would give every clause-run request members it can never
use, and every analyze request a single-trace shape it does not have.

The items of the document reuse the request-item members of FSpec FR-331
(`kind`, `temporal` with its `item_kind`, `probabilistic`, `hybrid`, `check`,
`options`, `reductions`, `correspondence`, `premises`, `assessed_root`,
`backend`), with FR-331's spellings and closure rules. So `analyze`, `prove`
and an external provider share one item vocabulary, and the standard owns
it once (FSpec FR-520). The document differs from an FR-331 request in what
a person writes: a claim is named by its declared name, not by a node key;
the extent, the candidates and the request index are computed by FSL, never
written by the caller.

## Inputs

A `native-analyze/1` request document. FSpec FR-520 "Document members"
defines its request members and its analysis-item members, their
multiplicities and spellings: the program or package selection, `models`,
`libraries`, `refinement_internals`, `documents`, `memory`, `subject`,
`items`, `limits` and `deadline_ms` (0 to 2^53 - 1 milliseconds, absent
meaning `DeadlineSupply::Unbounded`); and per item `claim`, `kind` (an FSpec
FR-290 kind or `simulation`, FR-634), `backend`, `required`, the FSpec FR-331
members with `temporal.item_kind`, `trace` and `simulation`. This requirement
states how FSL reads them.

## Outputs

The `analyze` outcome of FR-621: one `filament-outcome/1` document with one
item per resolved analysis item, or a whole-request refusal (FR-624).

## Behavior

### Reading

- The reader SHALL apply FR-026's shared envelope rules unchanged: source
  labels, relative-path resolution, the selected-file ceiling and the
  `request_bytes` and `dependent_bytes` limits. Each supplied document counts
  toward `dependent_bytes`.
- The request object and every item SHALL be closed. An unknown member, a
  duplicate provisioned document identity, a `program` and `package` both
  absent, or a member of the wrong JSON type refuses the whole request at the
  request stage with `invalid-request`, exit 20, naming the member.
- An item that carries `request_index`, `extent` or `candidates`, or a
  `claim` written as a node key object, refuses the whole request the same
  way. FSL computes those members, so a caller can never forge an item's
  request identity.
- An item that carries `temporal.fairness` refuses the whole request the same
  way. The fairness set of a temporal item is the resolved fairness of its
  checked clause; the source is its one authority.
- A `format` other than `native-analyze/1` handed to the analyze reader, or a
  `native-analyze/1` document handed to the run or compile reader, refuses at
  the request stage with `invalid-request`, exit 20.

### Subject, compilation and default items

- The program compiles through S1 to S4 with the request's models,
  libraries and refinement internals, exactly as FR-027 compiles it; a
  selected package is admitted by FR-027's reader with no fallback. A compile
  refusal refuses the whole request at its stage (FR-624).
- When the request has no `subject`, the default subject's initial states are
  every provisioned snapshot whose anchor kind is `initialization`, in
  ascending `{authority, identity}` order, its universes are the complete
  populations those snapshots declare, and its proof bounds are empty.
- A request's `subject.universes` is a list of `{authority, identity}`
  references, each naming one provisioned snapshot document. The subject's
  universe for a population is the set of object keys that population holds
  in the referenced snapshots, unioned in reference order. A reference that
  names no provisioned snapshot refuses the request `invalid-request`
  naming the entry; a population a referenced snapshot marks incomplete
  refuses `incomplete_population` naming it, because a universe is a
  complete population by definition (FSpec FR-331).
- When the request has no `items` member, the items are one item per checked
  claim of the program in source order across all declaration families (the
  program's units in request order, and in each unit its declarations by the
  start byte of their span), so a reader maps each default item to the source
  line it came from, one per kind its declaration
  family admits in FSpec FR-290's vocabulary order, each with the published
  default of every setting, followed by the derived items FR-368 derives for
  each model subject, in FR-368's position order. An empty `items` array is a
  request with no items, and its outcome is FR-621's empty `analyze`
  document.

### Resolving items

- Each item's `claim` SHALL resolve, by name lookup in the checked package,
  to the occurrence key `ClaimItem::new` (FR-281) takes. A name that resolves
  to no checked declaration settles that item `invalid-request` with
  `invalid_capability`/`unknown-claim`, naming the name; the request's other
  items still run.
- A kind that does not apply to the claim's declaration family settles the
  item by FR-507's `inapplicable-kind`. An absent kind on a family that
  admits several kinds settles the item `invalid-request` with FSpec FR-290's
  `absent-kind`.
- The item's `request_index` is its zero-based position in `items`, and the
  derived items FR-368 adds follow the caller's items. Its `extent` is the
  checked package's classification of the claim (FSpec FR-331 "Items").
- The driver computes each item's candidates from its registry (FR-075,
  FR-635), settles each item with negotiation, and calls `analyze` with the
  negotiated items, exactly as FR-281 "Negotiated items" states. FSL's reader
  computes no candidate and performs no negotiation.

### Budgets, deadline and cancellation

- Each item's own limits are the FSpec FR-331 members of its form
  (`temporal.limits` and `temporal.max_depth`, `probabilistic.limits`,
  `hybrid.limits`, `check.limits.max_cases`). An absent limit takes the
  published default of the descriptor of the engine the item runs on (FSpec
  FR-461). No item inherits a limit from another item or from the request.
- The request-level `limits` set only what the whole run shares: the
  accounting limits of every evaluation and the observation and normalization
  limits the subject is admitted under (FR-281 `AnalyzeLimits`). The wire
  names of `limits.observation` and `limits.normalization` are the FR-255
  setting names of those limits without their stage prefix: `normalization`
  takes `declaration_records`, `derivation_facts`, `effective_declarations`,
  `dispatch_candidates`, `hashed_bytes`, `work_units`, `ancestor_steps` and
  `family_steps` (the `model.*` rows), and `observation` takes the
  `observation.*` rows' names, each a JSON integer from 0 to 2^53 - 1 with the
  setting's published default when omitted. An unknown member refuses
  `invalid-request` naming it.
- `deadline_ms` arms `AnalyzeLimits::deadline`. Cancellation is not a
  document member: the driver's cancel handle (FR-287) is the `&Cancel` the
  run takes. A cancellation or deadline observed before the first item runs
  settles the whole outcome incomplete with the handle's cause, exit 22; one
  observed while items run settles each open item `incomplete` with the
  cancellation or deadline cause and keeps every record already settled, as
  FR-281 states.

### Engines

- `Engines::fsl()` SHALL hold every layer-A engine `fsl-analyze` builds, each
  under its own `BackendId`, including EN-7's closed-form schedulability
  engine under `en-7` (FR-246, FR-247) and EN-6's zone search, so that a
  schedulability item routed through the registry reaches EN-7.

### Library surface

- `fsl_command::command::prepare_analyze(path: &Path, limits: RunLimits)`
  SHALL return `Result<PreparedAnalyze, Box<RunError>>`, reusing FR-026's
  intake and FR-100's bounded request decoder. It performs no compiler,
  evaluator, engine or provider call. `PreparedAnalyze::into_input()`
  returns the owned source, package selection, domain packages, libraries,
  refinement internals, document provision, memory selections, default
  subject, item selections, `AnalyzeLimits` and deadline.
- `fsl_analyze::analyze::document::resolve_items` SHALL take the checked
  package, the default subject and the item selections, and return per item
  either a `ClaimItem` or the per-item disposition this requirement gives.
- `ClaimItem::request_item_wire` SHALL write the item's canonical FSpec
  FR-331 request item, with `request_index`, the `claim` occurrence key, the
  `extent`, the `candidates` and every limit made explicit. FR-621 echoes it
  in the outcome.

## Acceptance Criteria

The Counter fixture: domain package `test/counter` (population `counters` of
`Counter`, field `value: Int[0, 3]`, operation `inc` with
`pre CanInc { self.value < 3 }` and `post Inc { self.value - pre(self.value) = 1 }`),
the unit selecting it as `M`, the clause
`temporal Bounded using t0 over (c: M::Counter) clock "model-steps" on origin { always holds(c.value <= 3) }`,
and one initialization snapshot `start` holding counter `a` at 0.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-620-AC-1 | A `native-analyze/1` document over the Counter fixture with the one item `{"claim": "Bounded"}` is read; its outcome has one item whose `request` member is the FSpec FR-331 request item with `request_index` 0, the clause's occurrence key, kind `temporal-satisfaction`, `item_kind` `formula`, a `bounded` or `unbounded` extent as the package classifies it, and every `temporal.limits` member at EN-1's published default. | Test (TC-1580) |
| FR-620-AC-2 | Rewriting that `request` member as a document item (removing `request_index`, `extent` and `candidates`, and writing the claim by its name) and reading it again gives an equal `request` member: the item round-trips. | Test (TC-1580) |
| FR-620-AC-3 | An item carrying `request_index`, `extent`, `candidates`, `temporal.fairness` or an unknown member, and a request carrying an unknown member, and a request whose `deadline_ms` is 9007199254740992 (2^53), each refuse the whole request at the request stage with `invalid-request`, exit 20, naming the member, and run no item. | Test (TC-1580) |
| FR-620-AC-4 | A document with the items `{"claim": "Bounded"}` and `{"claim": "Missing"}` settles the second item `invalid-request` with `invalid_capability`/`unknown-claim` naming `Missing`, settles the first `proved`, and exits 20. | Test (TC-1580) |
| FR-620-AC-5 | The Counter document without `items` gives one item per checked claim in declaration order followed by the subject's derived deadlock-freedom item. | Test (TC-1580) |
| FR-620-AC-6 | `deadline_ms` 0 settles every item `incomplete` with the deadline cause and exits 22; the same document without `deadline_ms` settles `Bounded` `proved`. | Test (TC-1580) |
| FR-620-AC-7 | Two items over `Bounded`, the second with `temporal.limits.max_states` 1, settle `proved` and `incomplete` `LimitReached{max_states}` respectively: the second item's limit does not reach the first. | Test (TC-1580) |
| FR-620-AC-8 | `Engines::fsl()`'s registrations include `en-7` advertising `schedulability`. | Test (TC-1580) |
| FR-620-AC-9 | A `native-analyze/1` document handed to the run reader, and a `native-run/1` document handed to the analyze reader, each refuse at the request stage with `invalid-request`, exit 20. | Test (TC-1580) |
| FR-620-AC-10 | A Counter request with no `items` whose unit declares `BelowThree` before `Bounded` gives default items in that order, then the derived deadlock-freedom item; a `subject.universes` entry naming a snapshot the request does not provision refuses `invalid-request`, exit 20, naming the entry; `limits.normalization` `{"work_units": 5000000}` is admitted and `{"bogus": 1}` refuses `invalid-request` naming `bogus`. | Test (TC-2185) |

## Dependencies

- FR-281 owns the `analyze` operation, negotiation, settlement and the engine
  refusal table this reader feeds.
- FSpec FR-331 owns the request-item members; FSpec FR-520 owns the document
  wire shared with other tools.
- FR-026 and FR-027 own the shared envelope, source selections and the
  compile path.
