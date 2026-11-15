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

A `simulation` item carries FR-634's members in place of `record`,
`certificate`, `counterexample`, `replay` and `axes`.

### Item members

Each item, in request order, carries:

| Member | Meaning |
| --- | --- |
| `request_index` | The item's zero-based position (FR-620). |
| `claim_name` | The declared name the document used, or for a default item the claim's declared name; a derived item's anchor name. |
| `request` | The admitted FSpec FR-331 request item that `ClaimItem::request_item_wire` writes, every default made explicit, so the item can be re-requested exactly. A `simulation` item carries its admitted `simulation` settings instead (FR-634). |
| `category` | The item's ADR-013 O-16 category (FR-285). |
| `record` | The FSpec FR-331 terminal record, in its wire spelling: the verdict in its `result` member (`proved`, `refuted`, `tested`, `measured`, `inconclusive`, `unsupported`, `declined`, `incomplete`, `failed`); an `invalid-request` disposition record carries no `result` (FSpec FR-331), its cause or proof basis, its `certification` for a proved record, and `settled_by`. A disposition record carries its FR-272 `diagnostic` and no `cause`, as FR-286 states. |
| `certificate` | For a `proved` record whose engine returned a certificate: `{"identity", "checker", "accepted"}`. `identity` is FSpec FR-412's `CertificateIdentity` `{node_id, model, ordinal}` for every certificate kind, EN-1's closure and component certificates included: `node_id` the checked claim's node (a derived item's anchor node), `model` the subject model's identity text and `ordinal` the obligation's FSpec FR-418-AC-9 ordinal, so one structural identity names the obligation whatever engine proved it. `checker` is the qualified-core checker that ran (FR-282) and `accepted` whether it accepted. Absent when the engine returns none. |
| `counterexample` | The FSpec FR-331 counterexample entry, present exactly for a `refuted` record. |
| `replay` | Present with `counterexample`: `{"target", "reproduced"}`. `target` is the FSpec FR-323 replay target member object exactly as a replay request carries it: `{"state_clause": Q}` for a state or temporal clause, `Q` its declared qualified name (EN-1 counterexamples included), `{"function": ...}` or `{"frame_operation": ...}` for those targets, and FR-071's `{"protocol_refinement": {package_id, node}}` for a protocol refinement; `reproduced` is `true`. A counterexample that replay did not reproduce never reaches the document: its item is `inconclusive` with `replay-parity` or `replay-refused` (FR-281) and carries neither member. |
| `accounting` | The item's FSpec FR-331 accounting record: the limits applied, the method and seed where they apply, the limits the run reached (`reached`), and `settled_by`. With `exploration` it is the item's budget use. |
| `exploration` | For an item whose engine explores states: `{"states", "transitions", "automaton_states"}`, the states stored, transitions expanded and claim-automaton states, each an ASCII decimal count. Absent for a closed-form or provider item. |
| `axes` | The four outcome axes of FR-622, for an item that has an activation scope. |

The settlement of an item is its record's `settled_by` together with, for a
disposition record, the negotiation disposition and diagnostic the record
carries.

### Temporal dispositions, values and engine roles

- An item that reports a temporal disposition (a `finite-replay` or
  `monitor`-style evaluation of a clause over one trace) SHALL carry it as
  FSpec FR-352's `disposition.temporal` with one of its sixteen kinds
  (`holds`, `violated`, `undefined`, `insufficient-data`, `pending`,
  `pending-window`, `pending-next-event`, `indeterminate`,
  `missing-fairness-premise`, `incomplete`, `activation-incomplete`,
  `window-limit-reached`, `progress-limit-reached`, `progress-contradicted`,
  `condition-refused`, `trace-refused`) and exactly that row's `category`,
  `truth`, `basis` and `outcome`: the three pending kinds have category
  `pending` and basis `unsettled`; `indeterminate` has basis `unavailable`;
  `activation-incomplete` and `progress-limit-reached` have category
  `incomplete` and no outcome; `progress-contradicted` has category
  `refusal` with outcome `invalid_runtime_input`/`invalid-value`. Any other
  kind is refused by the writer as below. The `axes.settlement` member of
  such an item equals the disposition's `basis`.
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
| FR-621-AC-7 | A `finite-replay` item whose clause is still pending at the end of the supplied trace carries `disposition.temporal` kind `pending` with category `pending`, basis `unsettled` and `axes.settlement` `unsettled`, exits 0; the refuted counterexample of AC-2 spells `value` as `{"type": "integer", "value": "3"}`. | Test (TC-1581) |
| FR-621-AC-8 | Over the Counter subject, the `proved` EN-1 item of `Bounded` carries `certificate.identity` with exactly the members `node_id`, `model` and `ordinal`, `model` the Counter model's identity text; the `refuted` item of `BelowThree` carries `replay.target` `{"state_clause": Q}` with `Q` ending in `BelowThree`; and, both items required, `aggregate` is `incomplete` with one gap for `BelowThree`'s index, `reason` `not-proved`, `result` `refuted`. | Test (TC-2184) |

## Dependencies

- FR-286 owns the outcome document; FR-281 owns the records and aggregate.
- FSpec FR-331 owns the record, counterexample and accounting spellings;
  FSpec FR-352 owns the registry of category and disposition kinds.
