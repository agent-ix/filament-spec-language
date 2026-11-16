---
id: FR-880
title: "Write every checked protocol construct at S4"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-464
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-466
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-726
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-727
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-728
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-729
    type: depends_on
---
# FR-880: Write every checked protocol construct at S4

## Description

The S4 package emitter (FR-383) SHALL write every protocol that S3 checks
(FR-218, FR-381) as `filament.checked-semantic-graph/v2` protocol nodes, and
FSL's I2 reader SHALL read them back. S4 SHALL NOT refuse a checked protocol
as `unsupported_construct`/`not-yet-implemented`. Each construct below is
written in the form the standard defines for it:

- **Run controls.** Every `run` control at every depth is written as its
  FSpec FR-466 control node: `sequence`, `parallel`, `choice` and its cases,
  `repeat`, `await`, `attempt`, `send`, `receive`, `check`, `effect`,
  `event`, `commit` and `fence`.
- **Relationships.** Each checked relationship member is written into the
  clause's `relationships` set as its model relationship node, once, in
  ascending node-id order (FSpec FR-464 "Clause body").
- **Protocol references in a protocol body.** A `protocol Name` reference in
  a `check` or `send` constraint, an `each` guard, a `repeat` or `choice`
  guard, a `terminal when` condition, a capture initializer or a `finish`
  constraint is written as FSpec FR-322's `protocol_reference` carrier. The
  reader's binder closure stops at the carrier (FSpec FR-728).
- **References to a protocol that holds an `attempt`.** S4 writes nodes in
  dependency order. A protocol's node is written after the state clauses
  its attempts list. A function or clause that refers to that protocol is
  written after the protocol. The protocol's node key is the same whether or
  not anything refers to it.
- **Reference cycles.** A cyclic component of function calls and protocol
  references is written with FSpec FR-322's recursion-group encoding, as
  FSpec FR-464-AC-15 defines. This covers a protocol that refers to itself
  and a predicate-join function that refers to its own protocol. Every
  member of the component, the closing `protocol_reference` carrier
  included, shares the one `recursion_group`. FR-092 assigns the ordinals,
  and members are written in ordinal order. A requirement cycle never
  reaches S4, because S3 refuses it (FR-218-AC-25).
- **`terminal when` reading `over` under `activation on each`.** The
  condition is written as the `terminal` node. FSpec FR-464's stage table
  lets the `terminal` stage read `over` under either activation.
- **`over` inputs that are not object references.** The clause is written
  with the `owner_model` edge that FSpec FR-727 assigns: the element object
  type of an `Option` or collection of references, or no edge when the
  `over` type reaches no object type.
- **`repeat` forms.**
  - A post-test `repeat` (FR-218-AC-33: its guard reads a binder its body
    declares) is written with FSpec FR-729's `test` `post` member.
  - A `repeat` whose body steps through a nested `repeat` is written by the
    FSpec FR-464 "Nested `repeat` rule".
- **Channels.**
  - A channel that carries a bounded scalar or bounded collection is written
    with its `bounded_domain` node as `carries`.
  - A FIFO channel is written with its key whenever the key type is
    FIFO-key-stable (FSpec FR-726). This includes a bounded integer such as a
    model value type `PartId`, a quantity, and a model-owned record or variant
    whose leaves are FIFO-key-stable.

A pre-test `repeat` (its guard reads no body binder) whose `visible` entry,
`invariant` or `variant` reads a binder its body declares is written without
`test`, those reads being FSpec FR-729 back-edge reads. Each reads the value
from the most recently completed body iteration (FR-218-AC-18). At the
repeat's entry, before any iteration has completed, such a `visible` entry is
no decision fact and such an `invariant` or `variant` is not evaluated.

### Decision: S4 writes what S3 checks, in the standard's form

