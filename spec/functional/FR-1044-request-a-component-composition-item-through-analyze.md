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
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: depends_on
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
(FSpec FR-348-AC-4); FSL never settles it `proved` itself. A registered
provider's answer is admitted only as FSpec FR-331 states: its `proved` stands
only over the request's own proved component guarantees, and its `refuted` only
as a declared interface mismatch FSL recomputes from the declaration.

### Decisions

- **The selection is the existing item.** FSpec FR-491 "Derived item and
  slots" derives, for a `compose` declaration under any of the four rules, one
  FSpec FR-331 `composition` request item when the declaration is selected by
  name, and FR-530 compiles, emits and reads it
  (`RequestWriter::declaration_items`). A source request selects it as a
  refinement is selected (FR-630): `claim` the declaration's name, `kind`
  `composition`. The architecture ruling on QSL-2739 chose this over a new verb
  or request family, which would restate FR-620's name lookup, kind admission,
  negotiation and outcome rules.
- **The family is `protocol`, under every rule.** FSpec FR-290's claim-form rows
  give a composition under `protocol-refinement`, `refinement-transfer` and
  `interface`, and an `assume-guarantee` declaration selected by name as an
  item of an analyze request, the kind `composition` and the family `protocol`.
  FR-507 states the same rows and reads the family from the claim's
  `correspondence`/`component_composition` node. An item of any other kind on
  such a claim is inapplicable by FR-507's rule.
- **An `assume-guarantee` selection is the same routed item.** It is the one
  FR-331 `composition` item of the other rules, with its `premises` and
  `assumptions`, negotiated to the registered `composition` route; it is not
  settled by FSL's planner. FSpec FR-492's planner settlement of the same
  declaration stays the separate entry FR-530 owns, and no analyze request
  reaches it. FSL adds no native assume-guarantee, fixed-point or protocol
  algorithm and no generic provider API: the provider the item reaches is a
  registered external provider (FR-635), as for the other rules. Until a
  registered backend advertises `composition`, every rule settles `unsupported`
  naming `composition`.
- **The premises are derived; a caller may only restate their keys.** The
  slots are FSpec FR-491's, in its order. FSpec FR-520 keeps `premises` a
  caller item member, because it names slot keys and never a proof or a
  result: a proof or result is never external to FSL (FSpec FR-489). So FR-620
  is unchanged, and for a composition item only, FSL accepts a `premises`
  member that is the derived slot list exactly and refuses any other. The
  member of every other kind keeps its existing admission, a `realizability`
  item's `premises` groups (FR-571) included. A caller's item has no
  `assumptions` member: FR-620 closes the item, so a caller's `assumptions` is
  an unknown member.
- **The assumptions are request identity, derived beside the premises.** FSL
  writes the FSpec FR-331 `assumptions` member of every composition item it
  writes, beside `premises` and from the declaration alone (FSpec FR-491
  "Assumption binding"): one array per `component-guarantee` entry of
  `premises`, in `premises` order, each strictly ascending and holding the
  `premises` indices of the peer `component-guarantee` entries that
  guarantee's `rely` and `depends` members name, and empty when it assumes none.
  Under `protocol-refinement`, `refinement-transfer` and `interface` every array
  is empty. An `H` that no declared `guarantee` names contributes no index. FSL
  admits a composition request item only when its `assumptions` is exactly the
  derived array and otherwise refuses `invalid_runtime_input`/`invalid-value` at
  `/request/items/<i>/assumptions` (a wrong length, an out-of-range, wrong-role
  or self index, an array not strictly ascending, and a well-formed array that
  adds, drops or moves a peer index). Two composition items equal except in a
  valid `assumptions` are different requests.
