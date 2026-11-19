---
id: FR-411
title: "Emit a checked model terminal declaration as a v2 terminal node"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-015
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
---
# FR-411: Emit a checked model terminal declaration as a v2 terminal node

## Description

The S4 package emitter (ADR-011 layer 4 `package`, edge E4) SHALL emit each
checked model `terminal` declaration (FR-124) as one
`filament.checked-semantic-graph/v2` node with `node_tag: "temporal"` and
`semantic_form: "terminal"`, whose body is the aggregate FSpec FR-476 states,
and FSL's I2 reader SHALL read that node back. FSpec FR-476 owns the body,
the declaration path, the identity rules and the reader's refusals; this
requirement states what FSL's emitter writes from its checked
`TerminalDeclaration`, what its I2 reader returns, and how FSL settlement
joins a deadlock-freedom verdict to the declaration it was decided under.

## Use case

A verification operator writes `terminal Done using v on Counter when {
self.value = 3 };` for a counter that stops at 3 and runs its deadlock-freedom
item. The package carries the declaration, so the model checker, the deadlock
classifier and replay of a deadlock counterexample all read the same intended
terminal states from the recompiled package. The item's obligation identity
is the same with and without the declaration, so FSL settlement, not the
identity, keeps a verdict decided under one declaration from settling the
item under another.

## Inputs

- A checked state model's `TerminalDeclaration::{When(clause), Any}` (FR-124),
  with its model alias and the span of its declaration.
- The node keys FR-092 mints and FR-093's lowering of a Boolean expression.

## Outputs

- One `temporal`/`terminal` node per distinct checked declaration on a
  model node (equal declarations on two aliases of one model identity share
  one node, one `claim` occurrence each), body as FSpec
  FR-476 states, with `semantic_type` the package's `scalar_type`/`boolean`
  node, no `declaration` (FSpec FR-476: the node is content-addressed and
  nested under its model by its `model` reference), and one occurrence of
  role `claim` at the declaration's span.
- For `When(clause)`: the clause's predicate node as FR-093 lowers it, an
  `expression` node holding the predicate's Boolean term or, for a constant
  predicate such as `true`, its Boolean `value`/`literal` node (FSpec FR-476),
  and the `self` parameter node when the predicate reads `self`.
- No node for `TerminalDeclaration::None`.
- From the I2 reader: the package's terminal declarations in FSpec FR-476
  terminal row order, each keyed by (model selection identity, ordinal), or
  FR-476's refusal with its locus.
- From FSL settlement: the deadlock-freedom item settled under the recompiled
  package's declaration, or `stale_dependency`/`content-mismatch` when the
  verdict was decided under another one.

## Behavior

- The emitter SHALL write `model` as a reference to the `model` node of the
  declaration's model alias.
- The emitter SHALL write `mode` as `when` for `When(clause)` and `any` for
  `Any`.
- The emitter SHALL write `parameters` as an aggregate holding one
  reference, to the `self` parameter node at level 0 typed `Reference<M::T>`
  as FR-124 types it, when the predicate reads `self`, and as the empty
  aggregate otherwise and for `Any`.
- The emitter SHALL write `condition` as a reference to the predicate's
  node for `When(clause)`, the `expression` node or, for a constant
  predicate, the `value`/`literal` node FR-093 lowers it to, and as the empty
  aggregate for `Any`. A constant predicate reads no `self`, so its
  `parameters` is empty.
