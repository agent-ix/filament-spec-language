---
id: FR-621
title: "Write the analyze result as one filament-outcome/1 document per request"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-282
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-622
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-651
    type: references
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-352
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-520
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-181
    type: depends_on
---
# FR-621: Write the analyze result as one filament-outcome/1 document per request

## Description

The result of a `native-analyze/1` request SHALL be FR-286's
`filament-outcome/1` document with `operation` `analyze`: the document
`AnalyzeOutcome::document` already produces in process, extended with the
item members below. There is no separate analyze result format.

### Decision: reuse the outcome document and the standard's wire kinds

One outcome document per request keeps the library and the CLI byte-equal
(FR-286, FSpec FR-300-AC-3) and lets a consumer read `analyze`, `prove` and
`monitor` outcomes with one reader. The per-item terminal record, its
causes, its proof basis and its certification are FSpec FR-331's; every
category label and every temporal disposition kind an analyze item carries
is one FSpec FR-352 registers, so an analyze item never has a wire kind of
its own.

## Inputs

The `AnalyzeOutcome` of FR-281 for the items FR-620 resolved, with each
item's admitted request item and its declared claim name.

## Outputs

One `filament-outcome/1` document and its FR-285 exit code.

## Behavior

### Document members

The document carries FR-286's members (`format`, `operation`, `last_stage`,
`category`, `items`, `diagnostics`, `artifacts`, `result` `null`) and one
analyze member:

- `aggregate`: FR-281's aggregate over the required items,
  `{"status": "complete" | "incomplete", "gaps": [{"request_index", "reason", "result"?}]}`.
  `reason` is one of FR-281's `AggregateGap` spellings for an index the
  aggregate cannot account for (`duplicate-index`, `missing-index`,
  `unexpected-index`), or `not-proved` for a required item that settled
  other than `proved` or `measured`; a `not-proved` gap also carries
  `result`, the item's record `result` label (or `invalid-request` for a
  disposition record), so a reader sees why the aggregate is incomplete
  without opening the item. It is `complete` exactly when
  every required item settled `proved` or `measured`.
- `registry`: the backends the request negotiated against (FR-635).

A whole-request refusal, which settles no item, is still an `analyze`
document: it carries an empty `aggregate` (`status` `incomplete`, `gaps` `[]`: the
request settled nothing, so no item settled `proved` or `measured`) and an
empty `registry` (`[]`), so it validates against FR-286's published schema.
Its exit code 20 still comes from category `refusal`.

A `simulation` item carries FR-634's members in place of `record`,
`certificate`, `counterexample`, `replay` and `axes`.

### Last stage

An `analyze` document's `last_stage` is FR-286's greatest stage the request
entered, so a reader sees how far the request got without reading its items.
The program is compiled and linked through `S4` before any item is
admitted, and each item adds the stage that settled it:

| Outcome | `last_stage` |
| --- | --- |
| A request document refused before any source is read (malformed `native-analyze/1`, an unreadable file) | `null` |
| A source or compile refusal of the program | the refusing stage, `S1` to `S4` (`S3` for a checker refusal, FR-624-AC-2) |
| A request-level refusal after linking: a snapshot or binding that does not fit the linked package (FR-624-AC-3) | `S4` |
| The strict reader's refusal of the compiled package's read-back on the source route (FR-940 "A refused read of the compiled package") | `S4` |
| A document whose items are all settled by a disposition before any engine ran (an unknown claim, an unregistered backend) | `S4` |
| An item settled by a negotiated backend, or by its negotiation's `unsupported` disposition | `S6b` |
| An item settled by an FSL in-process engine (EN-1 to EN-7, the zone search): `proved`, `measured`, `incomplete` at a budget, `inconclusive` (an unknown verdict or a rejected certificate) | `S6c` |
| An item whose counterexample is built as a typed witness and not replayed | `S7` |
| An item whose counterexample is replayed (its `replay` member is present) | `S8` |
| A cancelled request, and an internal failure (an engine or evaluator fault), category internal-failure | `null` (FR-286) |
| A request whose package, or a dependency an item is analyzed over, has no admitted emission, so the document's node ids cannot be held | `null`; the run reports the emit limit or the strict reader's refusal as its cause, never internal-failure |