- **A premise item is its slot's item, not the clause requested alone.** A
  `component-guarantee` slot's item is `G` over `C`'s subject checked under
  exactly the assumptions of that slot's entry of the composition's
  `assumptions` (FSpec FR-491 "Derived item and slots"), and the assumption set
  is part of its slot key. FSL's typed slot key is the item's request identity
  plus that set (FSpec FR-492 "Request"). The item `declaration_items` writes
  for the slot is an ordinary `temporal-satisfaction` item: it has no
  assumptions member and FSL adds none, because FSpec FR-491 puts the set on
  the composition item and on the slot key only. So the `request` member of one
  guarantee under two assumption sets MAY be equal, and the two items are told
  apart by the typed key. No engine `Engines::fsl()` registers checks a clause
  under a peer's assumptions, and FSL never runs a slot with a non-empty set as
  the clause alone, which would settle another obligation under that slot's
  key. So a premise item whose assumption set is non-empty settles
  `unsupported`, naming kind `temporal-satisfaction`, before negotiation, and a
  premise item with an empty set is run by the engine that takes its clause.
- **The composition names its premises by slot key.** FSpec FR-331 `premises`
  holds `{role, key}` per slot and no result. A routed composition item holds
  the slots of FSpec FR-331's five roles (`component-guarantee`, `rely`,
  `liveness-dependency`, `fairness`, `environment`), which are FSpec FR-489's
  four premise lists and the component guarantees. FSpec FR-491 derives no
  `fixed-point` slot and FSL writes none (see the cycle rule below). FSL
  settles every premise item before it writes the composition item's outcome,
  and it never copies a premise result into the composition.
- **A declared interface mismatch refutes; a premise that did not prove does
  not.** FSpec FR-492 "Interface mismatch" decides a mismatch from the
  declaration alone, in one reason-major order: first every `absent` mismatch
  (a `rely` or `liveness-dependency` slot naming a peer `D::H` with no
  `component-guarantee` slot over `D`'s subject at `H`'s clause), over those
  slots in ascending slot order, and only when there is none every `weaker`
  mismatch (the peer's declared guarantee is conditional on a fairness
  constraint the composition supplies nothing that implies, FSpec FR-380
  "Fairness implication") in the same order. The first mismatch found settles
  the item, so an `absent` slot is reported although a lower slot is `weaker`.
  A provider's `refuted` answer for a composition item carries the FSpec FR-331
  `interface-mismatch` counterexample `{type, component, interface, premise,
  reason}`: `component` the peer's name, `interface` the node of the peer
  guarantee's clause, `premise` the `premises` index of the entry that needs it
  and `reason` `absent` or `weaker`. FSL admits that witness only when it is
  closed (exactly those members, `reason` one of the two spellings) and equals
  the first mismatch FSL recomputes from the checked declaration. FSL decides
  nothing it was not asked: with no registered provider the item is still
  `unsupported`, and the recomputation only admits or refuses a provider's
  answer. A premise whose declared interface exists and is not weaker, whose own
  item did not settle `proved`, is never a mismatch.

## Inputs

- A `native-analyze/1` request over a program that declares one or more
  `compose S rule R { ... }` (FSpec FR-491) with the component packages it
  imports.
- An `items` entry `{"claim": "S", "kind": "composition"}`, `S` the
  declaration's name. `kind` is required: the `protocol` family admits several
  kinds, so FR-620's `absent-kind` rule applies. The entry carries `required`,
  `backend` and `options` as any item does. It MAY carry `premises` and then
  only as Behavior states: the derived slot list, never a proof or result.

## Outputs

Each selected composition writes, in this order and only for the composition
it selects:

1. The FR-621 item at the entry's `request_index`.
2. Its premise items, after the caller's items and after FSpec FR-368's
   derived items, grouped per selected composition in the order of the
   caller's entries and, within a group, in FSpec FR-491 slot order: one item
   per `component-guarantee` slot. A `rely`, `liveness-dependency`, `fairness`
   or `environment` slot is not item-backed and writes no item. FSL writes no
   `fixed-point` item.

A premise item with the same slot key as another derived item is written once,
at its first position, and shared. A caller's own entry is never merged with a
derived item, and the same name listed twice writes two composition items, at
two indices, over one shared set of premise items.

