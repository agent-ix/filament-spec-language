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
  - target: ix://agent-ix/filament-spec-language/FR-507
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
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
  - target: ix://agent-ix/filament-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-489
    type: depends_on
---
# FR-1044: Request a component composition item through analyze

## Description

A `native-analyze/1` item that names a checked `compose` declaration SHALL
select that declaration's composition, through the `items` shape FR-620 reads
for every other claim, and FSL SHALL settle the composition and each of its
item-backed premise slots in the same request. FSL adds no verb and no request
member. A composition that no registered composition-capable provider can
discharge settles `unsupported`, naming the `composition` capability, exit 21
(FSpec FR-348-AC-4); FSL never settles it `proved` itself.

### Decisions

- **The selection is the existing item.** FSpec FR-491 "Derived item and
  slots" derives, for a `compose` declaration under `interface`,
  `protocol-refinement` or `refinement-transfer`, one routed FSpec FR-331
  `composition` item, and FR-530 compiles, emits and reads it
  (`RequestWriter::declaration_items`). What was missing is a way for a source
  request to select it. It is selected as a refinement is (FR-630): `claim`
  the declaration's name, `kind` `composition`. The architecture ruling on
  QSL-2739 chose this over a new verb or request family, which would restate
  FR-620's name lookup, kind admission, negotiation and outcome rules.
- **The family is `protocol`.** FSpec FR-290's claim-form row gives a
  composition under the three routed rules the kind `composition` and the
  family `protocol`, and FR-507 states the same row and reads the family from
  the claim's `correspondence`/`component_composition` node (this change). An
  item of any other kind on such a claim is inapplicable by FR-507's rule.
- **An `assume-guarantee` selection is admitted and planner-local.** FSpec
  FR-491 and FSpec FR-290 derive no request item for it and give it no family:
  the planner settles it from FSpec FR-492's settlement request. It is still
  selectable by name, so the second PR can settle it by FR-492's decision
  order, a non-proved premise giving `inconclusive`, `premise-unproved`
  (written only by that planner path, never for a routed item). Until then it
  settles as the routed rules do, `unsupported` naming `composition`. FR-507
  reads a family for request items only, so this selection is not refused by
  FR-507's no-family rule.
- **The premises are derived, never supplied.** The slots are FSpec FR-491's,
  in its order. A caller who could supply them would assert a guarantee no
  engine established, which FSpec FR-489 forbids, and FR-620 already refuses
  every member FSL computes.
- **A premise item is its slot's item, not the clause requested alone.** A
  `component-guarantee` slot's item is `G` over `C`'s subject checked under
  exactly the assumptions FSpec FR-491 gives it from `G`'s `rely` and
  `depends` members, and the assumption set is part of its slot key. Only a
  `G` with no such member has the item the clause settles to alone. Each
  premise item settles by the negotiation any item gets: FSL invents no
  engine for an assumption set, so an item no registered engine takes
  settles `unsupported` and is reported as such.
- **The composition names its premises by slot key.** FSpec FR-331 `premises`
  holds `{role, key}` per slot and no result. FSL settles every premise item
  before it writes the composition item's outcome, and it never copies a
  premise result into the composition. How a provider reads those results is
  the second PR's decision.
- **A composition provider is outside this requirement.** Discharging a
  composition from independently proved components (FSpec FR-348-AC-1) and
  refuting a declared interface mismatch, naming the component and interface
  (FSpec FR-348-AC-2), are not delivered here, and nothing here reports an
  `assume-guarantee` verdict as FSpec FR-348-AC-2. FSpec FR-492's composition
  result is never `refuted` while FSpec FR-348-AC-2 refutes, so the provider
  that meets AC-2 for the routed rules is open. Remaining work: QSL-2739,
  second PR.

## Inputs

- A `native-analyze/1` request over a program that declares one or more
  `compose S rule R { ... }` (FSpec FR-491) with the component packages it
  imports.
- An `items` entry `{"claim": "S", "kind": "composition"}`, `S` the
  declaration's name. `kind` is required: the `protocol` family admits several
  kinds, so FR-620's `absent-kind` rule applies. The entry carries `required`,
  `backend` and `options` as any item does, and carries no `premises`.

## Outputs

Each selected composition writes, in this order and only for the composition
it selects:

1. The FR-621 item at the entry's `request_index`.
2. Its premise items, after the caller's items and after FSpec FR-368's
   derived items, grouped per selected composition in the order of the
   caller's entries and, within a group, in FSpec FR-491 slot order: one item
   per `component-guarantee` slot, then the `fixed-point` slot's item when the
   composition has one. A `rely`, `liveness-dependency`, `fairness` or
   `environment` slot is not item-backed and writes no item.

