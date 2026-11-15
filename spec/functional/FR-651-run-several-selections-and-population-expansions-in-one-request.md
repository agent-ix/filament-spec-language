---
id: FR-651
title: "Run several selections and population expansions in one request"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-014
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-652
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
  - target: ix://agent-ix/filament-specification/FR-550
    type: traces_to
---
# FR-651: Run several selections and population expansions in one request

## Description

A model usually has many rules and many objects. A native-run/1 request for
a `1-draft` program SHALL be able to run several selections, several
objects and every object of a population in one request, against one
compilation of the program, and report one result per item. A run of every
rule of a model over its data is one request, not one request file per rule
and object.

The request carries the member `items` in place of `call` or `clause`. Each
item is settled on its own: a refused, undefined, incomplete or violated
item is that item's result and changes no other item. The run's exit status
is the most severe item's, under FR-285's order.

## Inputs

A native-run/1 request for a `1-draft` program (FR-100's closed envelope,
source selections, `models`, `libraries` and intake limits) carrying exactly
one of `call` (FR-100), `clause` (FR-312) and `items`. `items` is a closed
object:

- `selections`: a non-empty array. Each entry is a closed object with
  exactly one member, one of:
  - `call`: FR-100's call object (`function`, `arguments`, and optionally
    `snapshot`, here a key of `observations`), run as a value call with
    FR-650's argument and result forms;
  - `clause`, `frame`, `function` or `temporal`: FR-312's selection
    variants, with FR-312's member sets;
  - `context` or `value_invariants`: FR-652's selection variants.
- `observations`: optional, FR-312's map from key to the file of a
  `filament.state.snapshot/v1` or `filament.state.invocation/v1` document,
  each read once under FR-100's `dependent_bytes` limit and shared by every
  selection that names its key.
- `accounting`: optional, FR-100's closed accounting object. It sets the
  budget of each item separately.
- `observation_limits`: optional, FR-106's `ObservationLimits` overrides,
  applied to each admission.

### Population selector

Where a selection names one object, it MAY instead carry the **population
selector** `{"each": {"population": P}}`, with `P` a population named by
FR-106's naming rule. The positions are: the `self` of a `clause`
selection, the `over` of a `temporal` selection, and the `value` of an
argument of a `call` or `function` selection whose parameter is a
`Reference<T>`. The selector's snapshot is the selection's own: the
observation a `clause` selection names, the first trace snapshot of a
`temporal` selection, and the `snapshot` of a `call` or `function`
selection.

The selector expands its selection into one item per object of `P` in that
snapshot, in the snapshot's document order of `P`'s objects, each item
being the selection with that object's `{"population": P, "key": K}`
reference in place of the selector. A selection carries at most one
selector, so a selection expands along one population and never into a
cross product; a caller who wants several populations lists several
selections.

## Outputs

One `native-run-items/1` result document and FR-285's exit code of its
category, or, for a refusal of the whole request, FR-267's command-error
envelope and no result document. `native-run-items/1` is one closed JSON
object with exactly these members:

- `format`: `"native-run-items/1"`.
- `source`: the program source as `spine-run-result/1` renders it.
- `selections`: one entry per request selection, in request order, each
  `{"index": i, "expanded": n}`, `n` the number of items the selection
  produced (1 for a selection with no selector or expansion).
- `items`: one entry per item, ordered by selection and then by expansion
  order. Each entry has `selection` (the selection's zero-based index),
  `object` (the `{"population": P, "key": K}` reference that replaced the
  selector, present exactly when the selection carried one), the members
  FR-652 adds for a `value_invariants` item, and `result`: the document the
  same item yields as the single selection of a request, which is a
  `spine-run-result/1` document for a completed, undefined or incomplete
  `call`, a `native-run-result/2` clause-run document for a clause, frame,
  function, temporal or context selection, and FR-267's
  `native-run-result/2` command-error envelope for an item refused before
  evaluation.
- `category`: the most severe item category under FR-285's order (30, 20,
  21, 22, 10, 11, 0 by exit code); `success` when `items` is empty.

## Behavior

- The run SHALL decode the whole request, including every selection and
  selector, before compiling. A request carrying more than one of `call`,
  `clause` and `items`, an empty `selections` array, a selection entry with
  no member, several members or an unknown member, a selector in a position
  not listed above, a second selector in one selection, or a selection
  naming a key `observations` does not hold, SHALL refuse `invalid-request`
  at stage `request`, exit 20, naming the member or key, with no result
  document.
- A selector or a `value_invariants` selection in a `call` or `clause`
  request SHALL refuse `invalid-request` at stage `request`, naming `each`
  or `value_invariants`: each produces several items, so it is carried by
  `items`, whose result document lists them; a single-selection request
  keeps its single-result document.
