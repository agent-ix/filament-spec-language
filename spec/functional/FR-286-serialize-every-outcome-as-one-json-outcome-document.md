---
id: FR-286
title: "Serialize every library outcome as one JSON outcome document"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-095
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-277
    type: references
  - target: "ix://agent-ix/filament-specification/FR-331"
    type: references
  - target: "ix://agent-ix/filament-specification/FR-413"
    type: depends_on
  - target: "ix://agent-ix/filament-specification/FR-300"
    type: depends_on
  - target: "ix://agent-ix/filament-specification/FR-301"
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-283
    type: depends_on
---
# FR-286: Serialize every library outcome as one JSON outcome document

## Description

Each FSL lifecycle operation's outcome SHALL serialize to one JSON outcome
document (ADR-029 CB-4). A CLI in machine mode writes exactly that document
to stdout, so the same request through the library and through the CLI gives
the same document (FSpec FR-300-AC-3). The document's members are:

- `format`: the document's format identity, `filament-outcome/1`.
- `operation`: the library operation that produced it.
- `last_stage`: the last ADR-011 stage the operation reached, or `null`
  when no stage is known: a cancelled outcome that stopped before its first
  stage, or an internal-failure (fault) outcome.
  The stages are ordered `S1`, `S2`, `S3`, `S4`, `S5`, `S6a`, `S6b`, `S6c`,
  `S7`, `S8`, ADR-011's numbering; an operation whose work enters several
  stages writes the greatest it entered. A refusal of the request document
  itself, before any source is read (request intake), writes `null`.
  FR-621 "Last stage" gives every `analyze` outcome's stage and "Monitor
  documents from clause runs" every `monitor` outcome's.
- `category`: the ADR-013 O-16 category of the whole outcome, as the one
  `Category` type (FR-285); each item's category is the same type.
- `items`: for `prove`, `analyze` and `monitor`, one entry per requested item
  with its category, and for an `execute` outcome of a call carrying
  `requirements`, one entry per settled requirement item of the call (FR-963). The member is always present;
  it is an empty array for every other operation and for an outcome with no
  items. Each item that has a terminal record, every proof result, carries its
  FSpec FR-331 terminal record with its label
  and cause in FR-331's wire spelling (its wire-spelling table): for example
  `undefined-evaluation{where, cause}` and `replay-parity`. A record settled by
  an `unsupported` or `invalid-request` disposition (FSpec FR-331
  `settled_by` `disposition`) carries its FR-272 diagnostic as the record's
  `diagnostic` member, with code, cause and payload, and an `unsupported`
  record carries exactly one of `cause` and `diagnostic`, so a disposition's
  record carries the diagnostic and no `cause`. A `monitor` item
  built from a clause run that did not settle (pending or indeterminate)
  carries the clause-run record, with the `clause` member, and no FR-331
  terminal record (see "Monitor documents from clause runs"). The scalar-parity
  cause `ScalarAgrees` (FR-357) is spelled `scalar-agrees` and carries its kind
  only: the claim identity and outcome stay on the typed `ScalarAgreement`. A proof item (`prove`,
  `analyze`) never carries the label `undefined`: an undefined claim
  evaluation is `refuted` with cause `undefined-evaluation{where, cause}`,
  category violation (FR-281). A `monitor` clause whose evaluation over the
  trace is undefined is a violation with that cause (FR-283). The label
  `undefined` belongs to the category undefined and to no other category: it
  appears on a non-proof evaluation outcome, such as an `execute` outcome, and
  on a `monitor` item for FR-283's `InsufficientData` (category undefined,
  reason `insufficient-data`), and on no item of any other category (FR-285).
- `diagnostics`: each with its typed cause, catalog code, `Locus` (ADR-013
  T-5, FR-095), message, `details` ("Diagnostic details"), the `construct`,
  `declaration`, `profile` and `text` a forms refusal names, and, when the
  hint catalog has a row for its code and cause, `hint` ("Fix hints"); the
  "Diagnostic members" table lists them. A source `blank-label` cause also carries
  `label`, exactly `authority` or `identity` from the source owner's typed
  `SourceLabel`. Every other source cause omits `label`, including
  `empty-path`. A source refusal with no region has `locus` `null`, never a
  fabricated position at byte zero.
- `result` of a compile: see "Compile results".
- `artifacts`: always an empty array `[]`. No operation writes an artifact
  identity into the outcome document: `check` produces a `CheckedPackage` and
  no `package_id` (ADR-029 OP-1).