A document with several items takes the greatest of `S4` and its items'
stages. Every settled document therefore names a stage, `S4` or later, and
`null` appears only for request intake, cancellation and faults, as for
every other operation.

### Item members

Each item, in request order, carries:

| Member | Meaning |
| --- | --- |
| `request_index` | The item's zero-based position (FR-620). |
| `claim_name` | The declared name the document used, or for a default item the claim's declared name; a derived item's anchor name. |
| `request` | The admitted FSpec FR-331 request item that `ClaimItem::request_item_wire` writes, every default made explicit, so the item can be re-requested exactly. A `simulation` item carries its admitted `simulation` settings instead (FR-634). |
| `category` | The item's ADR-013 O-16 category (FR-285). |
| `record` | The FSpec FR-331 terminal record, in its wire spelling: the verdict in its `result` member (`proved`, `refuted`, `tested`, `measured`, `inconclusive`, `unsupported`, `declined`, `incomplete`, `failed`); an `invalid-request` disposition record carries no `result` (FSpec FR-331), its cause or proof basis, its `certification` for a proved record, `settled_by`, and `backend` exactly when `settled_by` is `result`. A disposition record carries its FR-272 `diagnostic` and no `cause`, as FR-286 states. |
| `diagnostics` | An optional non-empty array of this item's FSL diagnostics, distinct from the terminal record and the document-level `diagnostics`. For an unusable provider counterexample it contains exactly one `{code, cause, reason}` object as FR-290 defines; the member is absent when the item has no such diagnostic. |
| `certificate` | For a `proved` record whose engine returned a certificate: `{"identity", "checker", "accepted"}`. `identity` is FSpec FR-412's `CertificateIdentity` `{node_id, model, ordinal}` for every certificate kind, EN-1's closure and component certificates included: `node_id` the checked claim's node (a derived item's anchor node) as its structural id, never a document-held one ("Node ids in an item"), `model` the subject model's identity text and `ordinal` the obligation's FSpec FR-418-AC-9 ordinal, so one structural identity names the obligation whatever engine proved it. `checker` is the qualified-core checker that ran (FR-282) and `accepted` whether it accepted. A hybrid item's certificate (FSpec FR-652) is carried for every record whose provider entry carried one: a `proved` record, an `inconclusive` record of `insufficient-horizon` or of a trace, a `refuted` record and an `incomplete` record whose replay or witness check the work limit stopped. It also carries `form`, the FSpec FR-652 certificate exactly as the provider returned it, so a reader can re-check it. A certificate FR-751 checked carries `checker` and `accepted`; when it is rejected the member also carries `rule`, the spelling FSpec FR-652 "Rule names" gives the first failing rule (`box-shape`, `a` to `d`, `initial`, `picard`, `successor`, `jump`, `coverage`, `claim`, `witness-start`, `witness-picard`, `witness-unique`, `witness-guard`, `witness-violation` or `work-limit`), and `at`, the RFC 6901 pointer into the certificate, rooted at `/certificate`, at which it failed. A hybrid certificate the settlement does not rest on (FR-750 "Dispatch": a proof settled `insufficient-horizon`, or the witness of a trace exact replay decides, whose replay is refused or whose replay the work limit stops) is recorded unchecked: it carries `identity` and `form` and no `checker`, `accepted`, `rule` or `at`. Absent when the engine returns none. |
| `counterexample` | The FSpec FR-331 counterexample entry, present exactly for a `refuted` record. |
| `replay` | Present with `counterexample`: `{"target", "reproduced"}`. `target` is the FSpec FR-323 replay target member object exactly as a replay request carries it: `{"state_clause": Q}` for a state clause, a temporal clause or a state-graph claim (`possible`, `always possible`, `unique path`), `Q` its declared qualified name (EN-1 counterexamples included; a state-graph refutation replays through FSpec FR-393's model-graph arm, whose obligation identity binds the claim, so naming the claim is enough to select it), `{"function": ...}` or `{"frame_operation": ...}` for those targets, and FR-071's `{"protocol_refinement": {package_id, node}}` for a protocol refinement; `reproduced` is `true`. A counterexample that replay did not reproduce never reaches the document: its item is `inconclusive` with `replay-parity` or `replay-refused` (FR-281) and carries neither member. |
| `accounting` | The item's FSpec FR-331 accounting record: the limits applied, the method and seed where they apply, the limits the run reached (`reached`), and `settled_by`. With `exploration` it is the item's budget use. |
| `exploration` | For an item whose engine explores states: `{"states", "transitions", "automaton_states"}`, the states stored, transitions expanded and claim-automaton states, each an ASCII decimal count. Absent for a closed-form or provider item. |
| `axes` | The four outcome axes of FR-622, for an item that has an activation scope. Every `monitor` item has one, the activation of the clause it monitors, so every `monitor` item carries `axes` as FR-622 and FR-286 state; a `simulation` item carries FR-634's members instead. |
| `notes` | The item's non-fatal notes: facts about how the item was written that change neither its settlement nor its category and that a reader would otherwise lose. Present exactly when the item has at least one note, a non-empty array of closed objects `{"code", "details"}`, ascending by `code` (byte order) and then by the `details` members in the order the table below lists them, a count compared by its numeric value (`"9"` before `"10"`) and `at` in byte order. Each `code` is one of the closed list below; `details` is that code's closed object. |
| `package` | Present exactly on an item analyzed over a direct dependency of the request's package (a composition's component-guarantee premise, FR-1044): the dependency's package identity text, the identity its `dependencies.json` entry carries. Absent on an item over the request's own package. It names the graph the item's node ids index ("Node ids in an item"). |