- When two model aliases select one model identity and each carries a
  declaration, the emitter SHALL write both against the identity's one model
  node: two terminal nodes when the declarations differ, or one node with
  two `claim` occurrences (ordinals in source order, each with its
  `source_map` entry) when they are equal (FSpec FR-476 "Several terminal
  nodes on one model node"). It SHALL NOT drop, merge or refuse either.
- The emitter SHALL key the node by FR-322's rules over its body, so the mode, the model and the predicate enter the node id
  and the written name `T` does not.
- S4 SHALL NOT refuse a checked `terminal` declaration as not emitted.
- The request carries no separate terminal member.
- Replay of a deadlock-freedom counterexample (FR-128) SHALL read the
  model's `TerminalDeclaration` from the recompiled package's
  `temporal`/`terminal` node.
  Presence and mode are read from the checked graph's own `temporal`/`terminal`
  nodes with no emission; a `when` predicate's body is the checked body the
  node's `claim` occurrence names, since a checked body cannot be rebuilt from
  the wire. A declaration without its node, or a node without its declaration,
  is a disagreement and is refused, never read from the declaration alone.
- The I2 reader SHALL admit every emitted terminal node, several on one
  model node included, and return the package's declarations in FSpec FR-476
  terminal row order with their terminal keys (selection identity, ordinal),
  and no declaration for a model with no terminal node
  (`TerminalDeclaration::None`). A terminal's model is named by its
  selection identity on both routes (FR-940); a source alias is a display
  label and is never compared.
- The I2 reader SHALL refuse a node that breaks an FR-476 rule with FR-476's
  code at FR-476's locus, in FR-476's reader order.

### Deadlock-freedom verdict join

The deadlock-freedom item's obligation identity is the ADR-013 O-09 3-tuple
(node key, model identity, ordinal); its kind `deadlock-freedom`, its
arguments and their domains are record fields. A model's `terminal`
declaration is in neither, so two packages that differ only in the
declaration give the item equal identities and equal record fields.

- FSL settlement SHALL own the join of a deadlock-freedom verdict over a
  model subject to the request item it answers: the FR-281 derived-item
  adapters of EN-1 (`derived_model_item`) and EN-6 (`derived_zone_item`),
  and the FR-128 replay refusal those adapters wrap around the
  counterexample replay. The FR-127 and FR-235 settlement maps take no part
  in it: they settle what the adapter hands them. CG retains only opaque FSL
  bytes (ADR-013) and SHALL NOT be relied on for this join.
- FR-281's public `model_item`, `protocol_item` and `zone_item` SHALL refuse
  a derived item's run (deadlock, race or time-lock freedom) with
  `DerivedRun`, so a derived item settles only through its derived adapter.
- The derived adapter SHALL compare the obligation identity the caller
  supplies with the request item it answers: the obligation's kind with the
  request item's `item_kind` and with the item the run and its certificate
  check decide; for EN-1, the obligation's subject identity (FR-336) with
  the run's and the check's subjects; and the request item's claim, the
  `model_root` anchor that is the node of the 3-tuple (FSpec FR-368), with
  the `model_root` nodes of the packages the run and the check are over. A
  derived item has one item per kind and subject, so the model identity
  and the ordinal follow from the anchor and the kind. It has no arguments
  and no domains (FSpec FR-368: unbounded, no extent bounds), so no further
  record field is compared.
- The request writer SHALL write, on a model subject's derived
  deadlock-freedom request item, the record field `terminals`: for each model
  selection identity of the package, in ascending identity order, the list
  of its terminal node keys in FSpec FR-476 terminal row order (position in
  the list is the terminal ordinal), or none for a model with no
  declaration. The node key carries the mode, the model and the predicate.
  Every declaration counts, since the deadlock classifier reads every
  declaration of the package; two aliases of one identity contribute their
  declarations to that identity's one list.
- The derived adapter SHALL compare `terminals` with the terminal node keys
  of the package the run is over and of the package its certificate check
  is over, on EN-1 and on EN-6 alike. EN-6's run counts on its own: a
  symmetry-reduced deadlock-freedom run settles `Proved{Reduced,
  Uncertified}` with no certificate check (FR-160).
- If the kind differs, or on EN-1 the subject differs, then the adapter
  SHALL refuse `OtherItem` (EN-6 compares no subject identity); if no
  `model_root` of the run's or the check's package is the request item's
  anchor, it SHALL refuse
  `stale_dependency`/`content-mismatch` (`OtherAnchor`) naming the anchor;
  if the run's or the check's declaration differs, it SHALL refuse
  `stale_dependency`/`content-mismatch` (`TerminalDeclaration`) naming the
  first differing model selection identity and its two lists of terminal
  node keys (or `none`); the alias the source used for that model is a
  display label in the message only. Each
  refusal builds no item, so no verdict is settled.
- If the package the counterexample replay recompiled declares other
  terminal states than `terminals`, then the replay SHALL refuse
  `stale_dependency`/`content-mismatch` the same way, so the item settles
  `inconclusive` with cause `replay_refused`, never `refuted`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-411-AC-1 | Compiling FR-126-AC-9's `Counter` unit with `terminal Done using v on Counter when { self.value = 3 };` emits one `temporal`/`terminal` node with `mode` `when`, `parameters` an aggregate holding one reference to the `self` parameter node, `condition` a reference to the `expression` node of `self.value = 3`, no `declaration` and one `claim` occurrence at the declaration's span; with `terminal Halt using v on Counter any;` it emits `mode` `any` with empty `parameters` and `condition`; without a declaration it emits no terminal node. The I2 reader returns `When`, `Any` and `None` respectively. No checked declaration is refused as not emitted. | Test (TC-1142) |
| FR-411-AC-2 | AC-1's `when` and `any` terminal nodes have different node ids; renaming `Done` to `Stop` keeps the node id. | Test (TC-1142) |
| FR-411-AC-3 | **RETIRED (staleness check withdrawn).** There is no `PackageIdMismatch` refusal for a replay over an edited terminal declaration. | Test (TC-1142), retired |
| FR-411-AC-4 | Fed to FSL's I2 reader, with `dependencies` set to the body's targets, AC-1's `when` node with `mode` `some` refuses `invalid_package`/`invalid-value`, and with `condition` the empty aggregate refuses `invalid_semantic_graph`, each at FR-476's locus. | Test (TC-1142) |
| FR-411-AC-5 | AC-1's `when` and no-declaration packages give the deadlock-freedom item equal obligation identities, and the `when` package and one declaring `terminal Done using v on Counter when { self.value = 2 };` have different terminal node keys. For a request item written over the `when` package, an EN-1 run and certificate check over the no-declaration package refuse `stale_dependency`/`content-mismatch` naming the selection identity of the model that the alias `Counter` selects, the `when` terminal node key and `none` (the alias `Counter` appears only as a display label, and renaming it leaves the refusal's named identity and keys unchanged); so does a certificate check alone over it, a run alone over it, and a run and check over the `value = 2` package (naming both keys); a `when` run for an item written over the no-declaration package refuses the same way. The run over the `when` package settles `proved`, and over the no-declaration package for its own item `refuted`. That refutation, replayed against the `value = 2` package, under which it reproduces, settles `inconclusive` with cause `replay_refused`. | Test (TC-1142) |
| FR-411-AC-6 | `model_item` given the Counter's deadlock-freedom run, `protocol_item` given a protocol subject's deadlock-freedom or race-freedom run, and `zone_item` given a timed deadlock-freedom run each refuse `DerivedRun`. `derived_model_item` given the dense `Rpc`'s derived deadlock-freedom item with the Counter's obligation and run refuses `OtherAnchor` naming the item's anchor. Over FR-430-AC-3's latching timers, for the timed deadlock-freedom item written over the package with no declaration, a symmetry-reduced EN-6 run over the package declaring `terminal Done using v on M when { true };` refuses `TerminalDeclaration` naming `none` and that package's key, though the certificate request is over the written package, and so does a certificate request alone over the `when` package; the same reduced run for the item written over the `when` package settles `Proved{Reduced}`; and `derived_zone_item` given the timers' derived deadlock-freedom item with a run and certificate request over a package of another model refuses `OtherAnchor`. | Test (TC-1142) |
| FR-411-AC-7 | Compiling AC-1's unit with `terminal Done using v on Counter when { true };` emits a `when` node whose `condition` references the Boolean `value`/`literal` node of `true`, with empty `parameters`; FSL's I2 reader and the Contract IR reader both admit the package (no `ill_typed`/`operator-ineligible`), the I2 read returns `When`, and on the package route the deadlock-freedom item over the Counter settles `proved`, as it does on the source route; with `when { false }` both routes settle it `refuted` at the state `value = 3`. | Test (TC-1142) |
| FR-411-AC-8 | A unit with `model M = "test/vault"; model N = "test/vault";`, `terminal Done using v on M when { self.l = 1 };` and `terminal Stop using v on N when { false };` checks, emits two terminal nodes whose `model` binding is the one `test/vault` `model_root` node, and emit-then-admit succeeds with no `ambiguous-name`; the I2 read returns both, keyed (`test/vault`, 0) and (`test/vault`, 1) in terminal node key order, and the source route keys them the same. With `terminal Halt using v on M any;` and `terminal End using v on N any;` it emits one node with two `claim` occurrences and two `source_map` entries. For `Done` and `Stop`, the deadlock-freedom item's `terminals` field holds one entry, `test/vault` with the list of both node keys, and the item settles `proved` on both routes. A run over the same unit with `Stop` removed, for the item written with both, refuses `TerminalDeclaration` naming `test/vault` and the two lists; swapping the two `model` lines or the two declarations changes neither the package nor `terminals`. | Test (TC-1142) |

## Dependencies

- ADR-011 §6.1 layer 4 `package` and §2.1 E4; ADR-018 DL-1 to DL-3.
- [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (the checked declaration and the derived item),
  [FR-382](FR-382-build-the-terminal-declaration-form-at-s2.md) (the S2
  form), [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  (lowering the predicate), FR-092 (node keys),
  [FR-128](FR-128-replay-a-model-counterexample.md) (deadlock counterexample
  replay).
- ADR-013 O-09 (the obligation identity 3-tuple and its record fields),
  [FR-281](FR-281-analyze-claims-with-in-process-engines.md) (the
  derived-item adapters and the public item adapters).

## References

- FSpec FR-476 (checked-package model terminal declaration body), FR-366
  (the declaration and deadlock freedom), FR-322.
- Owning ticket: Linear STD-303.