- `result`: the evaluation result of an `execute` outcome, the outcome
  FR-100's `spine-run-result/1` member `outcome` carries for a completed,
  undefined or incomplete call. It is `null` for every other operation, except a
  `parse` outcome whose S1 and S2 complete, which carries FR-663's `parsed`
  report ([FR-663](FR-663-report-the-parsed-unit-and-the-locus-of-every-refusal.md)),
  a `format` outcome whose formatting completes, which carries
  `{"kind": "formatted", "text": T}` with `T` the exact formatted source as a
  JSON string (FR-003), so the JSON form of a format run holds the same text
  its plain-text form writes. A `replay` outcome whose replay runs carries
  `{"kind": "replayed", "results": [R, ...]}`, each `R` one row of the FSpec
  FR-323 `results` document FR-862 returns, unchanged and in row order, so its
  `arm`, `settlement`, `category`, `value`, `charges` and other FR-072 members
  are all present;
  `OutcomeDocument::from_replay` builds it from the replay result, so the
  replay's outcome document and its text carry the arm and settlement the
  results document carries. A refused replay carries `null`. The library's format outcome owns this member:
  `OutcomeDocument::from_format` builds it from the completed format before
  serialization, and the CLI writes the document unchanged, adding no member
  (driver FR-002-AC-3). Formatting is idempotent, so formatting `T` gives
  `T` again. A refused format outcome carries `null`, and so does an
  `execute` outcome that is refused (its refusal record is a
  diagnostic), cancelled or a fault. Otherwise it is exactly one of:
  - `{"kind": "completed", "value": V}`, where `V` is the completed value
    in FR-650's run value form, the form FR-100's `spine-run-result/1`
    writes: `{"boolean": true|false}`, `{"integer": "<ASCII decimal>"}`
    (FR-038's integer spelling), the Reference value
    `{"reference": {"universe": U, "type": T, "object_identity": O}}`, or
    FR-650's form of every other kind. For a Reference completion, `U` and `T` are the actual returned shared
    reference's universe and most-specific effective type, each as the structural
    identity object of FR-650 "Identity spellings"; `O` is its exact nonempty UTF-8
    object identity, without normalization, reminting or substitution by an
    input reference. It carries neither object state nor a Population binding. A clause
    evaluation still completes with its `Boolean` value.
  - `{"kind": "undefined", "reason": "<reason>"}`, `reason` spelled as
    FR-100's undefined tables spell it, such as `sum-out-of-domain` or
    `precondition-false`.
  - `{"kind": "incomplete", "limit": {"kind": K, "bound": B, "counter": C,
    "field": F, "setting": S}}`, the exhausted limit as FR-277 names one: `K` the limit
    kind, the exhausted counter's `filament.value.accounting/v1` member name
    (such as `work_units`); `B` the configured value; `C` the counter at the failed
    charge as FR-277 defines it (the consumed value plus the denied amount for
    a cumulative counter, the denied size for a high-water counter); `F` the name of the `execute`
    limits field that sets the bound, which for an accounting counter is the
    `ScalarLimits` field of the same name; `S` the FR-255 setting that
    raises it, which for an accounting counter is the counter name `K`. `B` and `C` are
    ASCII decimal strings in FR-038's integer spelling.
  - `{"kind": "incomplete", "reasons": [R, ...]}`, for FR-100's typed
    call-admission incompleteness, where each `R` is the exact typed reason
    record and text-valued fields FR-100 defines. It carries no `limit` member;
    FR-285 maps it to incomplete, exit 22.

## Monitor documents from clause runs

A `monitor` document is built from clause runs. The producer lives with the
outcome types in `fsl-replay` (the one renderer a core crate may hold,
FR-284): `OutcomeItem::from_clause_run(request_index, clause, &ClauseRunReport)`
builds one item from one FR-109 / FR-330 clause run, and
`OutcomeDocument::from_clause_runs(last_stage, runs)` builds the document
with operation `monitor` through `OutcomeDocument::settled`, one item per
selected clause in request order. `fsl-inspect` and the driver read the
document and build no item of their own. The `monitor` operation's records
(FR-283) are rendered into items through this producer, and no other code path
builds an item for a clause run.

A `monitor` document's `last_stage` is `S6a`, where FR-330 and FR-109
evaluate each clause, for a document of settled clause runs, whatever their
verdicts, and for a whole-request refusal that arises while input is admitted
to or written from that evaluation: a trace position's admission
(`Admission`), a clause refused at stage `admit` (`ClauseRefused`), a
record's missing source (`SourceNotSupplied`), a session's observation,
progress contradiction or checkpoint refusal (`Observation`,
`Contradiction`, `State`). It is `S4` for a selected name that resolves to
no temporal clause of the linked package (`UnknownClause`); the refusing
stage for a compile refusal; and `null` for a defective trace document
(`TraceDocument`, request intake), a cancelled request and an internal
failure (an evaluator fault, `TemporalRefusal::Fault`, FR-283), which
FR-286's fault rule settles as category internal-failure with no stage.

Each item takes its category from `ClauseRunReport::category()` (the catalog's
`Category`, FR-285) and from nowhere else, and its `result` label from that
category:

| Category of the run | `result` label | `cause` |
| --- | --- | --- |
| success (`Completed(true)`, FR-283's `Tested`) | `tested` | null |
| violation (`Completed(false)`, FR-283's `Violated`) | `refuted` | null: FR-331 gives a `refuted` result a cause only for `undefined-evaluation` |
| violation (an `Undefined` evaluation, FR-283's `Undefined`) | `refuted` | `undefined-evaluation{where, cause}` |
| pending (a finite-trace or finite-window `Pending`) | `pending` | null |
| indeterminate (FR-501's `Indeterminate`) | `indeterminate` | FR-501's reason, as its result report spells it |
| incomplete | `incomplete` | the exhausted limit |
| unsupported (for example a missing fairness premise) | `unsupported` | the refusal cause |
| refusal | `declined` | the refusal cause |
| undefined (FR-283's `InsufficientData`) | `undefined` | `insufficient-data` |

A `monitor` item carries the members of FR-283's record for its verdict, besides
`clause`, `result`, `category` and `cause`:

| Verdict | Members besides `clause`, `result`, `category` and `cause` |
| --- | --- |
| `Violated` | `position`; `counterexample`, FR-283's `TemporalCounterexample` with its `prefix`, its `loop` (`[]` for a finite trace, as FR-330 and FSpec FR-364 spell it) and `trace_position`; for a temporal violation `decisive` (FR-283, with its `position` member) when the report holds one, a sibling of `counterexample`; `witness`, the FSpec FR-351 record, only when `ClauseRunReport::witness` holds one, never for a temporal `Violated` or `Undefined` record |
| `Undefined` | `position`; `counterexample` as for `Violated`; no `witness` |
| `InsufficientData` | `position`, `count` and `min_count`; no `counterexample` and no `witness` |
| `Pending`, `Tested` | none |
| `Indeterminate` | `position` where FR-501's reason names one |
| `Declined` at stage `select` | `diagnostic` `{code, cause, detail}`: the refusal's catalog code and cause, and its FR-272 payload as `detail`, so the selected name and, for a non-`Boolean` function, its expected and actual result types reach the document ([FR-867](FR-867-name-the-function-and-its-result-type-when-a-non-boolean-function-is-selected-as-a-claim.md)) |
| `Declined` at any other stage, `Unsupported`, `Incomplete` | none |

`pending` and `indeterminate` are catalog labels (`Category::as_str`), not
FR-331 terminal-record labels: a clause run that did not settle has no terminal
record, so these two items carry no FR-331 terminal record, only the clause-run record, and the label is the catalog's.
Every item built from a clause run also carries `clause`, the clause's
qualified name, so that a consumer names the pending clauses (FR-285's status
`pending` and the ids of the pending clauses) from the document alone. An
evaluator fault is not an item: it fails the whole request (FR-283).

The document's category is the most severe item's under FR-285's order, so a
`monitor` document whose only item is a pending clause has category pending,
exit 0, and a document with a pending and an indeterminate item has category
indeterminate, exit 11, and with a violated item beside either it has category
violation, exit 10.

`filament-outcome/1` is the outcome document of every operation, `execute`
included; it replaces FR-100's `spine-run-result/1` when the driver's verbs
land (ADR-029 CB-1).

The serialization lives with the outcome types in the core; it is the one
renderer a core crate may hold (FR-284).

## Analyze items and their counterexamples

An `analyze` or `prove` item is built from one terminal record and, for a
`refuted` record, its one `ItemCounterexample` (FR-281, FR-280):
`OutcomeItem::from_analyze(record, counterexample) -> Result<OutcomeItem,
OutcomeItemError>`. The item carries the record's label, cause and, for a
disposition, `diagnostic` as above. A `refuted` item carries `counterexample`
in the wire spelling of the member of FSpec FR-331's `counterexamples` entry
that the `Counterexample` variant is:

| Variant | Spelling |
| --- | --- |
| `Temporal` | FSpec FR-364's `TemporalCounterexample`: `trace`, `kind` (`formula`, `deadlock`, `undefined-evaluation`, `race`, `time-lock` with its `final_delay`, or a refinement's `refinement_failure`), `initial_state`, `prefix`, `loop` (`[]` for a finite trace), `over_binding`, `fairness`, `interval` and `trace_position`; the monitor `Violated` row's `counterexample` (FR-283) is the same record |
| `Probabilistic` | FSpec FR-413's `ProbabilisticCounterexample` (FR-202's witness scheduler and path set) |
| `Hybrid` | FSpec FR-193's `HybridCounterexample`: `type` `hybrid-counterexample`, its `kind` and `HybridTrace` |
| `Hyper` | FSpec FR-400's `HyperCounterexample` |
| `Graph` | FSpec FR-393's `GraphEvidence` |
| `Assignments` | FSpec FR-197's canonical assignments `{type: "assignments", values}`, one FR-181 typed value per parameter node |
| `Contract` | FSpec FR-485's `contract-counterexample` `{type, obligation, violated, bound, executions}` |
| `ProtocolAnalysis` | [FSpec FR-331](ix://agent-ix/filament-specification/FR-331)'s `ProtocolAnalysisCounterexample`, the closed `CanonicalCounterexample` alternative with `type` `protocol-analysis-counterexample` and `kind` `indistinguishable-histories`, `environment-defeat` or `projection`; its members and nested encodings are exactly FR-331's |

An item has none of `position`, `decisive` and `witness`, which belong to a
clause run. An item of any result but `refuted` carries no `counterexample`.
`from_analyze` given a `refuted` record and no counterexample returns
`Err(OutcomeItemError::MissingCounterexample)` and builds no item. One
spelling serves the document `analyze` writes and the item `prove` writes, so
the same item through either gives the same bytes.

### Protocol-analysis refutations

When a realizability or local-projection analysis settles `Refuted`, the
shared analyze item producer SHALL carry its typed counterexample in the
`ProtocolAnalysisCounterexample` alternative above. The producer SHALL
associate that body with the originating request item, preserving its
`request_index`, declared claim and admitted request, through the same
record, item, spelling and counterexample-taking path used by other
refuted analyses. Source compilation and checked-Package intake SHALL use
that same assembly contract.

For `IndistinguishableHistories`, the body SHALL preserve the analysis
`key`, `role`, both plays `history_a` and `history_b`, both required actions
`action_a` and `action_b`, the violated `obligation` and the
`separating_witness`. For `EnvironmentDefeat`, the body SHALL preserve the
analysis `key`, `defeats_per_branch` in branch order, each branch's `play`,
the violated `obligation` and the `separating_witness`. Each play SHALL
preserve its `initial` state and ordered steps, including every
`successor` and protocol `transition`. A send step SHALL carry its
`send_record`; every non-send step SHALL omit that member, never write
`null`. Transitions,
send records and the FSpec FR-351 separating witness SHALL use their
existing canonical encodings as FR-331 requires.

For a projection refutation, the body SHALL preserve the analysis `key`,
`role`, complete typed `decision` and `separating_witness`. A `choice`
decision SHALL retain its `choice`, `owner` and both indexed `cases`,
`paths` and nullable `actions`; an `acting-instance` decision SHALL retain
its `event`, `owner`, `instances` and `maximum`. The producer SHALL use
FR-331's decision and action spellings without adding realizability
strategy data to a projection body.

The resulting item SHALL retain its `refuted` terminal record and category
violation. The document SHALL retain `filament-outcome/1`, operation
`analyze`, the category ranked from its items and the last stage reached
under [FR-621](FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md).
A completed typed refutation SHALL NOT become a refusal, unsupported
result or internal failure because the result assembler omitted its body.
The producer SHALL NOT substitute graph evidence, fabricate a body or
drop typed data to fit another counterexample alternative. An absent body
still returns `MissingCounterexample` under the rule above; it SHALL NOT
be accepted as a complete refuted item.

The terminal record's `format`, `analysis`, proof basis and witness do not
constitute the counterexample body. [FSpec FR-352](ix://agent-ix/filament-specification/FR-352)
continues to exclude that body; the item carries it separately as
`counterexample` in FR-331's closed union. Structural validation of an
outcome document does not establish that its body belongs to that item or
truthfully encodes the typed refutation.

## Export outcomes

The `export` operation (FR-1015, FSpec FR-817) writes one outcome document
with `operation` `export` and FR-285's standard categories, read from the
export outcome's `status`:

| Export outcome | `category` | Exit |
| --- | --- | --- |
| `assembled` | success | `0` |
| `refused` by admission (a malformed member, an unknown profile or limit, a family mismatch, a duplicate obligation, an existing output path, a doubled destination) | refusal | `20` |
| `refused` with `below-fidelity-floor` | unsupported | `21` |
| `incomplete` (an `export.*` limit) | incomplete | `22` |
| a mapper failure | internal-failure | `30` |
| an output failure: the target file of an `assembled` export cannot be written | internal-failure | `30` |

An output failure is the export's counterpart of FR-862's results-row
output failure: the export was made, and writing its named file failed (a
missing parent directory, a denied permission, a full device). The document
carries one diagnostic with `code` `output-failure`, `cause`
`output-failure` and `details` `null`, `result` `null`, and `last_stage` the
greatest stage the export reached (S4 for an export of a checked package),
not `null`: unlike a replay row, the failure follows a completed compile,
and the stage tells a caller the program was checked and only the write
failed. It is internal-failure, exit `30`, not refusal, because the request
was admitted and well formed, and FR-285 ranks an output failure with every
other failure of the implementation. A file found already at the path
remains an admission refusal, exit `20`, whether it was found before the
compile or when the file is created, so a caller tells a path conflict from
a write failure by the code.

Its `result` is `{"kind": "exported", "export": E}`, `E` the FSpec FR-817
`filament.export-outcome/v1` document exactly as the run outcome's `export`
member carries it, for every status the mapper reports. A refusal before the
mapper runs carries `result` `null` and its diagnostic, as every other
refused operation does. `last_stage` is the greatest compile stage the
export's program reached (S4 for an export of a checked package), or `null`
for a refusal of the request itself, an existing output path included.
`items` is empty and `artifacts` is `[]`: the target file is the caller's
named output, never an artifact identity.

An evidence intake through the same verb (`fsl export --evidence`, FR-1022)
writes a document with `operation` `export`, one item per named obligation
carrying its FSpec FR-331 terminal record in the item spelling `prove`
writes, the category the items rank to, and `result` `null`.

## Inputs

A lifecycle operation's outcome value.

For a refusal before source admission, the driver uses
`OutcomeDiagnostic::from_source_diagnostic(&fsl_cst::diagnostic::CompleteDiagnostic)`.
This public producer-owned projection SHALL preserve the diagnostic's
catalog code, typed cause tag, optional source region and human message.
The projection SHALL obtain the `blank-label` payload from the source
owner's typed identity-cause fields; its optional label is a `SourceLabel`,
not a caller-supplied string.
It requires neither bytes nor an admitted source. The source arm of
`OutcomeDiagnostic::from_compile_refusal` uses this same projection, so
parse, format and compile-refusal documents retain the same identity
cause and label. Consumers do not inject a JSON member or reconstruct a
label from the message.

## Outputs

The outcome document's bytes.

## Behavior

- Each lifecycle operation's outcome type shall serialize to an outcome
  document with the members the Description lists.
- The same outcome value shall serialize to the same bytes every time.
- The `category` member shall be the outcome's O-16 category, from which
  FR-285 derives the exit code.
- The serialization shall write the label `undefined` on an item exactly when
  the item's category is undefined, so on no `prove` or `analyze` item and on
  a `monitor` item only for FR-283's `InsufficientData`.
- The serialization shall write each item's label and cause in FSpec
  FR-331's wire spelling.
- The serialization shall write `last_stage` `null` for a cancelled outcome
  that reached no stage and for a fault outcome.
- The serialization shall write the `items` member on every document, as an
  empty array when the outcome has no items.
- The serialization shall write a refusal that keeps several intake refusals
  or assembly errors as one `diagnostics` entry per kept refusal, intake
  refusals in FR-056's intake order (FR-056 "Admission") and assembly errors
  in the order assembly reports them, each with the locus, message and
  `details` FR-056 states, and the refusal's human message (its `Display`, which the CLI prints)
  shall join those refusals' messages one per line, the form a source
  refusal's message already takes; no entry is folded into a
  count.
- `OutcomeDocument::from_run` shall serialize `fsl_replay::spine::run`'s
  result as one `execute` document: a call that ran is `from_call`'s
  document with `artifacts` an empty array; a
  compile refusal carries the stage it reached, the refusal's category and its
  diagnostic; a call-stage refusal (FR-100) carries `S6a`, its category and its
  code; a cancelled run is incomplete with `last_stage` `null`, whichever
  stage the cancel stopped, and the cancel's cause; a
  fault is internal failure at no stage with its catalog code.
- A driver failure before any operation runs shall be one
  `OutcomeDocument::new(operation, None, category)` with one diagnostic. An
  engine the request names that the driver has not built (AOT or JIT, where
  only the interpreter exists) is valid meaning the selected producer does
  not implement, so its diagnostic code is FSpec FR-271's
  `unimplemented_capability`, never `unsupported_construct` (a source form
  the selected profile prohibits), with FSpec FR-272's cause
  `backend-not-supplied` and `backend` the engine's name (`aot` or `jit`), in
  category unsupported (FR-285, exit 21).
- `OutcomeItem::from_clause_run` shall give each item the members the
  "Monitor documents from clause runs" member table lists for its verdict,
  a `select` refusal's `diagnostic` included, and no other, reading `counterexample` from `ClauseRunReport::counterexample`
  (FR-330 Outputs; the same record FR-267 writes in its `temporal` member).
- `OutcomeItem::from_clause_run` shall write `decisive`, a sibling of `counterexample`, from `ClauseRunReport::decisive` (FR-330 Outputs) on a temporal `Violated` item whose report holds one, and no `decisive` on any other item.
- `OutcomeItem::from_clause_run` shall write `witness` exactly when
  `ClauseRunReport::witness` (`Option<SeparatingWitnessRecord>`, FR-266)
  holds a record, and shall write none otherwise.
- `OutcomeItem::from_clause_run` shall write no `counterexample` on an item
  whose report holds none.
- `OutcomeItem::from_analyze` shall write `counterexample`, in the spelling of "Analyze items and their counterexamples", on each `refuted` item, none of `position`, `decisive` or `witness`, and a `counterexample` on no other item.
- `OutcomeDocument::from_clause_runs` shall build a `monitor` document with
  one item per clause run, the item's category read from
  `ClauseRunReport::category()` and its label and cause as the "Monitor
  documents from clause runs" table states, the pending and indeterminate
  items included.
- A `monitor` item for a pending clause shall have `result` `pending` and
  `clause` naming the clause.
- A `monitor` document whose only item is a pending clause shall have category
  pending and exit 0.
- The serialization shall write an `execute` outcome's completed value,
  undefined reason or exhausted limit as the `result` member, and `result`
  `null` on every other document.
- Every writer of an FSL JSON document shall encode it through
  `fsl_foundation::canonical_json::encode`, the RFC 8785 canonical encoding,
  its object members in ascending UTF-16 code-unit order at every depth.
- For an admitted request, and for a refusal whose named defect does not
  depend on request order, the serialization shall give byte-equal output
  for every permutation of the request's object members.
- Every serialized outcome document shall validate against the published
  schema `urn:filament:outcome:1`.
- The published schema shall declare every member path a writer of the
  document can emit.
- If a document holds a JSON integer outside -(2^53 - 1) to 2^53 - 1, then
  `fsl_foundation::canonical_json::encode` shall refuse it with
  `CanonicalJsonError::UnsafeInteger` naming its pointer.
- The serialization shall write an item's `seed` as FSpec FR-331's
  `U64String`, a decimal string.
- When the hint catalog has a row for a diagnostic's code and cause, the
  serialization shall write that row's text as the diagnostic's `hint`.
- When the hint catalog has no row for a diagnostic's code and cause, the
  serialization shall write no `hint` member on that diagnostic.
- If a hint catalog row names a code and cause no typed FSL cause produces,
  repeats a pair, or has text outside the hint form, then
  `check_hint_catalog` shall refuse the catalog naming that row's pair.

### Diagnostic details

Every diagnostic SHALL carry `details`: `null`, or one closed object that
locates the refusal inside the request and the call, so a consumer of the
outcome document reads the same facts a consumer of FR-100's command-error
envelope reads, and the driver can show them.

- An `execute` diagnostic from a refusal FR-100 writes at stage `call`
  (FR-100 "Refusals before S6a", FR-650's argument table and FR-650-AC-6's
  unsupported parameter and result types), and from the runtime-invariant
  fault at stage `call`, SHALL carry exactly the `details` object FR-100's
  command-error envelope carries for the same request, member for member and
  value for value.
- An `execute` diagnostic from a request-stage refusal of an argument
  `value` (FR-650 "Arguments") SHALL carry `{"path":
  "/call/arguments/<index>/value"}`, the pointer the envelope names.
- An `unimplemented_capability` diagnostic with cause `unimplemented-meaning`
  SHALL carry `{"capability": C}`, where C is the capability name required
  by FSpec FR-272, matching the export refusal when one is present.
- An `invalid-request` diagnostic at stage `request` that FR-1047 writes for
  a request lacking members SHALL carry `{"missing": [...]}`, and one it
  writes for a `format` the entry does not read SHALL carry `{"path":
  "/format", "accepted": [...]}`.
- Every other diagnostic, a source, profile or compile-stage diagnostic
  included, SHALL carry `details` `null`: its `locus`, `cause` and payload
  members already locate it.

The object's members come from one closed vocabulary, each with one
meaning and one JSON kind, and no other member is written:

| Member | Kind | Meaning |
| --- | --- | --- |
| `function` | string | the request's `function` string, exactly as supplied |
| `position` | integer | the zero-based position of the declared parameter |
| `path` | string | an RFC 6901 JSON pointer: inside the argument's `value` for a call-stage defect (the empty pointer for the value itself), or to the request's `value` member for a request-stage one; for a request-format refusal (FR-1047) the pointer `/format` of the request envelope |
| `missing` | array | the request members FR-1047 finds absent, in FR-1047's member order: each element is an RFC 6901 pointer string for a required member, or an array of pointer strings for a one-of group |
| `accepted` | array of strings | the request formats the entry reads, in FR-1047's order |
| `parameter` | string | the parameter name an argument names or lacks |
| `model_identity` | string | the offered document's model identity |
| `population` | object | the offered population's DeclarationKey `{"package", "node"}` |
| `expected` | object | the required DomainPackageRef `{"identity", "version"}` |
| `member` | string | the uncovered member of a foreign type |
| `type` | object | the uncovered type's DeclarationKey |
| `required` | object | the required universe, as FR-650 "Identity spellings" writes a universe |
| `supplied` | object | the supplied universe, spelled the same way |
| `stage` | string | the fault's stage identifier |
| `invariant` | string | the fault's invariant identifier |
| `capability` | string | the valid meaning the selected producer does not implement, for FSpec FR-272's `unimplemented-meaning` cause |

`missing` is written alone, and `accepted` only with `path` `/format`; both
are additive to the vocabulary, so a document that carries neither stays
valid. Each FR-100 row fixes which members its object holds; the members are
written in RFC 8785 order with the document. A `details` object is never
empty, and a refusal whose row has no `details` writes `null`.

### Analyze item members

An `analyze` document also carries FR-621's `aggregate` member and FR-635's
`registry` member, and each `analyze` item carries the members FR-621 lists
(`request_index`, `claim_name`, `request`, `record`, `certificate`,
`counterexample`, `replay`, `accounting`, optional item-level `diagnostics`,
and `axes`) or, for a simulation item, FR-634's members. Item-level
`diagnostics` are distinct from the document-level `diagnostics` array and
from FSpec FR-331's terminal-record `diagnostic`. A clause-run and a
`monitor` item carry FR-622's `axes`.

### Diagnostic members

A `diagnostics` entry is written by `OutcomeDiagnostic` and holds these
members, and no other:

| Member | Presence | Meaning |
| --- | --- | --- |
| `cause` | always, a string or `null` | the typed cause tag |
| `code` | always | the catalog code |
| `construct` | a forms refusal that names one | the refused construct's form |
| `declaration` | a forms refusal that names one | the declaration that holds the construct |
| `details` | always, an object or `null` | "Diagnostic details" |
| `hint` | when the hint catalog has a row for the code and cause, and for a `missing_import` diagnostic of cause `missing-selection` the row of its `selection` | "Fix hints" |
| `selection` | a `missing_import` diagnostic of cause `missing-selection`, except the one of plugin replay's defensive `UnresolvedDependency` | "Missing selections" |
| `label` | a source `blank-label` cause only | `authority` or `identity` |
| `locus` | always, a `Locus` or `null` | FR-095 |
| `message` | always | the human message |
| `profile` | a forms refusal that names one | the profile that prohibits the construct |
| `text` | a forms refusal that names one | the construct's source text |

### Compile results

The `package` outcome document that `fsl_command::command::compile_outcome*`
returns for a success SHALL carry `result` `{"kind": "packaged", "package":
{"bytes": N, "output": P, "handoff": D}}`, so the document alone says what
the compile produced and where it went; FR-865's text form is written from
it. `N` is the length in bytes of the package FR-027's `CompileOutcome::
package_bytes()` returns, as a JSON integer. `P` is the path of the package
file the compile wrote for the request's `output` member or for
`RequestOverrides::output` (FR-810), or `null` when it wrote none. `D` is the
path of the handoff directory written for the request's `handoff` member
(FR-665), or `null`. Each path is the path the command opened, as a string:
the `output` member and the `handoff` `directory` member joined to the
directory of the request file (the request path's parent, so a request
`reqs/c.json` with `output` `{"package": "p.json"}` writes `reqs/p.json`; no
join when the parent is empty), and an `--output` option as given; a path
that is not UTF-8 is written as `Path::to_string_lossy` writes it (one
U+FFFD per maximal invalid subsequence). A compile that
is not a success, a `check` outcome and `OutcomeDocument::from_package` over a
checked package carry `result` `null` as before. `artifacts` stays `[]`
(FR-810).

### Missing selections

The members `selection` and `result` `packaged` are additive changes to
`filament-outcome/1`, which the architecture lead accepted on QSL-2937. An
earlier ruling kept the JSON document unchanged for a text-only hint; the
later ruling that the text form renders FSL's document alone superseded it,
so what the text says is carried by the document.

A `missing_import` diagnostic of cause `missing-selection` has nine kinds of producer
that refuse for nine different missing things, and one hint cannot be true for
all of them. Such a diagnostic SHALL carry `selection`, except the one of the
defensive producer named below, the kind of the thing
that is missing, one of the nine strings of this table; the hint catalog keys
that pair's rows by `selection` as well, so the JSON `hint` is the text of
the kind and the text form (FR-865) writes it unchanged. The producers are
every producer of the pair that writes an `OutcomeDiagnostic`; a typed
cause that writes the pair adds its row here in the same change.

| `selection` | What is missing | Producers (typed cause) | `hint` |
| --- | --- | --- | --- |
| `model` | the domain package a `model` declaration selects | fsl-semantics model intake `ModelRefusalCause::MissingSelection`; assembly `AssemblyCause::UnadmittedModel`; `check_outcome`'s refusal of an unsupplied `--model` (FR-866) | `Supply the model in the request's models member, or with --model <identity>=<file> on fsl check.` |
| `library` | the library an `import` declaration names | fsl-replay spine `ImportRefusal::MissingSelection`; assembly `AssemblyCause::UnsuppliedImport` | `Supply the library in the request's libraries member, or with --library <identity>=<file> on fsl check.` |
| `sibling-unit` | a library, where the import names a sibling unit | fsl-replay spine `UnitListRefusal::SiblingImport` | `Supply the imported unit as a library, not as a sibling unit.` |
| `unimported-library` | an `import` of the unit for the library a call site names | fsl-replay `CallSiteRefusal::UnknownImport`, reached through the compile request's `call_site` member (FR-027) | `Call a function of a library that the unit imports directly, or add the import.` |
| `function-owner` | an `import` of the unit for the request's `function_owner` | fsl-replay `ReplayRefusal::UnknownOwner` | `Name a function_owner that the program imports directly, or add the import.` |
| `byte-provision` | the byte-provision entry for a source the package reference names | fsl-replay request `IncompleteByteProvision` | `Add a byte_provision entry for the named source to the replay request.` |
| `extension-definition` | the definition of a selected extension | fsl-semantics library bundle `PackageError::MissingDefinition` (cause `ResolutionCause::MissingSelection`) and `ExtensionRefusal::MissingDependency` (cause `ExtensionCause::MissingSelection`) | `Add the selected extension's definition to the request's extensions member, or remove the selection.` |
| `source` | a source the package's graph names, absent from the `SourceProvision` the analyze call supplies | fsl-analyze `AnalyzeRefusal::Provision` (`ProvisionRefusal`), the whole-request refusal `fsl_analyze::analyze::analyze` writes as a document diagnostic | `Add the named source to the source provision the analyze call supplies.` |
| `package-reference` | the domain package or library a checked package's bytes name | fsl-package `PackageAdmissionRefusal` of code `missing_import` | `Supply the domain package or library that the package names, in the request's models or libraries member.` |

**Producers no command reaches (current).** `LibraryRefusal::MissingImport` is written only by `resolve_libraries`, which no production entry calls; it writes no outcome diagnostic and has no row. `PackageError::MissingDefinition` is a library-entry cause no command reaches today: a compile whose extension dependency is absent from `extensions` refuses first with `ExtensionRefusal::MissingDependency`, which carries `extension-definition`, and the profile-closure check that can write it (`check_with_profile_catalog`) has no outcome-document conversion. This is the current implementation gap, not a requirement; a diagnostic of either, once a command reaches it, falls under the section's rule above.

The `source` kind is reached by a caller of the library entry, who supplies
the `SourceProvision`; `fsl analyze` builds that provision from the units it
admitted and so cannot reach it. An analyze item's terminal record that is
settled `missing_import`/`missing-selection` (FR-1044's component package
that is no dependency, and FR-280's declined record per item for a source
absent from the provision, written by `provider::run`) is an item record,
not a diagnostic: it carries no `selection` and no hint, and this section
does not change it. `fsl-cst` lists the pair in its `CATALOG_PAIRS` and
writes no diagnostic of it. Exactly one diagnostic producer writes the pair
without a `selection`: fsl-replay plugin replay's
`PluginReplayRefusal::UnresolvedDependency`, a defensive check that
consistent inputs never reach and that names no caller-supplied member; it
carries the catalog row of the pair, `Import the package that supplies the
selected name.`

The catalog row of the pair with no `selection` stays for that one
producer and so that `hint_for(code, cause, None)` answers for each of the
23 pairs of the table below. `hint_for` takes the selection as a third
argument, `None` for every other pair.

### Fix hints

A diagnostic MAY carry `hint`: one sentence, in the imperative, naming the
change to the input that removes the refusal, for example `Declare the
name, or import the package that declares it.` The member is a JSON string
of 1 to 200 bytes of UTF-8 that holds no control character (U+0000 to
U+001F, U+007F and U+0080 to U+009F) and ends in `.`. It is optional: a
diagnostic whose code and cause have no catalog row has no `hint` member,
never an empty or placeholder string.

Every hint comes from one catalog keyed by the diagnostic's code, cause and,
for the `missing_import`/`missing-selection` pair, `selection`.
The catalog lives in the crate that holds `OutcomeDiagnostic`, the outcome
module of `fsl-replay`, as `fsl_replay::outcome::hint::HINTS: &[HintRow]`,
each `HintRow { code: Code, cause: Option<&'static str>, selection:
Option<&'static str>, text: &'static str }`, with the lookup `hint_for(code:
Code, cause: Option<&str>, selection: Option<&str>) -> Option<&'static
str>`. A row whose `cause` is `None` matches a diagnostic with no cause and
no other, and a row whose `selection` is `None` a diagnostic with no
`selection` and no other. `OutcomeDiagnostic` takes its `hint` from
`hint_for` when it is built, through every constructor; no producer,
consumer or driver writes, edits or removes a hint. The text is a function
of the code, cause and selection alone and holds no payload value: the message and the
payload members already name the offending name, type, path or limit, and a
static text keeps the document's bytes a function of the outcome and lets
one reviewer read every hint in one place. A row is written only when its
text is true for every diagnostic that carries its key, its code and cause
and, for the `missing_import`/`missing-selection` pair, its `selection`
("Missing selections"): a key
that several unrelated refusals share, such as `invalid-request` with no
cause (a malformed request document, an output path that exists, a member
given twice, an unknown option), has no row until those refusals carry
distinct causes.

Each crate that defines a typed cause exports the (code, cause) pairs its
causes produce as `pub const CATALOG_PAIRS: &[CatalogCode]`, the union of
its cause types' variants' `CatalogCode`s; `fsl-cst`, `fsl-forms`,
`fsl-semantics`, `fsl-route`, `fsl-eval`, `fsl-package`, `fsl-source`,
`fsl-replay`, `fsl-analyze`, `fsl-hybrid-reference`, `fsl-inspect` and
`fsl-command` each do, and so does `fsl-foundation` for its own causes.
Every one of these crates but the last four is a dependency of `fsl-replay`,
and `fsl-command` depends, directly or through `fsl-analyze`, on all of
them, so the aggregate `fsl_command::diagnostic_pairs()`, the union of every
crate's `CATALOG_PAIRS`, is the one place that sees every pair. A typed
cause added to a crate is added to that crate's `CATALOG_PAIRS` in the same
change.

`filament-semantic-value`, the external crate pinned in the workspace
manifest, also defines typed causes (its call, definition and admission
causes produce, among others, `missing_declaration`/`missing-name` and
`invalid_runtime_input`/`missing-member`) and exports no pair list. Its
pairs are contributed by `fsl_replay::outcome::hint::SEMANTIC_VALUE_PAIRS`,
a table in the hint module, included in `diagnostic_pairs()`. A test of
`fsl-replay` guards the table: it builds every variant of every
`filament-semantic-value` cause type through an exhaustive `match` (so a
new variant fails to compile until the test names it), and fails when a
variant's `CatalogCode` is missing from the table or a table row is
produced by no variant. When that crate exports its own list, the table is
replaced by it.

A cause written as a string literal at a call site, such as
`OutcomeDiagnostic::new(Some("missing-member"), ..)`, belongs to no typed
cause and adds no pair to `known`. Its pair is in the catalog's reach only
when a typed cause also produces it; a hint row for a pair that only string
literals write is refused as `UnknownPair`, which keeps the catalog tied to
the closed cause variants FSpec FR-272 lists.

The catalog is checked by `fsl_replay::outcome::hint::check_hint_catalog(rows,
known) -> Result<(), HintCatalogError>`, run with `known` the aggregate
`fsl_command::diagnostic_pairs()` as a test of `fsl-command`, so a catalog
that fails it cannot merge. The check refuses:

- `HintCatalogError::UnknownPair { code, cause }` for a row whose cause is
  not `None` and whose pair is not in `known`, such as a misspelled cause;
  a row with cause `None` names any code of `Code::all()`;
- `HintCatalogError::DuplicatePair { code, cause }` for a second row of one
  pair;
- `HintCatalogError::MalformedText { code, cause }` for a text outside the
  form above: empty, over 200 bytes of UTF-8, holding a control character
  such as a line feed, or with no final `.`. FR-865 writes the text on one
  line. The byte length is checked here, because a JSON Schema `maxLength`
  counts code points, not bytes.

The catalog ships a row for at least each of these 23 pairs. These are the
refusals a new user most often meets, one or more per class of mistake:

| Mistake | Code | Cause |
| --- | --- | --- |
| A misspelled keyword or a stray token | `invalid_syntax` | `unexpected-token` |
| An unclosed block or an unfinished declaration | `invalid_syntax` | `unexpected-end` |
| A character that starts no token | `invalid_syntax` | `invalid-token` |
| A malformed escape in a text literal | `invalid_syntax` | `invalid-escape` |
| A value of the wrong type | `ill_typed` | `type-mismatch` |
| Quantities of different units combined | `ill_typed` | `unit-mismatch` |
| An operator applied to a type it does not take | `ill_typed` | `operator-ineligible` |
| A clause whose body is not `Boolean` | `ill_typed` | `non-boolean-root` |
| A package used and not imported | `missing_import` | `missing-selection` |
| A name used and not declared | `missing_declaration` | `missing-name` |
| A required selection left out, such as the integer-division law | `missing_declaration` | `missing-selection` |
| A name two imports declare | `ambiguous_declaration` | `ambiguous-name` |
| A division whose divisor may be zero | `undefined_expression` | `unproved-nonzero` |
| A read of a value that may be absent | `undefined_expression` | `unproved-presence` |
| A declaration form the profile does not admit | `unsupported_construct` | `declaration-form` |
| An expression form the profile does not admit | `unsupported_construct` | `expression-form` |
| A valid form this build does not implement | `unsupported_construct` | `not-yet-implemented` |
| An unknown profile, law or other selection value | `unknown_profile` | `unsupported-selection` |
| A source over the input size limit | `stage_limit_exceeded` | `input-bytes-exceeded` |
| A run over its work budget | `stage_limit_exceeded` | `work-budget-exceeded` |
| A request document member left out | `invalid_runtime_input` | `missing-member` |
| A request document member the format does not have | `invalid_runtime_input` | `unknown-member` |
| A file that cannot be opened, read or written, such as a missing input, a missing output directory or a denied checkpoint write | `io-error` | none |

`io-error` has a row with no cause because every `io-error` diagnostic is a
file operation the operating system refused: a selected input that could
not be opened or read, or an output file or monitor checkpoint that could
not be written. Its text names both, for example `Check that the path
exists and that you may read it, or write to it for an output file.`, so it
is true for every `io-error` diagnostic.
A row is added for a pair whenever a journey test of the CLI finds a
refusal a user cannot act on from its message alone.

The hint belongs to the document's `diagnostics` entries. An item's
`diagnostic` member is FSpec FR-331's terminal-record diagnostic, a wire
shared with external providers whose closed member set the outcome does
not extend, so it carries no `hint`.

### Canonical encoding and key order

The machine-output documents are encoded by one function,
`fsl_foundation::canonical_json::encode<T: Serialize>(value: &T) ->
Result<Vec<u8>, CanonicalJsonError>`, which writes the RFC 8785 (JCS)
canonical encoding through `quire-canonical`. The rule covers the
`filament-outcome/1` document of every operation, the `analyze` document
`fsl-analyze` assembles (with its `aggregate` and `registry`), the
`filament.monitor-event/1` events, the `filament.monitor-summary/1` summary
and the `filament.monitor-run/1` document `fsl-command` writes (FR-644), and
the command-error envelope. `OutcomeDocument::to_bytes` calls it, and so
does every writer of these documents that builds one as a
`serde_json::Value`; none of them is encoded through `serde_json::to_vec` or
its own sort.

The rule does not cover files whose own requirement fixes another form. The
FR-517 checked-package handoff files (`inventory.json`, `selection.json`,
`identities.json` and their siblings) keep the pretty-printed form with
two-space indentation and the fixed member order FR-517 states, and its
exact-bytes acceptance criteria stay as written; those files are read by
their strict readers, not compared with an outcome's bytes.

#### Numbers

The encoder refuses, with `CanonicalJsonError::UnsafeInteger` naming the
member's pointer, any JSON integer outside -(2^53 - 1) to 2^53 - 1: RFC 8785
writes numbers as IEEE 754 doubles, so a larger integer would not survive a
round trip, and `quire-canonical` refuses it. A member whose value can
exceed that range is therefore written as a decimal string. `seed`, the
state-graph seed an item carries, is FSpec FR-331's `U64String`, the
canonical decimal string of a `u64` from `"0"` to
`"18446744073709551615"`, because FSpec FR-392 admits any `u64` seed;
`OutcomeItem` writes it as that string, and a simulation item's `seed` is
the same string. Counts, bounds and limits the Description gives as ASCII
decimal strings (`bound`, `counter`, exploration counts) stay strings. The
other unsigned integers of the document are JSON numbers, each bounded
below 2^53 by where its value comes from:

| Member | Bound |
| --- | --- |
| `request_index`, an item's `position`, `count`, `min_count` | a position or count over a request or trace held in memory, below 2^53 |
| `max_depth`, `witness_samples`, the values of a state-graph `limits` | request settings the request reader admits only from 0 to 2^53 - 1, as FSpec FR-520 admits `deadline_ms` and `limits.accounting`; a larger value refuses `invalid-request` naming the member |
| `details.position` | a parameter position, below the declared parameter count |

A member added to this table or to the document with a `u64` that can
exceed 2^53 - 1 is a decimal string, not a number.

RFC 8785 orders an object's members, at every depth, by their names'
UTF-16 code units, writes no insignificant whitespace, and writes each
number in its own form. The order is a property of the bytes, not of the
Rust type: a struct's field order, a map's insertion order and the order in
which a request spelled its members never reach the output. A diagnostic is
therefore written with its members in the order of the "Diagnostic members"
table: `cause`, `code`, `construct`, `declaration`, `details`, `hint`,
`label`, `locus`, `message`, `profile`, `text`, each present member in its
place.

UTF-16 code-unit order and UTF-8 byte order agree for every key except one
whose characters differ first at a supplementary character (U+10000 and
above, a surrogate pair from 0xD800) against a character from U+E000 to
U+FFFF: UTF-16 puts the supplementary character first, UTF-8 puts it last.
Keys a program or request names, such as an FR-650 record's field names,
can hold either. ix-cli-kit's `json` module orders keys by UTF-8 bytes, so
the driver writes the bytes FSL's encoder returns, unchanged, and never
re-encodes a document through a UTF-8-ordered writer.

The member order of an input request carries no meaning wherever the
refusal it gives does not depend on it. For an admitted request, and for a
refusal whose named defect does not depend on request order, every
permutation of the request's object members, at any depth, gives
byte-equal output. Two kinds of refusal depend on request order by their
own requirement and legitimately change with a permutation: a refusal that
names the first defect in request order, such as FR-1031's refusal "at the
first member in request order" within a row of its table, and a refusal that
keeps several intake refusals in FR-056's intake order.

### Published schema

The document's shape is published as one JSON Schema (draft 2020-12),
`fsl-replay/schemas/filament-outcome-1.schema.json`, with `$id`
`urn:filament:outcome:1`, beside the outcome types it describes, and exposed
to dependants as `fsl_replay::outcome::OUTCOME_SCHEMA_1`, the file's text
(`include_str!`), so the driver's and every other consumer's tests validate
against the same bytes the library is tested against. A consumer that reads
the document validates it against this schema; it does not keep a copy.

The schema describes every member every writer of the document emits, at
every depth. The writers and the members each contributes:

| Writer | Members |
| --- | --- |
| `OutcomeDocument` (every operation) | the root members `format`, `operation`, `last_stage`, `category`, `items`, `diagnostics`, `artifacts` and `result` of the Description; and `registry`, FR-635's registry, on an `execute` document of a call carrying `requirements`, whose `items` are FR-963's settled requirement items in FR-621's item spelling |
| `OutcomeDiagnostic` | the "Diagnostic members" table, with `details` from "Diagnostic details" |
| `OutcomeItem` (`prove`, `monitor`, clause runs) | `request_index`, `category`, `result`, `cause`, `diagnostic`, `basis`, `certification`, `measured`, `method`, `max_depth`, `seed` (a decimal string, "Numbers"), `witness_samples`, `limits`, `limit_reached`, `position`, `counterexample`, `decisive`, `witness`, `evidence`, `certificate_document`, `count`, `min_count`, `clause`, `axes`, and the FR-990 protocol conformance members `obligations`, `closure`, `global`, `max_messages`, `projections`, `recording` and `replay` |
| The `analyze` document (`fsl-analyze`) | the root `aggregate` (`status`, `gaps`, each gap `request_index`, `reason`, `result`) and `registry` (FR-635); each item's FR-621 members (`request_index`, `claim_name`, `request`, `category`, `record`, `certificate`, `counterexample`, `replay`, `accounting`, `exploration`, optional `diagnostics`, `axes`), `disposition` for a finite-replay item (FR-941), and `obligations`, `closure`, `global`, `projections` and `recording` (FR-990, FR-1032); the record's `settled_by`, `backend`, `non_zeno`, `memory_bound`, `trace` and `steps` |
| A `simulation` item (FR-634, `fsl-analyze` `simulation_item`) | `request_index`, `kind` (`simulation`), `simulation` (the admitted settings: `mode`, `seed` and `samples` for `sample`, `max_steps`, `max_states`, `max_state_key_bytes`), `category`, `seed` for a `sample` run, `traces`, `coverage` (`states`, `transitions`, `choices`), `findings` (always present, an empty array when none), `frontier`, and `cause` when the run stopped at a limit or could not simulate; no `record` |
| `check_fences` (`from_native_fences`) | FR-355's fence entries as `items` |
| `result` | each kind of the Description: `completed`, `undefined`, `incomplete`, `parsed` (FR-663), `formatted`, `replayed` (FSpec FR-323 rows), `exported` (FSpec FR-817), `packaged` ("Compile results") |

- A record whose spelling another requirement owns (an FSpec FR-331
  terminal record and its counterexample variants, an FR-650 run value, the
  FR-663 parse report, an FSpec FR-323 results row, the FSpec FR-817 export
  outcome, FR-635's registry) is described by a `$defs` entry named after
  its owner, such as `$defs/fspec-fr-331-terminal-record`, transcribed from
  that owner's spelling; where the owner publishes its own schema, the
  entry is a `$ref` to that schema's `$id`.
- Every object whose member names the outcome fixes is closed
  (`unevaluatedProperties: false`), so a member a writer emits and the
  schema does not list fails validation. A map whose keys are names a
  program or a request declares (an FR-650 record's `fields`, a
  counterexample state's values keyed by declared names, an accounting
  record's `limits` keyed by counter name) is described with
  `propertyNames` for the form of its keys and `additionalProperties` for
  the schema of its values, not as a closed object.
- Members that depend on `operation` are selected by `if`/`then` on
  `operation`, so an `analyze` document without `aggregate` fails, and a
  `check` document with one fails.
- `selection` is an optional diagnostic member whose values are the nine
  strings of "Missing selections", absent only on the diagnostic of the
  defensive producer named there, and `details` admits the members `missing` and
  `accepted` of "Diagnostic details"; the schema file declares all three in
  the change that adds them.
- `hint` is an optional diagnostic member with `minLength` 1 and the pattern
  `^[^\u0000-\u001F\u007F-\u009F]*\.$`; its 200-byte bound is checked by
  `check_hint_catalog`, since a schema counts code points.

The schema is complete when no writer can emit a member it does not
describe. Each writer declares its own member list, and the list is tied to
the writer so a new member cannot be added to one without the other:

- A typed writer (`OutcomeDocument`, `OutcomeDiagnostic`, `OutcomeItem` and
  the structs they hold) declares `const MEMBERS: &[&str]`, the names it can
  write. Its test builds a full-member fixture through a constructor that
  takes every field by exhaustive struct destructuring or a struct literal
  with no `..`, so adding a field fails to compile until the fixture sets
  it; the fixture sets every optional field to `Some`, serializes, and the
  test fails unless the emitted keys equal `MEMBERS` exactly.
- A writer that assembles a `serde_json::Value` (the analyze document and
  item, the simulation item, the monitor events and summary) declares
  `const MEMBERS: &[&str]` beside the function, and inserts each member
  through a helper that takes a name from `MEMBERS` (an index or a typed
  key), never a free string, so a member it emits is a member it declares.
  Its test runs the writer over fixtures that reach every branch and fails
  when an emitted key is not in `MEMBERS` or a `MEMBERS` entry is never
  emitted.

The completeness test then reads every writer's `MEMBERS`, at every
nesting level, as member paths (a JSON pointer with array indices and
declared-name map keys replaced by `*`), and fails naming each path the
schema does not declare. A member added to a writer reaches its `MEMBERS`
(through the fixture's compile error or the helper), and from there fails
the completeness test until the schema declares it.

The schema is versioned with the format. The file for `filament-outcome/1`
changes in the same change as the outcome type: a new optional member, such
as `hint`, is added to this file and keeps `format` `filament-outcome/1`. A
change an existing reader cannot read (a removed or renamed member, a
changed JSON kind, a new required member) is a new format,
`filament-outcome/2`, with its own file `filament-outcome-2.schema.json` and
`$id` `urn:filament:outcome:2`.

### Golden corpus

The schema is checked against a golden corpus,
`fsl-command/tests/fixtures/outcome-golden/`, one file per golden document,
named after the criterion or operation that builds it (`FR-286-AC-2.json`,
`FR-286-AC-5-2.json`, `select-cancelled.json`), holding that document's
exact encoded bytes. The corpus lives in `fsl-command`, the one crate that
sees every writer, `fsl-analyze`'s and its own included. Its test rebuilds
each document through the writer that builds it, compares the bytes with
the file, checks they are their own RFC 8785 re-encoding, and validates
them against the schema.

The corpus holds at least one document of every `Operation`, each built the
way that operation's outcome is built:

| Operation | Builder |
| --- | --- |
| `parse` | `OutcomeDocument::from_parse` (FR-663) |
| `format` | `OutcomeDocument::from_format` |
| `check` | `OutcomeDocument::from_check`, and `from_native_fences` for a Markdown check |
| `package` | `OutcomeDocument::from_package` |
| `execute` | `OutcomeDocument::from_run`, a call with `requirements` among them (FR-963) |
| `monitor` | `MonitorOutcome::document`, through `from_clause_runs` |
| `analyze` | `AnalyzeOutcome::document`, a simulation item among them |
| `prove` | `OutcomeDocument::settled(Operation::Prove, ..)` as `fsl_analyze::provider::run` builds it (FR-280) |
| `replay` | `OutcomeDocument::from_replay` |
| `inspect` | `fsl_command::command::inspect_handoff`, accepted and refused |
| `export` | the export outcome builders ("Export outcomes") |
| `select`, `check_fences`, `lower`, `generate`, `render` | `OutcomeDocument::from_stopped(operation, stopped)`, one cancelled (`Stopped::Cancelled(Requested)`), one at a stage limit (`Stopped::Limit`) and one faulted (`Stopped::Fault`) document each: every operation takes `&Cancel`, limits and can fault (FR-275, FR-276, FR-277), and these three stops are the documents these operations write today |

A corpus coverage test checks the other direction: every member path the
schema declares appears in at least one golden document, so no schema
member goes untested against a real writer's bytes.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-286-AC-1 | The `check` outcome over `tests/fixtures/spine-compile.native` serializes to a document whose `format` is `filament-outcome/1`, `operation` `check`, `last_stage` S4, `category` success, `items`, `diagnostics` and `artifacts` each an empty array, and `result` `null`. The `package` outcome over that `check` outcome serializes with `operation` `package`, `last_stage` S4, `category` success and `artifacts` an empty array `[]`. | Test (TC-770) |
| FR-286-AC-2 | The `check` outcome of FR-100-AC-5's `inv` source serializes with `category` refusal and one diagnostic whose cause, catalog code `ill_typed`, `Locus` and message equal the `StageFailure`'s. | Test (TC-770) |
| FR-286-AC-3 | An `analyze` outcome of three items serializes with three `items` entries in request order, each holding its terminal record and category. Serializing any outcome twice gives equal bytes. | Test (TC-770) |
| FR-286-AC-4 | FR-281-AC-7's `analyze` outcome serializes its item with category violation and cause `undefined-evaluation`, and the document holds no `undefined` label; FR-283-AC-5's `monitor` outcome serializes the same way; FR-100-AC-10's `execute` outcome serializes with category undefined and the label `undefined`. | Test (TC-770) |
| FR-286-AC-5 | The `execute` outcome of FR-100-AC-1's `seven` serializes with `category` success and `result` `{"kind": "completed", "value": {"integer": "7"}}`; FR-100-AC-10's empty `sum` with `category` undefined and `result` `{"kind": "undefined", "reason": "sum-out-of-domain"}`; FR-100-AC-6's `seven` with `call.accounting` `{"work_units": 0}` with `category` incomplete and `result` `{"kind": "incomplete", "limit": {"kind": "work_units", "bound": "0", "counter": "1", "field": "work_units", "setting": "accounting.work_units"}}`. A `check` called with a `Cancel` already cancelled serializes with `category` incomplete, `last_stage` `null`, `items` `[]` and `result` `null`. | Test (TC-770) |
| FR-286-AC-6 | `OutcomeDocument::from_run` over a completed run holds the call's document and `artifacts` an empty array; over `run` refusing `ill_typed` source, `category` refusal, `last_stage` S3 and one `ill_typed` diagnostic; over an unknown function, `category` refusal at S6a with `missing_declaration`; over `run` with a cancelled handle, `category` incomplete, `last_stage` null and one `cancelled` diagnostic with cause `requested`; over an internal fault, internal failure with its catalog code; and a driver's `OutcomeDocument::new(Execute, None, Unsupported)` for an unbuilt engine holds one diagnostic whose code serializes as `unimplemented_capability` with cause `backend-not-supplied` and `backend` naming the engine, with `category` unsupported, `last_stage` `null`, and exits 21. | Test (TC-770) |
| FR-286-AC-7 | TC-792's present lookup through the public spine run entry serializes through `OutcomeDocument::from_run` as one `filament-outcome/1` document with `operation` `execute`, category success, `artifacts` an empty array `[]` (as FR-286-AC-6 states for every completed run) and `result` `{"kind":"completed","value":{"reference":{"universe":U,"type":T,"object_identity":O}}}`. All three components equal the actual returned shared Reference. A second source-compiled Reference-valued function returning a different admitted reference from its input serializes the returned reference, not the input. The existing Boolean/Integer completions remain unchanged; no Reference completion is emitted as a `/2` clause truth or a Population payload. The one compiled package artifact of the source the run compiles belongs to the compile outcome, whose package artifact is the `filament.checked-package/v2` bytes (FR-027 Outputs, FR-027-AC-5), and never to this execute document. | Test (TC-792) |
| FR-286-AC-8 | `OutcomeDocument::from_clause_runs` over one clause run of `always p` over a finite supplied trace that ends pending serializes to a document with operation `monitor`, `category` pending, exactly one item with `result` `pending`, `category` pending and `clause` naming the clause, `cause` null, no terminal record and no `undefined` label, and FR-285 maps the document to exit 0. | Test (TC-770) |
| FR-286-AC-9 | `OutcomeDocument::from_clause_runs` over FR-501-AC-4's `Indeterminate` run serializes one item with `result` `indeterminate`, category indeterminate and FR-501's reason, and the document exits 11; over that run and a pending run it is category indeterminate; over those two and a `Completed(false)` run (item `refuted`, category violation) it is category violation, exit 10; over a `Completed(true)` run it holds the item `tested`, category success. Every item's category equals `ClauseRunReport::category()` and equals the label and exit FR-285-AC-10 reads. | Test (TC-770) |
| FR-286-AC-10 | Over FR-283-AC-1's clause run `always p` violated at position 0, `from_clause_run` gives an item with `result` `refuted`, category violation, `clause`, `position` 0, a `counterexample` with a `prefix` of three identities, an empty `loop` (`[]`, the finite spelling of FR-330 and FSpec FR-364), `trace_position` 0, a `decisive` beside the counterexample with `position` 2, no `quantifier` and no `index`, `element` `false` and the empty `value_path`, `cause` null and a `witness` member exactly when the run's `ClauseRunReport::witness` holds a record (FR-266 gives this temporal run the basis `closed-scope` and no witness, so the item carries none). | Test (TC-770) |
| FR-286-AC-11 | Over FR-283-AC-5's clause run, `from_clause_run` gives an item with `result` `refuted`, category violation, `position` 1, a `counterexample` with `trace_position` 1, cause `undefined-evaluation{where, cause}` with `cause` `division-by-zero`, no `witness` and no `undefined` label. | Test (TC-770) |
| FR-286-AC-12 | Over FR-283-AC-11's clause run, `from_clause_run` gives an item with `result` `undefined`, category undefined, cause `insufficient-data`, `position` 0, `count` 1 and `min_count` 2, and neither `counterexample` nor `witness`; its document exits 10. | Test (TC-770) |
| FR-286-AC-13 | Each `MonitorRecord` the FR-283 `monitor` operation returns for the clause runs of AC-8 to AC-12 (the pending run, FR-501-AC-4's indeterminate run, and the violated, undefined-evaluation and insufficient-data runs of FR-283-AC-1, AC-5 and AC-11) renders through `OutcomeItem::from_clause_run`, and the document built from the records equals `from_clause_runs`'s document for the same runs, byte for byte. | Test (TC-770) |
| FR-286-AC-14 | Over FR-281-AC-2's record and `ItemCounterexample`, `from_analyze` gives an item with `result` `refuted`, category violation, naming `en-1`, a `counterexample` of kind `formula` with a non-empty `prefix`, a `loop`, an `initial_state` and a `trace_position`, and no `position`, `decisive` or `witness`; the same record with no counterexample returns `Err(OutcomeItemError::MissingCounterexample)`, and FR-281-AC-1's `proved` record gives an item with no `counterexample`. For each other `Counterexample` variant, a `refuted` record with that variant gives an item whose `counterexample` has the variant's spelling in the table, and a `time-lock` `Temporal` one carries its `final_delay`. The item `provider::run` gives for the same two records serializes to equal bytes. | Test (TC-770) |
| FR-286-AC-15 | An external caller projects a pre-admission blank-label source diagnostic through `from_source_diagnostic` and serializes a document: its code is `invalid_source_identity`, cause `blank-label`, `label` the exact typed `authority` or `identity`, `locus` null and message unchanged. Empty-path, byte-budget and located invalid-UTF-8 source diagnostics retain their exact cause, code, optional region and message while omitting `label`; the located case retains its original source reference and endpoints. Projecting the same source diagnostic through a compile refusal, a parse refusal or a format refusal gives the same diagnostic members, without a consumer-written JSON field. | Test |
| FR-286-AC-16 | `OutcomeDocument::from_format` over a completed format of `tests/fixtures/spine-compile.native` has `result` `{"kind": "formatted", "text": T}` with `T` byte-identical to the string `fsl_cst::format` returns, category `success`, exit 0; over the format of `T` itself the `text` is again `T`; over a refused format `result` is `null` and the refusal is a diagnostic. | Test (TC-2178) |
| FR-286-AC-17 | Over FR-650's identity fixture, the `execute` outcome document of `id_rec` called with `fields` lacking `y` holds one diagnostic with `code` `invalid_runtime_input`, `cause` `missing-member` and `details` `{"path": "/record/fields", "position": 0}`, equal to the command-error envelope's `details` for the same request; of `id_set` with a repeated element, `details` `{"path": "/set/1", "position": 0}`. | Test (TC-2377) |
| FR-286-AC-18 | The `execute` outcome document of a call of `function` `nope` holds a diagnostic with `details` `{"function": "nope"}`; of `id_rec` called with an argument naming the parameter `zz` instead of `a`, `details` `{"parameter": "zz"}`; and of `id_big` called with `"7"`, stage `request`, `details` `{"path": "/call/arguments/0/value"}`. | Test (TC-2378) |
| FR-286-AC-19 | The `execute` outcome document of a run whose unit S3 refuses `ill_typed` (`function bad using v(): Int[0, 9] pure { true }`) and the `parse` outcome document of FR-763-AC-2's unit each hold diagnostics whose `details` is `null`; no diagnostic of any document holds a `details` member outside the closed vocabulary or an empty `details` object. | Test (TC-2379) |
| FR-286-AC-20 | A `monitor` document of settled clause runs (`MonitorOutcome::document`, here with no records) has `last_stage` `S6a`; the whole-request refusal document of `MonitorRefusal::UnknownClause` has `S4` and of `MonitorRefusal::Observation` `S6a`, each with its refusal category. | Test (TC-2383) |
| FR-286-AC-21 | The `replay` outcome of FR-862-AC-2's replay of `entry` serializes with `result` `{"kind": "replayed", "results": [R]}`, `R` equal to the one row of the `results` document `command::run` returns (`arm` `input`, `settlement` `reproduced-without-witness`); FR-862-AC-1's refused request serializes with `result` `null`. | Test (TC-2963) |
| FR-286-AC-22 | The `export` outcome of FR-756-AC-1's counter serializes with `operation` `export`, `category` success, `last_stage` S4, `items` `[]`, `artifacts` `[]` and `result` `{"kind": "exported", "export": E}` with `E` equal to the run outcome's `export` member; the below-floor export of FR-1015-AC-1 serializes with category unsupported and the same `result` kind, the export under `export.output_bytes` 1 with category incomplete, and the export naming an existing output path with category refusal, `last_stage` `null` and `result` `null`. | Test (TC-2994) |
| FR-286-AC-23 | The export of `SlowEnough` over `Braking` under `minimum_fidelity` `any` with `output` `{"file": "missing-directory/m.tla"}`, whose parent directory does not exist, exits `30` and serializes with `operation` `export`, `category` internal-failure, `last_stage` S4, `result` `null` and one diagnostic with `code` `output-failure`, `cause` `output-failure` and `details` `null`; no file is written. | Test (TC-3029) |
| FR-286-AC-24 | The `check` outcome of FR-056-AC-73's package with three `DUPLICATE_PARAM` diagnostics serializes with category `refusal` and exactly three `diagnostics` entries in the reader's order, each with code `invalid_model_binding`, cause `malformed-declaration`, the locus of the unit's `model` declaration, `details` `null` and a message naming `DUPLICATE_PARAM` and that entry's own package pointer; the refusal's human message holds the three messages on three lines in that order and contains no ", and N more" suffix. Serializing it twice gives the same bytes. | Test (TC-3044) |
| FR-286-AC-25 | Every file of `fsl-command/tests/fixtures/outcome-golden/` equals the encoded bytes of the document its writer builds and validates against `OUTCOME_SCHEMA_1` (`$id` `urn:filament:outcome:1`). The corpus holds a document of every `Operation`, each built by the builder the "Golden corpus" table names (`select`, `check_fences`, `lower`, `generate` and `render` each cancelled, at a limit and faulted through `from_stopped`), every `result` kind and every `Counterexample` variant. FR-286-AC-2's golden document fails validation with removing `/diagnostics/0/code` (an error at instance pointer `/diagnostics/0`, keyword `required`), with adding a member `fix` to `/diagnostics/0` (an error at `/diagnostics/0`, an unevaluated member `fix`), with `/category` set to `"violated"` (an error at `/category`) and with `/diagnostics/0/hint` set to `""` (an error at `/diagnostics/0/hint`); FR-520-AC-1's `analyze` document with `aggregate` removed fails at the root pointer, and FR-286-AC-1's `check` document with an `aggregate` member added fails at the root pointer. | Test (TC-3110) |
| FR-286-AC-26 | The bytes of every golden document, the `analyze` documents `fsl-analyze` assembles and the `monitor` and `execute` documents `fsl-command` writes included, equal their own RFC 8785 re-encoding through `quire-canonical`. FR-286-AC-2's diagnostic is written with its members in the order `cause`, `code`, `details`, `hint`, `locus`, `message`, and an `unsupported_construct`/`declaration-form` diagnostic carrying `construct`, `declaration`, `profile` and `text` in the order `cause`, `code`, `construct`, `declaration`, `details`, `hint`, `locus`, `message`, `profile`, `text`. The canonical check refuses the bytes `{"code":"ill_typed","cause":"type-mismatch"}`, whose members are out of order. `fsl_foundation::canonical_json::encode` of the object `{"\u{E000}": 1, "\u{1F600}": 2}` writes the `U+1F600` key first, by UTF-16 code units, where UTF-8 byte order would write it second. | Test (TC-3111) |
| FR-286-AC-27 | FR-286-AC-5's `seven` `execute` request and FR-286-AC-18's request naming the parameter `zz`, each written with the members of every object reversed, give documents byte-equal to the documents of the requests as written, and so does a `refinement_transfer` request (FR-1031) with the one unknown member `zz`. The same request with two unknown members written `yy` then `zz` refuses naming `/refinement_transfer/yy`, and written `zz` then `yy` names `/refinement_transfer/zz`: FR-1031 refuses at the first member in request order, so this pair differs by design. FR-056-AC-73's three-defect refusal keeps its three diagnostics in intake order. | Test (TC-3111) |
| FR-286-AC-28 | FR-286-AC-2's `ill_typed`/`type-mismatch` diagnostic carries `hint` equal to `hint_for(Code::IllTyped, Some("type-mismatch"), None)`, a nonempty sentence ending in `.`; the `io-error` diagnostic of FR-865-AC-4's `fsl run nosuch.json` carries `hint` equal to `hint_for(Code::IoError, None, None)`. FR-286-AC-23's `output-failure`/`output-failure` diagnostic, the cause-less `invalid-request` diagnostic of a malformed request document, and FR-286-AC-15's `invalid_source_identity`/`blank-label` diagnostic, pairs with no catalog row, have no `hint` member; the blank-label diagnostic has none through `from_source_diagnostic` and through a compile, a parse and a format refusal alike. | Test (TC-3112) |
| FR-286-AC-29 | `check_hint_catalog(HINTS, fsl_command::diagnostic_pairs())` returns `Ok`, and `diagnostic_pairs()` holds every pair of the "Fix hints" table; with a row `(Code::IllTyped, Some("type-mistmatch"), None)` added it returns `Err(HintCatalogError::UnknownPair)` naming `ill_typed` and `type-mistmatch`; with a second `(Code::IllTyped, Some("type-mismatch"), None)` row added, or a second row of one `selection` of the missing-selection pair, `Err(DuplicatePair)`, while the nine `selection` rows of that pair together are not duplicates; with a row whose text holds a line feed, one holding U+0085, one that does not end in `.`, and one of 101 `é` and a final `.` (203 bytes, 102 code points), `Err(MalformedText)` naming its pair. | Test (TC-3113) |
| FR-286-AC-30 | `hint_for(code, cause, None)` returns a text for every one of the 23 pairs of the "Fix hints" table, `io-error` with no cause included, and returns `None` for `(Code::OutputFailure, Some("output-failure"), None)` and for `(Code::InvalidRequest, None, None)`. | Test (TC-3113) |
| FR-286-AC-31 | The schema completeness test passes over every writer of the "Published schema" table: the golden `execute` document of FR-963-AC-1's call with `requirements` and its root `registry`, an FR-990-AC-1 `analyze` item with `obligations`, `closure` and `global`, an FR-990-AC-3 item with `recording`, an FR-290 item diagnostic `{"code":"invalid_result","cause":"invalid-value","reason":"node-id-unresolved"}`, FR-634-AC-1's simulation item, a diagnostic with `construct`, `declaration`, `profile` and `text`, and an FR-650 record value whose `fields` hold the names `a` and `zz` all validate. Run against a copy of the schema with `max_messages` removed from the item definition, it fails naming `/items/*/max_messages`. A new optional field added to `OutcomeItem` with the full-member fixture left unchanged fails to compile; set in the fixture but absent from `MEMBERS`, it fails the writer's `MEMBERS` test; added to `MEMBERS` and not to the schema, it fails the completeness test naming its path. A Value-built writer that emits a key outside its `MEMBERS` fails its `MEMBERS` test; the corpus coverage test run against a copy declaring a diagnostic member `unused` fails naming `/diagnostics/*/unused`. | Test (TC-3116) |
| FR-286-AC-33 | An `execute` outcome wrapping FR-100-AC-24's correlation incompleteness writes `result` `{"kind":"incomplete","reasons":[...]}` with the same typed reason codes and fields as `spine-run-result/1`, category incomplete and exit 22; a resource-limit outcome keeps FR-286's existing `limit` object form. | Test (`tests/it/native_run_relationships.rs::outcome_document_preserves_native_run_incomplete_reasons`) |
| FR-286-AC-32 | A `prove` item from a state-graph record with seed 18446744073709551615 encodes `"seed":"18446744073709551615"`, validates against the schema, and reads back to the same `u64`; with seed 7 it encodes `"seed":"7"`. `fsl_foundation::canonical_json::encode` refuses `{"n": 9007199254740992}` with `CanonicalJsonError::UnsafeInteger` at `/n` and accepts `{"n": 9007199254740991}`. A request whose `witness_samples` is 9007199254740992 refuses `invalid-request` naming `witness_samples`. The handoff tree FR-517-AC-5 writes keeps its `selection.json` pretty-printed with two-space indentation and FR-517's member order, which the encoder rule does not touch. | Test (TC-3117) |
| FR-286-AC-33 | A genuine `IndistinguishableHistories` refutation assembled through both Source and checked-Package intake gives an `analyze` item with its original request index, claim, admitted request, `refuted` record and category violation. Its `counterexample` has `type` `protocol-analysis-counterexample`, `kind` `indistinguishable-histories`, the exact analysis key and role, both histories and required actions, obligation and separating witness of the typed result. Each history retains its initial state and ordered successor/transition steps in FR-331's encodings; each send step carries its send record, and every non-send step omits `send_record`. Serializing the same settled item through the shared item producer gives the same canonical body bytes on either route. | Test |
| FR-286-AC-34 | A genuine `EnvironmentDefeat` refutation assembled through both intake routes gives the same body type with `kind` `environment-defeat`, preserving its key, every `defeats_per_branch` play in branch order, obligation and separating witness; each play retains the data AC-33 requires. An empty or shortened replacement for a nonempty branch list, reordered branches or a substituted play fails the typed-result-to-body comparison even when the enclosing outcome validates structurally. | Test |
| FR-286-AC-35 | Genuine projection refutations assembled through both intake routes give the same body type with `kind` `projection` and preserve the typed key, role, decision and separating witness. A choice decision retains both indexed cases, paths and nullable actions with its choice and owner; an acting-instance decision retains event, owner, both instances and maximum. The result remains `refuted`, category violation, with a separate canonical body and no fabricated realizability strategy. | Test |
| FR-286-AC-36 | Running the exact Source request `fsl-analyze/tests/fixtures/hidden-choice-source/requests/protocols/protocol-unrealizable.json` with its referenced `handover-hidden-choice.fsl`, `lock.semantic-ir.json` and `lock-free.json` gives one `Handover` item at request index 0, `record.result` `refuted`, category violation and a non-null `protocol-analysis-counterexample` body equal to the genuine realizability counterexample. The document is `filament-outcome/1`, operation `analyze`, category violation, with the non-null last stage FR-621 assigns to the completed analysis, and exits 10. Exit 30 with `runtime_invariant`, `items` `[]` and `last_stage` `null` does not satisfy this criterion. | Test |
| FR-286-AC-37 | For each of AC-33 to AC-35, the full emitted document validates against the published outcome schema and its body satisfies FR-331's positive/adverse protocol-counterexample contract vectors. Removing a required body member, adding an unknown member, using an unknown kind, mixing members of different kinds, omitting a send step's `send_record`, adding it to a non-send step or setting it to `null` fails that body contract. Removing the body makes the shared producer return `MissingCounterexample`; attaching a valid body from a different request item fails association checks. A record alone, a graph substitution or a rewritten `declined`/`unsupported` result never passes as the original refutation. Existing non-protocol counterexample spellings and non-refuted item behavior remain as specified above. | Test |
| FR-286-AC-38 | The outcome document of FR-1047-AC-1's refusal serializes with one diagnostic whose `details` is exactly `{"missing":["/request/models",["/request/program","/request/package"],["/request/call","/request/clause","/request/items"]]}`; the document of FR-1047-AC-5's `native-analyze/1` run refusal serializes with `details` exactly `{"accepted":["native-run/1","native-monitor/1","native-replay/1"],"path":"/format"}`; both validate against `OUTCOME_SCHEMA_1`. | Test (TC-3201) |
| FR-286-AC-39 | `compile_outcome_with_overrides` over FR-810-AC-1's request (`output` `{"package": "spine.package.json"}`, the request file in directory `reqs`) returns a document with `operation` `package`, `result` exactly `{"kind":"packaged","package":{"bytes":N,"output":"reqs/spine.package.json","handoff":null}}` with `N` the length of `package_bytes()`, `artifacts` `[]`, valid against `OUTCOME_SCHEMA_1`; the same request without `output` has `"output":null`; with `RequestOverrides::output` `cli.package.json` has `"output":"cli.package.json"`; with FR-665-AC-1's `handoff` and no `output` has `"handoff":"reqs/<directory>"`, `D` the `directory` member joined to `reqs`. A compile refusal and a `check` outcome have `result` `null`. | Test (TC-3203) |
| FR-286-AC-40 | For each `selection` of "Missing selections", the diagnostic its producer writes carries `selection` equal to that string, `code` `missing_import`, `cause` `missing-selection`, and `hint` equal to that row's text byte for byte; the document validates against `OUTCOME_SCHEMA_1`, which accepts exactly those nine `selection` values; the diagnostic of `UnresolvedDependency` carries no `selection` and the pair's catalog `hint`; `hint_for(Code::MissingImport, Some("missing-selection"), Some(<selection>))` returns the row's text for each of the nine, and `check_hint_catalog` returns `Ok`. | Test (TC-3203) |

## Status

The protocol-analysis amendment for QSL-2501 is a serialization contract.
Its production fix and AC-33 to AC-37 verification remain outstanding;
authoring and validating this requirement do not establish runtime repair.

Driver follow-up: driver FR-002-AC-9 still states that the CLI encodes
machine-mode outcomes through ix-cli-kit's `json` module, which orders keys
by UTF-8 bytes. Its change to writing FSL's canonical bytes unchanged is
tracked by QSL-2606; until it lands the two disagree only for keys mixing a
supplementary character with one from U+E000 to U+FFFF.

The Reference-completion amendment under QSL-665 is source-only; producer,
serializer and consumer qualification for AC-7 remain UNRUN. Existing
accounting-limit rendering and AC-5 remain their accounting owner's scope.

Partial (QSL-592). AC-1, AC-2 and AC-5 are tested over real outcomes. AC-3
and AC-4's `analyze` and `monitor` halves are tested over items built with
the public outcome builders, because layer A has no `AnalyzeOutcome` or
`MonitorOutcome` yet. Remaining: QSL-596 (FR-281, `analyze`) and QSL-597
(FR-283, `monitor`) each re-run TC-770 steps 3 and 4 over their real
outcomes.

## Dependencies

- [FR-109](FR-109-run-a-state-clause-through-the-spine.md), [FR-330](FR-330-run-a-temporal-clause-through-the-spine.md), [FR-501](FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md) and [FR-283](FR-283-monitor-a-supplied-trace-offline.md): the clause runs and verdicts the monitor document reports.
- **Delivery:** FR-286-AC-8 to FR-286-AC-12 (the clause-run item and document producer) are delivered by QSL-1622. FR-286-AC-13 (the monitor operation's records render through that producer) is delivered by the ticket that builds the `monitor` operation (FR-283, QSL-597).
- ADR-029 CB-4: the document members.
- ADR-013 O-16, T-5: category and `Locus`.
- [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md): `Locus`.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the operations.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): the exit code.
- FSpec FR-300-AC-3, FR-301-AC-1: library and CLI agree.
- FSpec FR-331: the terminal record and its wire spellings.
- FSpec FR-271: the diagnostic code catalog, `unimplemented_capability` among
  it.
- [FR-100](FR-100-run-a-named-function-through-the-spine.md): the value,
  undefined reason and limit spellings of `result`.
- [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md): the
  exhausted limit's members.
- RFC 8785 (JSON Canonicalization Scheme) and `quire-canonical`: the byte
  encoding and key order.
- JSON Schema draft 2020-12: the published schema's dialect.
- FSpec FR-272: the closed cause variants the hint catalog's pairs are drawn
  from; [FR-865](FR-865-render-every-outcome-document-as-text-the-cli-default.md)
  writes the hint in text; FSpec FR-520 and
  [FR-644](FR-644-stream-observations-through-the-monitor-command.md) name
  this schema for the analyze result and the monitor summary's items.
- [FR-963](FR-963-report-a-run-call-s-settled-requirement-items-in-the-run-outcome.md),
  [FR-990](FR-990-report-per-obligation-and-global-conformance-results.md),
  [FR-1032](FR-1032-bind-a-protocol-recording-to-the-analyze-request-by-document-identity.md),
  [FR-634](FR-634-request-a-simulation-item.md) and
  [FR-621](FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md):
  members the schema describes; [FR-1031](FR-1031-refuse-a-refinement-transfer-request-by-stage.md):
  a refusal that depends on request order.

## Overlap

The driver repository specifies and tests that `quire --format json` writes
this document to stdout with no member added or removed, by FSpec
FR-300-AC-3 and FR-301-AC-1. FSL's part is the document and its
serialization.

## References

- QSL-390 (ARCH-50): the team-leader decision ADR-029's References records,
  which keeps the `undefined` label off proof items (QSL-366 ruling).
- FSpec FR-300 (STD-141): the FSpec half.