### Node ids in an item

An item holds every node id, except a certificate identity's `node_id`, as an
FSpec FR-322 document-held id: each operand of the id's path is the index
segment `{"node": i}` naming `semantic_graph.nodes[i]` of one package graph,
and no such path holds a node id object. That covers the `certificate` and its
`document`, the `request` claim locus, the `counterexample` and the `record`
and `accounting` members, on every item including a simulation item. The
`node_id` of a certificate identity is not a document-held id: it is the
structural id FSpec FR-412 names (`CertificateNodeId`), each path segment a
string or a nested node id and never an index segment, in the item's
`certificate.identity` and in every identity inside it. The graph is the
request's own admitted package, or for an item with a `package` member, the
dependency that member names, whose graph is the one its `dependencies.json`
entry carries. A node id's own `package` member names the node's owner and
never selects a graph (FSpec FR-322 "Scope"). An identity's `model` is the
owning model's identity text and is never null, in the item's
`certificate.identity` and in every identity inside it. A reader resolves a
held id back to its structural key with the same graph. A document with a node
id outside a certificate identity that is not held, a certificate identity
whose `node_id` holds an index segment, an index its package does not have, a
`package` member that names no dependency of the request, or a `package` member
on an item over the request's own package is not a result of this standard.

A `composition` item writes the node ids of a component slot in the graph of that component's package (FSpec FR-322 "Scope"): every node id in the `key` of each `component-guarantee` entry of `request.premises`, its `claim` node, its subject and fairness ids and the peer clause ids of the subject's `assumptions` included, and the `interface` of an `interface-mismatch` counterexample. The component package is the `package` of the `components` entry of the composition whose `name` is the `component` of the entry's guarantee, which is the package the key's subject identity carries; a peer clause id of a key's `assumptions` is written in the graph of the package of the `components` entry whose `name` is the `component` of the peer guarantee that assumption names, never by the id's own `package` member (a node id's `package` names its owner, not a graph). The index segments of these ids name nodes of those graphs and never of the request package's graph. Every other node id of the item is held in the request package's graph. A premise item of a component writes the same ids with the same graph, so a premise key and the premise item's `request.claim` are equal as written. A node id that has no held form fails the run as an internal failure. A reader resolves a key by the graph of its slot's component package.

### Item notes

Some facts about an item are worth reporting and are not failures: an
engine found more sampled witnesses than the request keeps, or it holds a
witness it cannot write in the FSpec wire form. The FSpec FR-331 record and
counterexample carry only what their schemas name, and the document-level
FR-286 `diagnostic` describes a refusal and has closed `details` with no item
index, so neither can hold such a fact without dropping it or turning it
into a failure. `notes`
is the one carrier for such facts on an item:

| `code` | `details` | Written when |
| --- | --- | --- |
| `witnesses-beyond-limit` | `{"kept", "found"}`, each an ASCII decimal count | A statistical item's engine found `found` sampled witnesses (FR-193) and the item's FSpec FR-410 verdict keeps the first `kept` of them in trace-index order, `kept` being the request's `max_witnesses`, with `found > kept`. |
| `witness-omitted` | `{"test", "trace", "at"}`: the witness's FR-410 `test` index and trace index as ASCII decimal counts, and `at` the RFC 6901 pointer, rooted at the witness object, of the first member that has no FSpec FR-410 spelling | A sampled witness the engine kept is not written to the verdict's `witnesses`, because one of its members has no wire spelling. One note per such witness, so the verdict's `witnesses` and the item's `witness-omitted` notes together account for every kept witness. |

A note SHALL change nothing else: the item's record, category, settlement,
exit code contribution (FR-285), certificate and counterexample are what
they are without it, and no FSpec obligation, result or replay identity
reads `notes`. The writer SHALL write a note exactly when its condition
holds and SHALL NOT write a note for a fact a record member already carries.
An item with no notes has no `notes` member, and every note's bytes are a
function of the item, so equal requests give byte-identical documents
(FR-620).

The settlement of an item is its record's `settled_by` together with, for a
disposition record, the negotiation disposition and diagnostic the record
carries.

### A refuted schedulability item

A `schedulable` or `response` claim refuted by EN-7 (FR-247, FR-248) carries
its miss as the item's `counterexample`: the FSpec FR-331 counterexample
entry `{request_index, backend, counterexample}` whose `counterexample` is
FSpec FR-419's `schedulability-miss` object, written from the `Miss`
evidence `check_closed_form` agreed with. The document writer SHALL write it from the record's `MissEvidence` and
pass it to `OutcomeItem::from_analyze`: a `FixedPriority` miss as the
`response` miss (`task`, `job`, `iterates`, the response time of the last
iterate and the claim's bound, with `mode` `lo` for an AMC-rtb low-mode
miss), an `Edf` demand witness as the `demand` miss, and `Utilization(U)` as
the `utilization` miss, every value an exact rational in the task set's
checked unit. The item carries no `replay` member: the closed-form check
recomputed the miss from the recompiled task set, and FSpec FR-419 gives it
no replay target. Its `last_stage` contribution is `S7`, a typed witness not
replayed. A refuted EN-7 item SHALL NOT fail the document for a missing
counterexample.

### The record's `backend` member

FSpec FR-331 keeps two artifacts per item: the result entry, which carries
`backend`, the `BackendId` of the provider that produced it (FR-288), and the
accounting record, whose `settled_by` says whether the item was settled by
that result entry (`result`) or by a negotiation disposition with no result
entry (`disposition`). The item's `record` is the two merged in FSpec FR-331's
spellings, so it SHALL carry `backend`, copied from the result entry, exactly
when `settled_by` is `result`, and SHALL carry no `backend` when `settled_by`
is `disposition`: a disposition (`invalid-request`, or a negotiation
`unsupported` that no candidate met) has no producing backend, and
writing one would claim an engine decided what negotiation refused. No other
member names the backend, so a reader finds the deciding engine of every
result-settled item in one place, whatever its verdict.

### Items over several claim instances

A claim whose `over` parameter binds a population has one instance per
object of the subject's universe, and the item reports one record for all of
them. The writer SHALL order the instances by the bound object's universe
key in FSpec FR-181 key order and SHALL report: the first `refuted`
instance, when any instance is refuted, with its counterexample and replay;
otherwise the first instance whose record is not `proved`; otherwise a
`proved` record, whose certificate covers every instance. A claim is false
when any instance is false, so a single refutation decides the item, and a
fixed key order makes the reported instance the same on every run. The
counterexample names the instance it refutes (FSpec FR-364
`over_binding`; FSpec FR-393 `instance` for state-graph evidence).

### Temporal dispositions, values and engine roles

- An item that reports a temporal disposition (a `monitor` item's
  evaluation of a temporal clause over one supplied trace, or a
  `finite-replay` item's evaluation of a protocol clause over its replayed
  trace) SHALL carry it as
  FSpec FR-352's `disposition.temporal` with one of its seventeen kinds
  (`inactive`, `holds`, `violated`, `undefined`, `insufficient-data`, `pending`,
  `pending-window`, `pending-next-event`, `indeterminate`,
  `missing-fairness-premise`, `incomplete`, `activation-incomplete`,
  `window-limit-reached`, `progress-limit-reached`, `progress-contradicted`,
  `condition-refused`, `trace-refused`) and exactly that row's `category`,
  `truth`, `basis` and `outcome`: the three pending kinds have category
  `pending` and basis `unsettled`; `indeterminate` has basis `unavailable`;
  `activation-incomplete` and `progress-limit-reached` have category
  `incomplete` and no outcome; `progress-contradicted` has category
  `refusal` with outcome `invalid_runtime_input`/`invalid-value`; `inactive`
  has category `success`, no truth and basis `closed-scope`. Any other
  kind is refused by the writer as below. The `axes.settlement` member of
  such an item equals the disposition's `basis`. A temporal clause is never
  evaluated over a supplied trace by a `finite-replay` item: that kind
  applies to the `protocol` family only, and such an item settles
  `inapplicable-kind` (FR-507, FR-627); the FR-641 clause run and the monitor
  evaluate a temporal clause over a supplied trace and write the same
  `disposition.temporal` rows (FR-267).
- Every value in a counterexample state, an assignment, a witness or a
  simulation trace SHALL use FSpec FR-181's typed canonical forms, such as
  `{"type": "integer", "value": "3"}`, because the analyze document is the
  standard's envelope (FSpec FR-520). An analyze document has no value
  spelling of its own. FSL's run documents keep the run value form FR-650
  defines; the two map one to one, so a value read from either is the same
  value.
- An item settled by the bounded Kani engine settles `refuted` or
  `incomplete` when it reaches its bound, never `proved`, following the
  standard's engine assignment.
- The `items` array follows the multi-item rules FR-651 gives a run's
  `items`: entries are ordered, each gives exactly one outcome item in that
  order, and one entry's refusal settles only that entry. An analyze item
  names a claim, and the claim quantifies over every object of the subject's
  universes, so an analyze item has no `each` population selector.

### Categories and exit

- The document's `category` is the most severe item category, and its exit
  code FR-285's multi-item code; a document with no items is `success`, exit
  0.
- A writer SHALL refuse to emit a category label or temporal disposition kind
  that FSpec FR-352 does not register: the outcome becomes
  `internal-failure`, exit 30, with a diagnostic naming the kind.

### Text rendering

In `--format text`, the driver prints one line per item,
`<request_index> <claim_name> <record label>` followed by the cause or proof
basis when present, then one line with the document category, and exits with
the same code as JSON mode.

## Acceptance Criteria

The Counter fixture is FR-620's.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-621-AC-1 | The item `{"claim": "Bounded"}` gives an item with category `success`, record `proved` with proof basis `exhaustive` and certification `certified`, `certificate.accepted` `true`, `exploration.states` 4 and `exploration.transitions` 3; the document exits 0 and `aggregate.status` is `complete`. | Test (TC-1581) |
| FR-621-AC-2 | A clause `always holds(c.value <= 2)` gives record `refuted`, a `counterexample` entry ending in the state `value = 3`, `replay.reproduced` `true`, category `violation`, exit 10. | Test (TC-1581) |
| FR-621-AC-3 | An item with `temporal.limits.max_states` 2 gives record `incomplete` with cause `LimitReached{max_states}`, `exploration.states` 2, no `counterexample`, exit 22, and `aggregate` `incomplete` naming request index 0. | Test (TC-1581) |
| FR-621-AC-4 | Every `category` and record label in the analyze documents of AC-1 to AC-3 is a kind FSpec FR-352 registers; a writer handed an unregistered kind writes an `internal-failure` document, exit 30, naming it. | Test (TC-1581) |
| FR-621-AC-5 | The document the library writes for a request and the document the driver writes in machine mode for the same request are byte-equal. | Test (TC-1581) |
| FR-621-AC-6 | In text mode the AC-2 request prints `0 Bounded refuted` as its first line and exits 10. | Test (TC-1581) |
| FR-621-AC-7 | The temporal clause `Reaches` (`eventually holds(r.level = 2)` under infinite-trace over a record trace subject), still pending at the end of the `finite` samples 0, 1 it is run over through the FR-641 clause-run request, reports `disposition.temporal` kind `pending` with category `pending`, basis `unsettled` and `axes.settlement` `unsettled`, exits 0; a `finite-replay` analyze item naming a temporal clause settles `invalid-request` `inapplicable-kind` with `detail.family` `temporal-trace`, exit 20; the refuted counterexample of AC-2 spells `value` as `{"type": "integer", "value": "3"}`. | Test (TC-1581) |
| FR-621-AC-8 | Over the Counter subject, the `proved` EN-1 item of `Bounded` carries `certificate.identity` with exactly the members `node_id`, `model` and `ordinal`, `model` the Counter model's identity text; the `refuted` item of `BelowThree` carries `replay.target` `{"state_clause": Q}` with `Q` ending in `BelowThree`; and, both items required, `aggregate` is `incomplete` with one gap for `BelowThree`'s index, `reason` `not-proved`, `result` `refuted`. | Test (TC-2184) |
| FR-621-AC-9 | Over the Counter subject provisioned with counters `a` at 1, `b` at 0 and `d` at 0, `always holds(c.value >= 1)` settles `refuted` with a counterexample whose `over_binding` names `b` (the first refuted instance in key order, though `a` precedes it and is proved), and the same document run twice writes equal items; the refuted `always possible (value = 0)` item over the one-counter fixture carries `replay.target` `{"state_clause": Q}` with `Q` ending in the claim's declared name, and `replay.reproduced` `true`. | Test (TC-2292) |
| FR-621-AC-10 | Over the Counter subject, the `proved` item of `Bounded` and the `refuted` item of `BelowThree` each carry `record.settled_by` `result` and `record.backend` `en-1`; the `invalid-request` item of an unknown claim `Missing` carries `record.settled_by` `disposition` and no `backend` member. | Test (TC-2296) |
| FR-621-AC-11 | Over the Counter subject, the analyze document of `[{"claim": "Bounded"}]` has `last_stage` `S6c`; of `[{"claim": "Bounded"}, {"claim": "BelowThree"}]`, whose refuted item is replayed, `S8`; of `[{"claim": "Bounded", "temporal": {"limits": {"max_states": 2}}}]`, whose item is `incomplete`, `S6c`; and of `[{"claim": "Nothing", "temporal": {"item_kind": "deadlock-freedom"}}]`, whose only item is an `unknown-claim` disposition, `S4`. | Test (TC-2381) |
| FR-621-AC-12 | A malformed `native-analyze/1` document gives `last_stage` `null`; a program the checker refuses gives `S3`; a snapshot naming the undeclared population `ix://test/counter/nowhere` gives `S4`; and a request cancelled before its first item gives `null`. | Test (TC-2382) |
| FR-621-AC-13 | A request over FR-246-AC-1's `Ctl` with the third WCET 6 and the items `CtlFp` and `CtlEdf` gives two `refuted` items, category `violation`, exit 10, each with a `counterexample` entry whose `counterexample` is FSpec FR-419's `schedulability-miss`: for `CtlFp` `analysis` `fixed-priority` and `miss` `{"kind": "response", "task": "c", "job": "0", "iterates": [6, 10, 13], "response": 13, "bound": 12}` in unit `ms`, for `CtlEdf` `analysis` `edf` and `miss` `{"kind": "utilization", "utilization": 13/12}`; neither carries `replay`, the document's `last_stage` is `S7`, and no item is refused for a missing counterexample. Over the unchanged `Ctl` both items are `proved` with no `counterexample`. | Test (TC-2960) |
| FR-621-AC-14 | A hybrid item whose `bounded-solver` proof FSpec FR-193 settles `insufficient-horizon` and whose entry carries a `flowpipe` writes `certificate` with exactly `identity` and `form`; a refuted hybrid item that exact replay reproduces and whose provider entry carried a `validated-witness` writes `certificate` with exactly `identity` and `form`; FSpec FR-652-AC-5's accepted certificate writes `checker`, `accepted` `true` and `form`; its `[1, 10]` variant writes `accepted` `false`, `rule` `jump` and `at` `/certificate/steps/160`; a witness check the work limit stops writes the `incomplete` item with `accepted` `false`, `rule` `work-limit` and an `at` rooted at `/certificate`. | Test (TC-3035) |
| FR-621-AC-15 | A statistical `probability` item over FR-193's violating model with `max_witnesses` 2, whose engine finds five sampled witnesses, writes a `measured` record with two `witnesses`, the first two in trace-index order, and `notes` `[{"code": "witnesses-beyond-limit", "details": {"kept": "2", "found": "5"}}]`; with `max_witnesses` 8 the item has no `notes` member. A kept witness with a member that has no FSpec FR-410 spelling is left out of `witnesses` and named by one `witness-omitted` note holding its `test`, its `trace` and the pointer `at` of that member, and the notes sort by `code`, then `test` and `trace` by numeric value, so a witness with `test` `"9"` precedes one with `"10"`. In each case the item's record, category, exit code and every identity are those of the same item with `notes` removed. | Test (TC-3136) |
| FR-621-AC-16 | Over the probability fixture `Eventually`, the `proved` exact item writes `certificate.identity` and `certificate.document` that validate against FSpec's backend-provider `CertificateIdentity` and `ProbabilityCertificate`, with `identity.model` the owning model's identity text and an `identity.node_id` that is the structural id, each path segment a string or a nested node id; the item's `request.claim.node` holds index segments and, resolved through the package graph, equals `certificate.identity.node_id`; the `refuted` exact item `TooLikely` writes a `request.claim` that validates as FSpec's `Locus`, a `counterexample` in which no node id's path holds a node id object; this criterion does not assert that the `counterexample` validates against `CounterexampleEntry`; and the independent EN-5 checker accepts the proved item's `certificate.document` as written, with the item naming the same model, and refuses it when `identity.node_id` is replaced by the held `request.claim.node`; the same checker accepts as written the `ratio` `ProbabilityCertificate` of a workload quantile claim and of an `on each` claim, and refuses it when a part's `identity.model` is not the item's model; and the checker's source route (`check_probability_certificate_source`) reads the identity of an exact `ProbabilityCertificate` in this held form as written when it nests past 128 JSON levels, resolving the document's held ids, and refuses one naming another claim as an identity mismatch without overflowing the stack; this criterion states nothing about the depth a value-bounds `lower` or `upper` document's reader admits. | Test (TC-3154) |
| FR-621-AC-17 | A composition's component-guarantee premise item writes a `package` member naming its dependency and node ids that index that dependency's graph, so that item holds no node id object whose path holds a node id outside a certificate identity; the composition item writes no `package` member, holds the node ids of each component-guarantee premise key in the graph of that slot's component package (so each key's `claim` equals its premise item's `request.claim` as written, and the item holds no node id object whose path holds a node id outside a certificate identity) and every other node id outside a certificate identity in the request package's graph; and a run whose package has no admitted emission (the emit limit is reached) reports the typed cause, the strict reader's refusal or an unavailable graph, and writes no result with an unheld node id. | Test (TC-3154) |
| FR-621-AC-18 | A provider verdict carrying each FR-290 unusable-counterexample reason settles its item `inconclusive` with record cause `replay-parity`, no `counterexample` or `replay`, and exactly one item-level `diagnostics` entry `{"code":"invalid_result","cause":"invalid-value","reason":R}`; an item with mapped assignments has no item-level `diagnostics` entry. The document-level `diagnostics` remains unchanged. | Test (TC-774) |
| FR-621-AC-19 | Starting from `examples/lock/requests/fairness/one-served.json`, set the request format to `native-analyze/1`, set `request.documents` to `[ {"file":"examples/lock/data/lock-free.json"} ]`, and set `request.items` in order to `[{"claim":"ServedWithoutFairness"},{"claim":"ServedIfAcquireIsStronglyFair"},{"claim":"ServedIfEachAcquireIsStronglyFair"}]`. In `command::analyze`'s document, item 0 has `request_index` `0`, `claim_name` `ServedWithoutFairness`, `record.result` `refuted`, and a nonempty `counterexample.loop`. | Test (TC-1581) |
| FR-621-AC-20 | For the `command::analyze` request defined in AC-19, item 1 has `request_index` `1`, `claim_name` `ServedIfAcquireIsStronglyFair`, `record.result` `refuted`, and a nonempty `counterexample.loop`. | Test (TC-1581) |
| FR-621-AC-21 | For the `command::analyze` request defined in AC-19, item 2 has `request_index` `2`, `claim_name` `ServedIfEachAcquireIsStronglyFair`, `record.result` `proved`, and `certificate.accepted` `true`. | Test (TC-1581) |
| FR-621-AC-22 | For the `command::analyze` request defined in AC-19, the three outcome items have `request_index` `[0, 1, 2]` and `claim_name` `["ServedWithoutFairness", "ServedIfAcquireIsStronglyFair", "ServedIfEachAcquireIsStronglyFair"]` in request order, and the document exits 10. | Test (TC-1581) |
| FR-621-AC-23 | On a 512 KiB stack, `check_probability_certificate_package` given a package with the deepest operand chain admitted by default checked-package limits (10,000 graph nodes, 9,999 operand links, and at most 1 MiB) and a held certificate whose first value holds an id into node 9,999 and whose next value holds the out-of-range index 10,000 returns typed `Malformed` without overflowing the stack. | Test (TC-3154) |
| FR-621-AC-24 | On a 512 KiB stack, `check_probability_certificate_source` given source that compiles under default limits to a package with 10,000 graph nodes and 9,999 operand links (within the default 1 MiB package-byte limit), and a held certificate whose first value holds an id into node 9,999 and whose next value holds the out-of-range index 10,000, returns typed `Malformed` without overflowing the stack. | Test (TC-3154) |

## Dependencies

- FR-286 owns the outcome document; FR-281 owns the records and aggregate.
- FSpec FR-331 owns the record, counterexample and accounting spellings;
  FSpec FR-352 owns the registry of category and disposition kinds.
