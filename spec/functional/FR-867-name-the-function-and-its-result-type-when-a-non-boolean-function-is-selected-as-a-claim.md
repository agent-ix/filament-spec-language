---
id: FR-867
title: "Name the function and its result type when a non-Boolean function is selected as a claim"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-272
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
---
# FR-867: Name the function and its result type when a non-Boolean function is selected as a claim

## Description

A clause-run request may select a function as a claim over a snapshot
(FR-312's `{"function": {"name", "arguments", "snapshot"}}`, FR-109's
`Function` selection). Only a function with a `Boolean` result is a claim;
FR-109 refuses any other at stage `select` with `ill_typed`/`type-mismatch`,
before any call. That check runs at `run`, because only the request knows
the selection: `fsl check` of the unit cannot know which function a later
request selects, so it never reports this refusal.

The refusal SHALL tell the reader which function was selected and what it
returns. Its FSpec FR-272 `type-mismatch` payload SHALL carry `function`,
the selected function's qualified name, `expected` `Boolean`, and `actual`,
the function's declared result type as the checker spells it; its locus is
the function's declared result type in the source; and its message names
the function, `Boolean` and the actual type, for example
"function `seven` returns `Integer`; a claim selection needs a function
whose result is `Boolean`".

A selection naming a function the package does not declare keeps FR-109's
`missing_declaration`/`missing-name` refusal at stage `select`, with the
name in its payload and message.

### The refusal in the outcome document

The driver writes a clause-run request's result as FR-286's
`filament-outcome/1` `monitor` document, one item per selection, and that
document is what a consumer that reads only outcome documents sees, such as
the examples runner, which compares each case's `category`, `last_stage`,
`result`, first diagnostic and `items` with its recorded structure. An item
cause holds only the FR-331 spelling and catalog code, so without more the
function and its types would reach the `native-run-result/2` document only.
The `declined` item of a `select` refusal therefore SHALL carry, beside its
`cause` (`{"kind", "code"}`), FR-286's `diagnostic` member with `code`,
`cause` and `detail` keyed by the FR-272 payload names above:
`{"function", "expected", "actual"}` for `type-mismatch` and `{"name"}` for
`missing-name`. Its `clause` member is the selected function's name. The
locus stays on the `native-run-result/2` refusal record: an outcome-document
consumer compares structure, and a byte position would change its record on
every unrelated edit of the source.

### How the examples suite records a function claim

A function claim case runs through the `run` verb with a `native-run/1`
request whose `clause.selection` is FR-312's `function` member, because a
claim over one snapshot is a clause run (FR-109, FR-312) and `analyze`
decides claims over a model's behaviors, not over a supplied snapshot. Its
recorded expectation is the exit code (FR-285) and the outcome-document
structure above. A Boolean function that returns `true` records exit 0, item
`tested`, `cause` null; one that returns `false` exit 10, item `refuted`,
`cause` null; one whose evaluation is undefined exit 10, item `refuted`,
category `violation`, `cause` `{"kind": "undefined-evaluation", "where": W,
"cause": R}` (FR-286, FSpec FR-331), with `W` the undefined evaluation's
location and `R` its FR-100 undefined reason (FR-109: an undefined claim does
not hold); a refused selection exit 20, document and item category `refusal`,
item `declined` with the `cause` and `diagnostic` above. Message text is
recorded beside the structure and never compared.

## Inputs

A clause-run request with a `function` selection.

## Outputs

The run's result document and FR-285's exit code; the refusal record named
above for a non-Boolean or unknown function.

## Behavior

- If a `Function` selection's function does not declare a `Boolean`
  result, then the clause-run entry SHALL refuse at stage `select`
  `ill_typed`/`type-mismatch` with payload `function`, `expected` `Boolean`
  and `actual` the declared result type, at the result type's locus, before
  any call.
- If a `Function` selection names no function of the package, then the
  entry SHALL refuse at stage `select` `missing_declaration`/`missing-name`
  naming the selected name.
- When a `Function` selection's function declares a `Boolean` result, the
  entry SHALL run it as a claim as FR-109 states.
- When the outcome document is built from a run refused at stage `select`,
  its `declined` item SHALL carry `diagnostic` `{code, cause, detail}` with
  the refusal's FR-272 payload as `detail`, beside its `cause`, and
  `clause` naming the selected function.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-867-AC-1 | Through `command::run`, a clause request over FR-108's ConfigVersion unit extended with `function seven using v(): Integer pure { 7 }`, selecting `{"function": {"name": "seven", "arguments": [], "snapshot": "current"}}`, exits 20 with stage `select`, `ill_typed`/`type-mismatch`, payload `function` `seven`, `expected` `Boolean` and `actual` `Integer`, and a message naming `seven`, `Boolean` and `Integer`. | Test (TC-2459) |
| FR-867-AC-2 | The same request selecting `function yes using v(): Boolean pure { true }` exits 0 with `truth` true; selecting the name `nope` exits 20 with stage `select`, `missing_declaration`/`missing-name`, naming `nope`. | Test (TC-2459) |
| FR-867-AC-3 | The filament-examples `shop` system holds running cases over one program declaring `yes` (`Boolean`, `true`), `no` (`Boolean`, `false`), `seven` (`Integer`), `boundedValue` (`Int[0, 3]`) and one `Boolean` function whose evaluation over the selected snapshot is undefined, each run as `run` over a `native-run/1` request selecting `{"function": {...}}` over one shop snapshot: `claim-function-holds` (`yes`) exit 0, item `tested`; `claim-function-false` (`no`) exit 10, item `refuted`; `claim-function-undefined` exit 10, item `refuted`, `cause` `{"kind": "undefined-evaluation", "where": W, "cause": R}` with `R` its FR-100 undefined reason; `claim-function-integer` (`seven`) exit 20, item `declined`, diagnostic `ill_typed`/`type-mismatch` with detail `function` `seven`, `expected` `Boolean`, `actual` `Integer`; `claim-function-bounded` (`boundedValue`) the same with `actual` `Int[0, 3]`; `claim-function-unknown` (`nope`) exit 20, item `declined`, diagnostic `missing_declaration`/`missing-name` with detail `name` `nope`. Each has its recorded `expected/<case>.json`, and the examples gate runs all six. | Inspection (TC-2459): the cases live in the filament-examples repository and run under the driver's examples gate (filament-driver FR-021). |
| FR-867-AC-4 | `OutcomeDocument::from_clause_runs` over the FR-867-AC-1 run of `seven` gives a `monitor` document with category `refusal` and one item with `clause` `seven`, `result` `declined`, category `refusal`, `cause` `{"kind": "type-mismatch", "code": "ill_typed"}` and `diagnostic` `{"code": "ill_typed", "cause": "type-mismatch", "detail": {"function": "seven", "expected": "Boolean", "actual": "Integer"}}`; over the `boundedValue` run, `detail.actual` `Int[0, 3]`; over the `nope` run, `diagnostic` `{"code": "missing_declaration", "cause": "missing-name", "detail": {"name": "nope"}}`; over the `yes` run, an item `tested` with `cause` null and no `diagnostic`. | Test (TC-2951) |

## Dependencies

- [FR-109](FR-109-run-a-state-clause-through-the-spine.md): the `Function`
  selection and its `select` refusals.
- [FR-312](FR-312-read-a-clause-run-request-and-run-it.md): the request's
  `function` selection.
- FSpec FR-272: the `type-mismatch` payload (expected and actual named type).
- [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md):
  the `monitor` item and its `diagnostic` member.