- The composition item of any rule, `assume-guarantee` included, has the
  `request` member `ClaimItem::request_item_wire` writes, its `premises` the
  derived slot list and its `assumptions` the derived array beside it, empty
  `candidates` when no backend advertises `composition`, and then a record
  `unsupported`, `settled_by` `disposition`, no `backend`, no counterexample,
  settled at negotiation (FR-621 `S6b`), with diagnostic
  `unsupported_projection`/`unsupported-requested-capability` naming kind
  `composition`.
- The composition item of a declaration whose dependency graph holds a cycle,
  under any of the four rules, is settled before any premise is read and
  before any request is written: no `request` member and no `candidates`, no
  negotiation, `claim_name` the declaration's name, record `unsupported`,
  basis `unavailable`, `settled_by` `disposition`, no `backend`, stage `S4`
  (FR-621, an item settled by a disposition before any engine ran), diagnostic
  `unsupported_projection`/`unsupported-requested-capability` whose `detail`
  names the kind `composition`, the form `fixed-point` and the cycle's
  guarantees in edge order (FSpec FR-489 "Cycles"). It derives no premise item.
- A composition item a registered provider settles `refuted` with an admitted
  `interface-mismatch` has the record `refuted`, basis `decisive-counterexample`,
  the typed witness as its `counterexample` entry built, not replayed (FR-621
  `S7`), no `replay` member, no certification and no conclusion; its premise
  items keep their own records, in slot order.
- Each premise item carries its own record, extent, trust basis and
  certification (FR-530-AC-6); a premise item with a non-empty assumption set
  is settled `unsupported` at stage `S4` as Behavior states. The composition
  inherits none: a `proved` premise does not make it `proved`, and a `refuted`
  one does not make it `refuted`.
- The document's category and exit code are FR-285's multi-item code, the most
  severe present in the order 30, 20, 21, 22, 10, 11, 0.

## Behavior

- When an entry names a checked `compose` declaration under any rule, FSL SHALL
  resolve it by FR-620's name lookup and build the item `declaration_items`
  writes for it, whose `premises` are the derived slot list and whose
  `assumptions` are the derived array.
- When no registered backend advertises `composition`, FSL SHALL settle the
  item `unsupported` naming the capability, and SHALL NOT settle it `proved`,
  `refuted` or `inconclusive`.
- When a premise item's assumption set is non-empty, FSL SHALL settle it
  `unsupported` before negotiation, by a disposition at stage `S4`, with the
  diagnostic `unsupported_projection`/`unsupported-requested-capability`
  naming kind `temporal-satisfaction`, its `request` echo as written, empty
  `candidates` and no `backend`, without running an engine on it.
- When an entry names a name no declaration resolves, FSL SHALL settle that
  item `invalid-request` with `invalid_capability`/`unknown-claim` naming the
  name, and the request's other items still run.
- When an entry's `kind` does not apply to its claim, whether the claim is not
  a composition (`kind` `composition`) or is one (another `kind`), FSL SHALL
  settle that item `invalid-request` with FR-507's `inapplicable-kind` and its
  payload `request_index`, `capability` and `family`: `protocol` for a
  composition claim of any rule, the claim's own family otherwise.
- When an entry names a composition with no `kind`, FSL SHALL settle that item
  by FR-620's `absent-kind`.
- If a `kind` `composition` item carries a `premises` member that is not an
  array of closed objects `{role, key}`, `role` one of FSpec FR-331's five
  roles, then FSL SHALL refuse the whole request at the request stage with
  `invalid-request`, exit 20, naming the member's RFC 6901 pointer
  (`/items/<i>/premises`, or `/items/<i>/premises/<j>` and its member), running
  no item, as FR-620 does for any mistyped member. A `premises` member of an
  item of another `kind` keeps its existing admission.
- When a `kind` `composition` item carries a well-formed `premises` that
  parsed equals the derived slot list, FSL SHALL treat the item as one with no
  `premises`. Two lists are equal when they have the same length, and at each
  position the same `role` and the same `key` as parsed JSON values, so the
  order of an object's members is not significant and the order of the list
  is.
