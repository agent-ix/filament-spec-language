---
id: FR-1044
title: "Request a component composition item through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-530
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-630
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: references
  - target: ix://agent-ix/filament-specification/FR-348
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-491
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-492
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
---
# FR-1044: Request a component composition item through analyze

## Description

A `native-analyze/1` item that names a checked `compose` declaration SHALL
request that declaration's FSpec FR-331 `composition` item, through the
`items` selection FR-620 reads for every other claim, and FSL SHALL settle
that item and each of its component-guarantee premise items in the same
request. FSL adds no verb and no request member. A composition item no
registered composition-capable provider can discharge settles `unsupported`
naming the `composition` capability, exit 21 (FSpec FR-348-AC-4); FSL never
settles it `proved` itself.

### Decisions

- **The selection is the existing item.** FSpec FR-491 "Derived item and
  slots" says a `compose` declaration under `interface`,
  `protocol-refinement` or `refinement-transfer` derives one routed
  `composition` item, and FR-530 compiles, emits and reads it
  (`fsl-route`'s `RequestWriter::declaration_items`). The only thing missing
  was a way for a source request to select it. It is selected as a named
  claim and a refinement are (FR-630): `claim` the declaration's name, `kind`
  `composition`. The architecture ruling on QSL-2739 chose this over a new
  verb or request family, because a second request family for one item kind
  would restate FR-620's name lookup, kind admission, negotiation and outcome
  rules.
- **The premises are derived from the declaration, never supplied.** The
  item's `premises` are the declaration's premise slots in FSpec FR-491's
  order. A caller who could supply them would let a request assert a
  guarantee that no engine established, which FSpec FR-489 forbids.
- **The component results come from the same request.** A
  `component-guarantee` slot is item-backed (FSpec FR-491): its item is the
  component's guarantee checked by that component's own engine. FSL settles
  those items in the request that selects the composition, so the composition
  reads results this request produced, and a result carried in from another
  run, or invented here, fills no slot.
- **Not assume-guarantee.** An `assume-guarantee` declaration derives a
  planner-local item settled from FSpec FR-492's settlement request
  (FSpec FR-491 "Derived item and slots"; FR-530), not a routed item. It has
  no `composition` item to select here, and no analyze request carries an
  FR-492 settlement request.
- **A composition provider is outside this requirement.** Discharging a
  composition from independently proved components (FSpec FR-348-AC-1) and
  refuting a weaker or undeclared interface naming the component and
  interface (FSpec FR-348-AC-2) belong to a composition-capable provider.
  This requirement does not deliver them: with none registered the item
  settles `unsupported`. Remaining work: QSL-2739, second PR.

## Inputs

- A `native-analyze/1` request over a program that declares at least one
  `compose S rule R { ... }` (FSpec FR-491) under rule `interface`,
  `protocol-refinement` or `refinement-transfer`, with the component packages
  the declaration imports.
- An `items` entry `{"claim": "S", "kind": "composition"}`, `S` the
  declaration's name. `kind` MAY be absent: a composition declaration admits
  one kind, so FR-620's `absent-kind` rule does not apply. The entry carries
  `required`, `backend` and `options` as any item does, and no member of a
  form that is not the composition's.

## Outputs

One FR-621 item for the composition, at the entry's `request_index`, followed
by one item per component-guarantee premise slot in FSpec FR-491's slot order,
each at the index FR-620 gives a derived item, after the caller's items.

- The composition item's `record` is `unsupported` with the diagnostic
  `unsupported-requested-capability` naming kind `composition`, settled at
  negotiation (FR-621 `S6b`), with no `backend` and no counterexample.
- Each premise item's record, extent, trust basis and certification are those
  the same clause settles to when requested alone (FR-620). The composition item inherits none of them: a `proved`
  premise does not make the composition `proved`, and a `refuted` one does
  not make it `refuted`.
- The document's exit code and category are FR-285's multi-item code over
  these items (FR-621 "Categories and exit").

## Behavior

- When an `items` entry names a declaration that FSpec FR-491 checked, the
  reader SHALL resolve it by FR-620's name lookup to that declaration's
  node, and SHALL build the item with `RequestWriter::declaration_items`'s
  `composition` item, its `premises` the slots of FSpec FR-491 in order.
- When no registered backend advertises `composition`, the item's candidate
  set is empty and FSL SHALL settle it `unsupported`, with the capability
  named, and SHALL NOT settle it `proved`, `refuted` or `inconclusive`.
- When the entry names a name no declaration resolves, FSL SHALL settle that
  item `invalid-request` with `invalid_capability`/`unknown-claim` naming the
  name, as FR-620 does for any claim, and the request's other items still
  run.
- When the entry has `kind` `composition` and names a declaration that is not
  a `compose` declaration, FSL SHALL settle that item `invalid-request` with
  FR-507's `inapplicable-kind`, as FR-620 does for a kind that does not
  apply to the claim's family.
- When the entry names a `compose` declaration under `assume-guarantee`,
  FSL SHALL settle that item `invalid-request` with `inapplicable-kind`
  whose detail names the rule `assume-guarantee`, and FSL SHALL NOT route it
  to a provider or settle it itself.
- When the entry carries a `premises` member, FSL SHALL settle that item
  `invalid-request` with `invalid_runtime_input`/`invalid-value` naming
  `premises`, before any item of it is negotiated.
- When a component-guarantee premise does not settle `proved`, FSL SHALL
  still return that premise's own record and SHALL NOT settle the
  composition `proved`.
- A request with no `items` member keeps FR-620's default items: this
  requirement adds none for a `compose` declaration, so a request that does
  not name the declaration settles as before.
- When a `package` request (FR-850) selects the same unit's checked
  package, FSL SHALL settle the same items, records and exit code as the
  source request.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1044-AC-1 | `fsl analyze` over a request whose program declares `compose Sys rule interface` over two imported components, each with one guarantee that its own engine proves, and the item `{"claim": "Sys", "kind": "composition"}`, writes the composition item at `request_index` 0 as `unsupported` with `unsupported-requested-capability` naming kind `composition`, no `backend`, and two derived items at indices 1 and 2, the two component guarantees, each `proved`; the document exits 21 in machine and text mode, and never reports the composition `proved`. | Test |
| FR-1044-AC-2 | The same request without `kind` settles the composition item as AC-1 does; with a registry that holds no backend advertising `composition` (the default `Engines::fsl()`), the item's `candidates` are empty. | Test |
| FR-1044-AC-3 | The items `{"claim": "Sys", "kind": "composition"}`, `{"claim": "Missing", "kind": "composition"}` and `{"claim": "Bounded"}` (a state invariant of the unit) settle the second `invalid-request` with `invalid_capability`/`unknown-claim` naming `Missing`, and the first and third as they settle alone; `{"claim": "Bounded", "kind": "composition"}` settles `invalid-request` with `inapplicable-kind`; the document exit is FR-285's code with a refusal item. | Test |
| FR-1044-AC-4 | `compose Sys rule assume-guarantee` named by `{"claim": "Sys", "kind": "composition"}` settles `invalid-request` with `inapplicable-kind` naming `assume-guarantee`, exit 20, and no premise item is derived. The `protocol-refinement` and `refinement-transfer` rules settle as AC-1's `interface` does. | Test |
| FR-1044-AC-5 | The item `{"claim": "Sys", "kind": "composition", "premises": []}` settles `invalid-request` with `invalid_runtime_input`/`invalid-value` naming `premises`, exit 20, and no component-guarantee item is settled for it. | Test |
| FR-1044-AC-6 | With AC-1's unit where one component's guarantee is violated by its component, the request writes that premise item `refuted` with its counterexample and the other `proved`, and the composition item `unsupported` and not `refuted`; each item's `extent` and certification are those the clause settles to when requested alone, and the exit is FR-285's multi-item code. | Test |
| FR-1044-AC-7 | AC-1's request over the handoff directory of the same unit through `package` (FR-850) writes the same items, records and exit code as the source request. | Test |
| FR-1044-AC-8 | A request carrying no `items` over AC-1's unit settles the items FR-620 gives and no composition item, with the exit code it gave before this requirement. | Test |

## Dependencies

- FR-530 (compile, emit and read the composition declaration and its derived
  item and slots); FR-620 and FR-621 (the analyze request, item resolution
  and outcome document); FR-630 (the precedent for selecting a declaration
  as a named claim); FR-850 (the package route).
- FSpec FR-348 (the routed composition obligation; AC-1 and AC-2 stay with
  its provider), FSpec FR-491 (the declaration, its derived item and slots),
  FSpec FR-492 (the planner settlement an `assume-guarantee` item uses),
  FSpec FR-331 (the `composition` item and its `premises`).