- The run SHALL compile the program once. A compile refusal is the whole
  request's: FR-267's command-error envelope with the refusing stage, and no
  result document.
- The run SHALL then select, admit and evaluate the items in item order,
  each against the same compiled package, each with a fresh evaluation
  budget from `accounting`, so an item's result does not depend on the
  items before it. Each item's select, admit and call-stage refusals,
  undefined outcomes, exhausted limits and violations SHALL be that item's
  `result`; the run SHALL continue with the next item.
- A selector naming a population its snapshot does not hold SHALL give that
  selection one item whose `result` is FR-106's refusal
  `invalid_runtime_input`/`wrong-role-mapping` naming `P`, at stage `admit`
  for a clause-run selection and stage `call` for a `call` selection, with
  no `object` member. A selector over a population with no object gives no
  item; its `selections` entry records `expanded` 0.
- An internal failure in any item (an FR-100 internal failure, exit 30) is
  not an item: it SHALL fail the whole request with FR-100's internal-failure
  envelope and no result document, as an evaluator fault fails a monitor
  request (FR-286). A cancellation observed during any item SHALL end the
  whole request as FR-100's `RunRefusal::Cancelled`, category incomplete,
  exit 22, with no result document.
- The number of items is bounded by the request's inputs: each selector
  expands over objects of a snapshot already admitted under
  `dependent_bytes` and `observation_limits`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-651-AC-1 | Through `run`, an `items` request over FR-108's ConfigVersion unit with two `clause` selections of `ParentOrder`, one over the healthy-parent current snapshot and one over the violating-parent snapshot, returns one `native-run-items/1` document with two items whose `result`s equal, byte for byte, the documents FR-312-AC-1's two single runs return (`success`, then `violation`), `selections` `[{"index": 0, "expanded": 1}, {"index": 1, "expanded": 1}]`, `category` `violation`, exit 10. | Test (TC-1708) |
| FR-651-AC-2 | An `items` request with one `clause` selection of `ParentOrder` over the healthy-parent snapshot whose `self` is `{"each": {"population": "ix://example/config-version/config_history"}}` returns two items in document order, `object` `{"population": ..., "key": "root"}` then `key` `child`, each `result` equal to the single run with that `self`, and `selections` `[{"index": 0, "expanded": 2}]`. A `call` selection of FR-109-AC-4's `sameIdentity` with `a` a selector and `b` the reference to `child` returns one item per object, the item for `child` completing `{"boolean": true}`. | Test (TC-1709) |
| FR-651-AC-3 | An `items` request over the FR-650 identity fixture with the `call` selections `id_big` with `4`, `id_rec` with a record lacking `y`, and `id_big` with `7` returns three items: completed `{"integer": "4"}`, the `native-run-result/2` command-error envelope `invalid_runtime_input`/`missing-member` at stage `call` with `details.position` 0, and completed `{"integer": "7"}`; the first and third `result`s equal the single runs; `category` `refusal`, exit 20. | Test (TC-1710) |
| FR-651-AC-4 | An `items` request holding FR-312-AC-1's violating-parent `clause` selection and a `call` of FR-100-AC-1's `seven` with `accounting` `{"work_units": 0}` returns a violation item and an incomplete item, `category` `incomplete`, exit 22; the same request with `accounting` set to the units one `seven` call consumes completes `seven` in the second item even when the same `seven` call is listed twice, because each item has its own budget. | Test (TC-1711) |
| FR-651-AC-5 | Each refuses `invalid-request` at stage `request`, exit 20, no result document, naming the member: a request carrying `items` and `call`; `selections` `[]`; a selection entry holding `call` and `clause`; a selection with two selectors; a selector as an argument `value` of a `call` request; a selector in a `clause` request; a selection naming an `observations` key that is absent. A request whose program has a syntax error refuses at stage `source` with one command-error envelope and no `native-run-items/1` document. | Test (TC-1712) |
| FR-651-AC-6 | A selector naming a population the snapshot does not hold gives its selection one item refused `invalid_runtime_input`/`wrong-role-mapping` naming the population, with no `object`, and the request's other items run; a selector over a complete population with no object gives no item and `expanded` 0, and a request whose only selection is that one returns `items` `[]`, `category` `success`, exit 0. | Test (TC-1713) |

## Dependencies

- FR-100 and FR-650: the envelope, the value call and its argument and
  result forms.
- FR-312 and FR-109: the clause-run selections and their reports.
- FR-652: the context and value-invariant selections.
- FR-106: observation documents, naming and admission.
- FR-267: the clause-run document and the command-error envelope.
- FR-285: the exit code of a category and the multi-item severity order.
- FSpec FR-550: one disposition per selected item in the standard envelope.