- If a `kind` `composition` item carries a well-formed `premises` that is not
  equal to the derived slot list (a slot removed, added, reordered, relabelled
  or with another key), then FSL SHALL settle that item, after name and kind
  admission and the cycle answer, `invalid-request` at stage `S4` with
  `invalid_runtime_input`/`invalid-value` at `/items/<i>/premises`, as FSpec
  FR-492-AC-2 refuses a slot list that differs from the derived one, deriving
  no premise item for it, and the request's other items still run. The
  comparison is made after the declaration is checked, because the derived
  list does not exist before. An item naming an unknown name or a name that is
  not a composition keeps its code from the bullets above, derives no premise
  item and compares no `premises`.
- When a checked declaration's dependency graph (FSpec FR-491) holds a cycle,
  FSL SHALL settle the composition item `unsupported`, as Outputs gives, under
  every one of the four rules and whether or not a provider is registered,
  before it reads any premise, runs the planner or a provider, or writes a
  request to a provider, so that no supplied result, discharge entry or
  provider answer can make it `proved`. FSL SHALL derive no `fixed-point` slot
  or item, and no selected profile changes this answer: no requirement yet
  defines how a profile declares a lattice. A declaration that is not well
  formed keeps its own located refusal (FSpec FR-491 "Static checks"), distinct
  from this answer, and the unit's other items are unaffected.
- When a component-guarantee premise does not settle `proved`, FSL SHALL
  return that premise's own record and SHALL NOT settle the composition
  `proved`.
- When a request has no `items` member, FSL SHALL add the checked compose
  declarations to FR-620's default items, one `composition` item each in
  source order, with their premise items after FSpec FR-368's derived items,
  as FR-620's default rule gives every claim of the program. An
  `assume-guarantee` declaration is not a default item: FSpec FR-491 selects
  it by name only.
- When a `package` request (FR-850) selects the same unit's checked package,
  FSL SHALL settle the same items, records and exit code as the source request.

### Registered provider

- When a registered backend advertises `composition`, FSL SHALL negotiate the
  routed item, of any rule, to it (FSpec FR-348-AC-3) and send it, through the
  external provider request (`Engine::external`, FR-635), the item with its
  derived `premises` slot keys, its derived `assumptions` and its `options`. The
  `options` are negotiated as for any provider item: FR-967 for a named backend,
  FR-243 and FR-635 against the candidates' manifests otherwise. FSL sends no
  premise result and no component proof with it, and adds no provider API,
  verb or grammar. FSL settles the provider's verdict as for any external
  provider, with these causes: `unknown` is `inconclusive` `solver-inconclusive`
  (FSpec FR-331); `proved` carries the certification FSL assigns the provider's
  origin (FR-635: `trusted` for a plugin, `uncertified` for a linked provider),
  never one the provider states, and is subject to the guard below; and
  `refuted` is admitted only as a declared interface mismatch, below.
- **Producer-only guard.** FSL SHALL apply FSpec FR-331's `core-guard cause`
  to every `proved` composition answer that carries `premises`, whether or not
  its request holds the component items. FSL decides from its own records of the
  same request, never from the provider: it reads each `component-guarantee`
  entry `k` by the request index of the item whose typed slot key was given, and
  a held item matches the entry only when its request identity and the derived
  `k`th `assumptions` array both equal the entry's, never by its claim node
  alone, so a guarantee equal on the wire but checked under other peer
  guarantees never discharges this one. When every entry is held and settled
  `proved`, the answer stands. Otherwise FSL SHALL replace it with
  `inconclusive`, cause `provider-answer-unconfirmed` and `premise` the
  `premises` index of the first `component-guarantee` entry, in `premises` order,
  whose matching item the request does not hold or does not settle `proved`
  (by an engine, by disposition before any engine ran, or not at all, and
  including a request that holds no component items), with no certification.
  The cause is not `replay-refused` and not `premise-unproved`. A composition
  with no `component-guarantee` entry has no premise to name, so the guard
  confirms nothing and the provider's `proved` is not replaced. FSL runs no
  composition algorithm of its own and adds no proof.