A premise item with the same slot key as another derived item is written once,
at its first position, and shared. A caller's own entry is never merged with a
derived item, and the same name listed twice writes two composition items, at
two indices, over one shared set of premise items.

- The composition item's record is `unsupported`, settled at negotiation
  (FR-621 `S6b`), with diagnostic `unsupported_projection`/
  `unsupported-requested-capability` naming kind `composition`, no `backend`
  and no counterexample. An `assume-guarantee` selection writes the same
  record and no FSpec FR-331 request item.
- Each premise item carries its own record, extent, trust basis and
  certification (FR-530-AC-6). The composition inherits none: a `proved`
  premise does not make it `proved`, and a `refuted` one does not make it
  `refuted`.
- The document's category and exit code are FR-285's multi-item code, the most
  severe present in the order 30, 20, 21, 22, 10, 11, 0.

## Behavior

- When an entry names a checked `compose` declaration under `interface`,
  `protocol-refinement` or `refinement-transfer`, FSL SHALL resolve it by
  FR-620's name lookup and build the item `declaration_items` writes for it,
  whose `premises` are the declaration's slots in order.
- When no registered backend advertises `composition`, FSL SHALL settle the
  item `unsupported` naming the capability, and SHALL NOT settle it `proved`,
  `refuted` or `inconclusive`.
- When an entry names a declaration under `assume-guarantee` with `kind`
  `composition`, FSL SHALL settle it `unsupported` as above without routing it
  to a provider.
- When an entry names a name no declaration resolves, FSL SHALL settle that
  item `invalid-request` with `invalid_capability`/`unknown-claim` naming the
  name, and the request's other items still run.
- When an entry's `kind` does not apply to its claim, whether the claim is not
  a composition (`kind` `composition`) or is one (another `kind`), FSL SHALL
  settle that item `invalid-request` with FR-507's `inapplicable-kind` and its
  payload `request_index`, `capability` and `family` (absent for an
  `assume-guarantee` claim), and no other member.
- When an entry names a composition with no `kind`, FSL SHALL settle that item
  by FR-620's `absent-kind`.
- If an entry has `kind` `composition` and a `premises` member, then FSL SHALL
  refuse the whole request at the request stage with `invalid-request`, exit
  20, naming `/items/<i>/premises`, and SHALL run no item, as FR-620 does for
  `request_index`, `extent` and `candidates`.
- When a declaration's dependency graph has a cycle and no selected profile
  declares FSpec FR-176's lattice, FSL SHALL settle the composition item
  `invalid-request` with `invalid_model_binding`/`malformed-declaration`
  naming the cycle's guarantees in edge order, as FSpec FR-491 states,
  without deriving a premise item for it.
- When a cycle's profile declares the lattice, FSL SHALL write the
  `fixed-point` slot's item as one more premise item, and SHALL settle it as it
  settles the composition.
- When a component-guarantee premise does not settle `proved`, FSL SHALL
  return that premise's own record and SHALL NOT settle the composition
  `proved`.
- When a request has no `items` member, FSL SHALL add the checked compose
  declarations to FR-620's default items, one `composition` item each in
  source order, with their premise items after FSpec FR-368's derived items,
  as FR-620's default rule gives every claim of the program. An
  `assume-guarantee` declaration is a default item of FR-620's rule only as
  far as FSpec FR-290 gives it an item, which is none.
- When a `package` request (FR-850) selects the same unit's checked package,
  FSL SHALL settle the same items, records and exit code as the source request.

## Acceptance Criteria