A construct that passes S3 and is then refused at S4 is one the user can
write and can never compile. Each construct listed above has a
checked-package form in the standard, so S4 writes it. Where a package
reader in another component does not yet admit the standard's form, the
reader is out of conformance, and S4 SHALL NOT refuse the construct to work
around that reader. The same rule applies to a role on a whole model
(FR-383). A construct the standard forbids is refused at S3, with a named
cause and a locus on the construct. An example is a FIFO key that is not
FIFO-key-stable (FR-881).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-880-AC-1 | A protocol whose `run` holds, at a depth of at least two, each of `parallel` (with each join policy), `choice`, `repeat`, `await`, `attempt`, `send`, `receive`, `check`, `effect`, `event`, `commit` and `fence` compiles, writes one FSpec FR-466 control node per control, and is admitted by FSL's I2 reader; no refusal names a control keyword. | Test (TC-2500) |
| FR-880-AC-2 | `relationship Pipe = M::pipe;` over the architecture fixture compiles; the clause's `relationships` set holds `pipe`'s model relationship node once; a second relationship gives two entries in ascending node-id order; the package is admitted. | Test (TC-2500) |
| FR-880-AC-3 | A protocol `Flow` whose `repeat` guard, `check` constraint, `each` guard, `terminal when`, capture initializer and `finish` constraint each hold `protocol Other` compiles, each reference is one `protocol_reference` carrier targeting `Other`'s clause, and the package is admitted. | Test (TC-2501) |
| FR-880-AC-4 | A function and a predicate that refer to a protocol with an `attempt` compile. The protocol's node follows the state clauses its attempt lists, the referrers follow the protocol, and the protocol's node key equals the key it has with no referrer. | Test (TC-2501) |
| FR-880-AC-5 | A protocol whose `repeat` guard holds `protocol Flow` inside `Flow` itself, and a predicate-join function that refers to its own protocol, each compile into one recursion group. Every member of the cyclic component shares that group, members are written in FR-092 ordinal order, and the package is admitted. An acyclic reference in the same unit carries no `recursion_group`. | Test (TC-2501) |
| FR-880-AC-6 | `terminal when { present(deref(input).trail) }` under `on each (t: Tick)` compiles, and the `terminal` node's condition reads the `over` binder. | Test (TC-2502) |
| FR-880-AC-7 | Protocols over `Int[0, 3]`, `Boolean`, a model record `Parts::Handoff`, `Option<Integer>` and `Sequence<Integer>` each compile with no `owner_model` edge. Protocols over `Option<M::Order>` and `Sequence<M::Order>` compile with the edge to `M::Order`. Every package is admitted. | Test (TC-2502) |
| FR-880-AC-8 | `repeat Loop by Seller visible (got.ok) while { not got.ok }` over a body that binds `got` compiles with `test` `post`. A `repeat` that steps only through a nested `repeat` compiles. Both packages are admitted. A post-test `repeat` whose body has a cycle through no step is still refused at S3 by FR-218's progress rule. | Test (TC-2502) |
| FR-880-AC-9 | A pre-test `repeat` (guard `true`) whose `visible` entry reads the body binder `got`, and one whose `invariant` and `variant` read it, each compile without a `test` member and are admitted. The `visible` entry is an `expression` node over `got`. No refusal is produced at S3 or S4. | Test (TC-2502) |
| FR-880-AC-10 | Channels carrying `Int[0, 5]`, `Text[0, 8; nfc]`, a bounded `Decimal`, a bounded `Rational`, `Float64[nearest-even]` and `Sequence<Boolean>[0, 3]` compile with the `bounded_domain` node as `carries`. FIFO channels keyed by `Int[0, 99]`, a model value type `PartId` (a bounded integer), `Quantity<m>`, `Option<Int[0, 5]>`, a record with a bounded field and a model variant compile with their keys. Every package is admitted. | Test (TC-2503) |
| FR-880-AC-11 | Over a corpus holding one minimal protocol for each construct FR-218 checks, every protocol that S3 admits writes a package, and no refusal of a checked protocol carries `not-yet-implemented`. | Test (TC-2503) |

## Dependencies

- [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md): the
  protocol node family this requirement completes.
- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md): the S3
  check whose admitted protocols are written.
- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md): node keys and
  recursion-group ordinals.
- [FR-881](FR-881-refuse-a-fifo-key-that-is-not-fifo-key-stable-at-s3.md):
  the S3 key rule.
- FSpec FR-464, FR-466, FR-726, FR-727, FR-728 and FR-729: the node forms.