- **Provider-supplied causes.** FSL SHALL NOT let a provider, linked or plugin,
  supply `provider-answer-unconfirmed`: a result entry that carries it, on a
  `composition` item or any other, is refused `invalid_result`/`unknown-cause`
  at `/cause/type`, by the same reader that refuses `premise-unproved` and
  `transfer-not-preserving` from a provider.
- **Interface mismatch.** When a provider settles a composition item
  `refuted` with an `interface-mismatch` counterexample, FSL SHALL recompute the
  first mismatch from the checked declaration in FSpec FR-492's reason-major
  order, and SHALL settle the item `refuted`, `decisive-counterexample` as
  Outputs gives only when the witness is closed and equal to that mismatch in
  `component`, `interface`, `premise` and `reason`. A witness with another
  member, another `reason` spelling, other values, or a declaration that holds
  no mismatch, and every other `refuted` counterexample of a composition (an
  assignment list, for one), is not accepted: the item settles `inconclusive`
  `replay-refused` with the code `unknown_required_feature`, because FSL
  recomputes only the declared mismatch (FSpec FR-290 "Replay context"), and no
  provider's `refuted` is settled `refuted` on its word.

### Current system behavior

The architecture ruling on QSL-2739 (PR #3287) lands this requirement with
the following typed results where the system cannot yet deliver an AC. Each
AC below stays as written and open on QSL-2739.

- **AC-10, `package` handoff.** Today a `package` handoff of a unit that
  declares compositions is admitted, and the package route reads no
  composition term's slots or component packages. A request with no `items`
  is settled whole as unsupported at the request stage, category
  `unsupported`, `unsupported_construct`/`not-yet-implemented` naming the
  `composition` rule, exit 21, with no item. An item whose name stays unresolved after
  ordinary claim, declaration and model-alias resolution, which includes a
  composition's name, is settled the same way naming the `unnamed-claim`
  family, never as `unknown-claim`, because the package carries no name row
  for a composition. A name an ordinary row carries resolves as usual, a
  non-claim declaration name is refused as `declaration`, and a derived item
  whose model name does not match settles `model-alias`. These typed results
  are public; the source/package parity of AC-10 stays open and is deferred to the
  package-consumer API lane, which owns the package form.
- **Text mode.** Text rendering of the outcome belongs to the external driver.
  No test in this repository exercises it, and this requirement does not
  weaken the text half of the fixture.

## Acceptance Criteria

The fixture. The domain package `test/counter` is FR-620's Counter fixture
(field `value: Int[0, 3]`, operation `inc`), and a request's default subject
is that package with FR-620's one initialization snapshot `start` holding
counter `a` at 0, which is the initial state every clause below is checked
from. Two checked component packages each select `test/counter` as `M`:
`test/part-a` declares
`temporal Bounded using t0 over (c: M::Counter) clock "model-steps" on origin { always holds(c.value <= 3) }`
and `test/part-b` declares the same clause named `Capped`. Their subject
identities differ, being their own package's and `M`'s. The unit
`test/system` imports them as `A` and `B`, selects `test/counter` as `M`,
declares the clause `Local` (`always holds(c.value <= 3)`, family
`temporal-trace`), and declares, each over
`component PA from A using M; component PB from B using M; guarantee PA::Bounded; guarantee PB::Capped;`:
`Sys` under `interface`; `SysPR` and `SysRT` under `protocol-refinement` and
`refinement-transfer`; `SysAG` under `assume-guarantee`; `SysRely`, `Sys`
with `rely PB::Capped on PA::Bounded;`; `SysAGRely`, `SysAG` with the same
`rely`; `SysCycle`, `Sys` with that and `rely PA::Bounded on PB::Capped;`, and
`SysCyclePR`, `SysCycleRT` and `SysCycleAG` the same body under
`protocol-refinement`, `refinement-transfer` and `assume-guarantee`; and
`SysAbsent`, over the same components with only `guarantee PB::Capped;` and
`rely PB::Capped on PA::Bounded;`, under `assume-guarantee` (`PA::Bounded` is no
declared guarantee, so the peer interface is absent). The unit `test/system-lattice` is
`test/system` selecting the profile `fixed-point-lattice` (the profile FR-530's
emit fixture declares FSpec FR-176's lattice with), declaring `SysLat`
(`SysCycle` under `interface`) and `SysLatAG` (under `assume-guarantee`). The
unit `test/system-default` imports the same packages, selects `test/counter`
as `M`, declares no `pre` or `post` clause on `M::Counter::inc`, the clause
`Local`, then `Sys`. Every request runs through `fsl analyze`, machine and text mode.
A provider fixture is a recording provider registered with `Engine::external`
that advertises `composition`, keeps each request it receives and answers the
verdict the criterion names.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1044-AC-1 | Over `test/system`, the item `{"claim": "Sys", "kind": "composition"}` writes the composition at `request_index` 0 with a `request` member whose `premises` are the two component-guarantee slot keys (the derived slot list), whose `assumptions` is `[[], []]` and empty `candidates`, and a record `unsupported`, `settled_by` `disposition`, no `backend`, diagnostic `unsupported_projection`/`unsupported-requested-capability` naming `composition`; it writes two premise items, `Bounded` at index 1 and `Capped` at index 2, each `proved` by EN-1; the document exits 21 and the composition is not `proved`. | Test |
| FR-1044-AC-2 | `SysPR`, `SysRT` and `SysAG` write the composition as AC-1 does, each with a `request` member of family `protocol`, empty `candidates` and a record `unsupported` by `disposition` naming `composition` at negotiation, with premise items `Bounded`, `Capped` at indices 1 and 2, and exit 21; `SysAG` carries a `request` member and is not settled by a planner-local disposition. | Test |
| FR-1044-AC-3 | The items `{"claim": "Sys", "kind": "composition"}`, `{"claim": "Missing", "kind": "composition"}`, `{"claim": "Local", "kind": "composition"}`, `{"claim": "Sys", "kind": "temporal-satisfaction"}`, `{"claim": "Sys"}` and `{"claim": "SysAG", "kind": "temporal-satisfaction"}` settle, in order: `unsupported`; `invalid-request` `invalid_capability`/`unknown-claim` naming `Missing`; `invalid-request` `inapplicable-kind` with payload `request_index` 2, `capability` `composition`, `family` `temporal-trace`; `invalid-request` `inapplicable-kind` with `request_index` 3, `capability` `temporal-satisfaction`, `family` `protocol`; `invalid-request` `absent-kind`; `invalid-request` `inapplicable-kind` with `request_index` 5, `capability` `temporal-satisfaction` and `family` `protocol`; the document exits 20. | Test |
| FR-1044-AC-4 | Over `test/system`: (a) `{"claim": "Sys", "kind": "composition", "premises": P}`, `P` AC-1's `request.premises` with the members of each object written in reverse order, writes AC-1's items, records and exit; the same with `SysAG` and `P` its derived slot list (the same two keys) writes AC-2's items, records and exit. (b) With `P` having the `Capped` slot removed, the two slots swapped, or the second slot's `role` relabelled `rely`, and the same three variants over `SysAG`, each followed by the item `{"claim": "Local"}`, the composition item settles `invalid-request` at stage `S4` with `invalid_runtime_input`/`invalid-value` at `/items/0/premises`, no premise item is written, `Local` is `proved`, and the document exits 20. (c) `"premises": {}`, `"premises": [1]` and `"premises": [{"role": "nope", "key": {}}]` each refuse the whole request at the request stage, `invalid-request`, exit 20, naming `/items/0/premises`, `/items/0/premises/0` and `/items/0/premises/0/role`, and write no item. (d) `{"claim": "Missing", "kind": "composition", "premises": []}` settles `unknown-claim` and `{"claim": "Local", "kind": "composition", "premises": []}` settles `inapplicable-kind`, each per item with no premise item. (e) A FR-571 `realizability` item carrying `premises` `{"role_knowledge": [], "environment": []}` is not refused for `premises` and settles as FR-571 gives it. (f) An `assumptions` member on a caller's `Sys` or `SysAG` item refuses the whole request at the request stage as an unknown member, exit 20, naming `/items/0/assumptions`, and writes no item. | Test |
| FR-1044-AC-5 | `SysRely` (`interface`) selected as in AC-1 derives no assumption set: it writes `Bounded` at index 1 and `Capped` at index 2 each `proved` by EN-1, its `request` member's `assumptions` is `[[], []]`, no item is written for the `rely` slot, the composition is `unsupported` naming `composition`, exit 21. `SysAGRely` selected the same way writes `Bounded` at index 1 `proved`, and at index 2 `Capped`, whose assumption set is `Bounded`, with the `request` member of the clause as `test/part-b` declares it, empty `candidates` and no `backend`: a record `unsupported`, `settled_by` `disposition`, stage `S4`, diagnostic `unsupported_projection`/`unsupported-requested-capability` naming `temporal-satisfaction`, no engine run; the composition's `request.assumptions` is `[[], [0]]`, the composition is `unsupported` naming `composition`, and the document exits 21. | Test |
| FR-1044-AC-6 | Over `test/system`, each of `SysCycle`, `SysCyclePR`, `SysCycleRT` and `SysCycleAG` writes one item and no premise item: no `request` member and no `candidates`, `claim_name` the declaration's name, a record `unsupported`, basis `unavailable`, `settled_by` `disposition`, no `backend`, stage `S4`, diagnostic `unsupported_projection`/`unsupported-requested-capability` whose `detail` names kind `composition`, form `fixed-point` and `PB::Capped` and `PA::Bounded` in edge order; exit 21. This holds with no provider registered and with a provider fixture registered that answers `proved`: the provider fixture receives no request, no component item runs and no premise item is written, and the unit's other items settle unchanged. A cycle declaration with a `rely` of a component on itself still refuses `invalid_model_binding`/`malformed-declaration`, exit 20, distinct from this answer. | Test |
| FR-1044-AC-7 | Over `test/system-lattice`, `SysLat` and `SysLatAG` each write the AC-6 item alone: the selected profile changes nothing, no `fixed-point` item and no premise item is written, and the provider fixture, when registered, receives no request; each exits 21. | Test |
| FR-1044-AC-8 | The items `Sys` twice (`kind` `composition`) write two composition items at indices 0 and 1 and the `Bounded` and `Capped` items once, at indices 2 and 3. With `SysAGRely` as a third entry, the compositions are at 0, 1 and 2, `Bounded` at 3 (shared, written once) and `Capped` at 4 `proved`, and at 5 a second `Capped`, from `SysAGRely`'s group, whose `request` member equals item 4's and whose record is `unsupported` as AC-5 states. | Test |
| FR-1044-AC-9 | With `test/part-b`'s `Capped` replaced by `always holds(c.value <= 2)`, `Sys` writes `Capped` `refuted` with a counterexample ending in `value = 3` and `Bounded` `proved`, each keeping its own extent and certification, and the composition `unsupported` and not `refuted`; the document exits 21. | Test |
| FR-1044-AC-10 | Over `test/system-default`, a request with no `items` writes `Local` at index 0, `Sys` at 1, the subject's derived deadlock-freedom item at 2 (FR-620-AC-5), then the premise items `Bounded` at 3 and `Capped` at 4, and exits 21; over the unit's `package` handoff (FR-850) it writes the same items, records and exit code. | Test |
| FR-1044-AC-11 | With the provider fixture registered, `Sys`, `SysAG` and `SysAGRely` each reach it as one `composition` item of family `protocol` through `Engine::external` with the derived `premises` and `assumptions` (`[[], []]`, `[[], []]` and `[[], [0]]`), no premise result and no component proof; the requests of `SysAG` and `SysAGRely` differ by `assumptions` alone and are different requests, and the existing provider options, limits and negotiation are unchanged. A composition request item whose `assumptions` adds, drops or moves a peer index, or has the wrong length, an out-of-range, wrong-role or own-entry index, or an entry not strictly ascending, is refused by FSL's admission `invalid_runtime_input`/`invalid-value` at `/request/items/0/assumptions`, and no provider is called. | Test |
| FR-1044-AC-12 | With the provider fixture answering `proved` for `SysAG`: when `Bounded` and `Capped` are held and `proved`, the composition is `proved`, certified as the provider's origin gives (`trusted` plugin, `uncertified` linked), and `Capped` replaced by `always holds(c.value <= 2)` (refuted) or `SysAGRely`'s assumed `Capped` (unsupported) settles the composition `inconclusive`, cause `provider-answer-unconfirmed` with `premise` the `premises` index of the first such component-guarantee entry (1 for both), no certification, and not `replay-refused` or `premise-unproved`. At the guard seam, an answer over a request that holds no component item, and over a held item with the same clause node and other identity members or another assumption array, settles `provider-answer-unconfirmed` naming the first entry. | Test |
| FR-1044-AC-13 | A linked provider and a plugin provider fixture each answering `inconclusive` with cause `provider-answer-unconfirmed` for a composition item, and for an item of another kind, are refused `invalid_result`/`unknown-cause` at `/cause/type`, as is a cause `premise-unproved` or `transfer-not-preserving`; the settled item is the refusal and never the supplied cause. | Test |
| FR-1044-AC-14 | With the provider fixture answering `refuted` with an `interface-mismatch` witness for `SysAbsent` that names component `PA`, `PA::Bounded`'s clause node as `interface`, `premise` the `premises` index of the `rely` entry (1) and `reason` `absent`, the composition settles `refuted`, basis `decisive-counterexample`, with the witness as its `counterexample` entry, no `replay` member, no certification, no conclusion, category `violation` and exit 10, and its premise items keep their own records in slot order. The same witness changed in `component`, `interface`, `premise` or `reason`, with an extra member, with a `reason` spelled other than `absent` or `weaker`, or answered for `Sys` (no mismatch in its declaration), settles `inconclusive` `replay-refused` `unknown_required_feature`, as does a `refuted` composition answer carrying an assignment counterexample. With the provider fixture answering `proved` for `Sys` over AC-9's refuted `Capped` (a declared interface that is present and not weaker, whose premise did not prove), the composition is `provider-answer-unconfirmed` and never `refuted`; with no provider registered `SysAbsent` is `unsupported` naming `composition`. | Test |

## Dependencies

- FR-530 (compile, emit and read the composition declaration and its derived
  item and slots); FR-620 and FR-621 (the analyze request, item resolution,
  default items and outcome document); FR-507 (the family and the
  `inapplicable-kind` rule; its rows for a composition are added with this
  requirement); FR-630 (the precedent for selecting a declaration by name);
  FR-850 (the package route); FR-281 and FR-285 (negotiation and exit codes).
- FR-635 (the registered external provider route the composition item reaches).
- FSpec FR-348 (the routed composition obligation and its interface-mismatch
  refutation), FSpec FR-491 (the declaration, derived item, slots, assumption
  binding and cycles), FSpec FR-492 (the slot key and the reason-major
  interface-mismatch order; its planner settlement is FR-530's, not reached
  here), FSpec FR-489 (the premise rule and the cycle answer), FSpec FR-290 (the
  family) and FSpec FR-331 (the `composition` item with `premises` and
  `assumptions`, the `interface-mismatch` counterexample and the
  `provider-answer-unconfirmed` core-guard cause).
- Remaining work: QSL-2739 (the registered provider path these criteria
  describe). Source/package parity (AC-10) stays with the package-consumer API
  lane.