The fixture: domain packages `test/producer` and `test/consumer`, one model
each (`Producer`, `Consumer`, a field `value: Int[0, 3]`, an operation `inc`
that stays within it), imported by a unit as `P` and `C` and selected as `PM`
and `CM`, with the clauses
`temporal PSafe using t0 over (p: PM::Producer) clock "model-steps" on origin { always holds(p.value <= 3) }`
and the same `CSafe` over `CM::Consumer`, each proved by EN-1 alone, and
FR-620's Counter clause `Bounded`. The unit declares
`compose Sys rule interface { component P1 from P using PM; component C1 from C using CM; guarantee P1::PSafe; guarantee C1::CSafe; }`;
`SysRely`, the same with `rely C1::CSafe on P1::PSafe;`; `SysAG`,
`SysPR` and `SysRT`, `Sys` under `assume-guarantee`, `protocol-refinement`
and `refinement-transfer`; and `SysCycle`, `Sys` with
`rely C1::CSafe on P1::PSafe;` and `rely P1::PSafe on C1::CSafe;` in a unit
that selects no lattice profile. Every request runs through `fsl analyze`,
machine and text mode.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1044-AC-1 | The item `{"claim": "Sys", "kind": "composition"}` writes the composition at `request_index` 0 as `unsupported` with `unsupported_projection`/`unsupported-requested-capability` naming `composition`, no `backend`, empty `candidates`, and two premise items at indices 1 and 2, `PSafe` over `PM::Producer` and `CSafe` over `CM::Consumer`, each `proved`; the document exits 21 and no item is `proved` for the composition. | Test |
| FR-1044-AC-2 | `SysPR`, `SysRT` and `SysAG` selected as in AC-1 each write the composition `unsupported` as `Sys` does and exit 21; `SysAG` writes no FSpec FR-331 request item and its premise items are indices 1 and 2. | Test |
| FR-1044-AC-3 | The items `{"claim": "Sys", "kind": "composition"}`, `{"claim": "Missing", "kind": "composition"}`, `{"claim": "Bounded", "kind": "composition"}`, `{"claim": "Sys", "kind": "temporal-satisfaction"}` and `{"claim": "Sys"}` settle, in order: `unsupported`; `invalid-request` `invalid_capability`/`unknown-claim` naming `Missing`; `invalid-request` `inapplicable-kind` with payload `request_index` 2, `capability` `composition`, `family` `temporal-trace`; `invalid-request` `inapplicable-kind` with `capability` `temporal-satisfaction`, `family` `protocol`; `invalid-request` `absent-kind`; the document exits 20. | Test |
| FR-1044-AC-4 | `{"claim": "SysAG", "kind": "temporal-satisfaction"}` settles `invalid-request` `inapplicable-kind` with payload `request_index`, `capability` and no `family`; exit 20. | Test |
| FR-1044-AC-5 | The item `{"claim": "Sys", "kind": "composition", "premises": []}` refuses the whole request at the request stage, `invalid-request`, exit 20, naming `/items/0/premises`, and writes no item. | Test |
| FR-1044-AC-6 | `SysRely` selected as in AC-1 writes its premise items at indices 1 and 2 and no item for the `rely` slot; the `CSafe` premise item is not the item `{"claim": "CSafe"}` requests alone, its slot key and request identity differing by the assumption `PSafe`, and its record is the one negotiation gives it, never copied from `Sys`'s. | Test |
| FR-1044-AC-7 | `SysCycle` selected as in AC-1 writes one item, `invalid-request` `invalid_model_binding`/`malformed-declaration` naming `C1::CSafe` and `P1::PSafe` in edge order, no premise item; exit 20. | Test |
| FR-1044-AC-8 | The items `{"claim": "Sys", "kind": "composition"}` listed twice write two composition items at indices 0 and 1 and one `PSafe` and one `CSafe` premise item at indices 2 and 3; the same two items plus `{"claim": "SysRely", "kind": "composition"}` write the `PSafe` item once, shared. | Test |
| FR-1044-AC-9 | The AC-1 unit with one component's guarantee violated by its own component writes that premise item `refuted` with its counterexample and the other `proved`, and the composition `unsupported` and not `refuted`; each premise keeps its own extent and certification; the document exits 21. | Test |
| FR-1044-AC-10 | A request with no `items` member over the unit declaring only `Sys` writes `Sys` as the composition at index 0 and its premises at 1 and 2, after the items FR-620 gives for the unit's other claims in source order, and exits 21; over the unit's `package` handoff (FR-850) it writes the same items, records and exit code. | Test |

## Dependencies

- FR-530 (compile, emit and read the composition declaration and its derived
  item and slots); FR-620 and FR-621 (the analyze request, item resolution,
  default items and outcome document); FR-507 (the family and the
  `inapplicable-kind` rule; its rows for a composition are added with this
  requirement); FR-630 (the precedent for selecting a declaration by name);
  FR-850 (the package route); FR-281 and FR-285 (negotiation and exit codes).
- FSpec FR-348 (the routed composition obligation; AC-1 and AC-2 stay with its
  provider), FSpec FR-491 (the declaration, derived item, slots and cycles),
  FSpec FR-492 (the planner settlement an `assume-guarantee` item uses),
  FSpec FR-489 (the premise rule), FSpec FR-290 (the family) and FSpec FR-331
  (the `composition` item and its `premises`).
