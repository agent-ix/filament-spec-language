---
id: FR-897
title: "Carry a protocol channel over a systems connection between Port roles"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-880
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-738
    type: implements
  - target: ix://agent-ix/filament-specification/FR-465
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-152
    type: depends_on
---
# FR-897: Carry a protocol channel over a systems connection between Port roles

## Description

A protocol role is bound to an endpoint of a systems model by binding it to
a Port: `role Outlet on M::pump_out;` (FR-218, FR-383-AC-62, FSpec FR-465
"Member"). A Connection (FSpec FR-152), which is a relationship whose two
ends are Ports, is not a participant, so no role is bound to it. FSpec
FR-465-AC-12 refuses a role on a connection, and that refusal stands. The
connection is what a channel between two Port roles travels over. FSL SHALL
admit and check that binding:

- **Relationship member.** S3 SHALL admit `relationship Link = M::link;`
  whose export is a Connection. The member is bound to the connection's
  declaration key, its two Port ends in declaration order (source, then
  target) and its declared `direction`. It SHALL NOT be refused as not yet
  implemented.
- **`related by` over a connection.** A Connection relates Ports, which are
  declarations, not values that a record field can hold. S3 SHALL refuse a
  `related by Link (a, b)` clause whose relationship is a connection with
  `ill_typed`/`operator-ineligible` at `Link`, retaining `relationship` and
  the message `a connection relates ports, not values; bind it to a channel
  with via`.
- **Channel `via`.** The channel production gains an optional `via
  <relationship>` after its `to` role (FSpec FR-738): `channel Feed from
  Outlet to Inlet via Link carries Flow ...;`. S3 SHALL check, in this order,
  each refusal at the name or type it names:

  | Check | Refusal |
  | --- | --- |
  | `Link` names a relationship member of the protocol | `missing_declaration`/`missing-name` at `Link` |
  | That member's export is a Connection | `invalid_model_binding`/`wrong-export` at `Link`, required `connection`, retaining the actual kind |
  | `from` and `to` are each roles on a Port | `invalid_model_binding`/`malformed-declaration` at the role name, retaining `role` |
  | The two roles' Ports are the connection's ends, the flow source sending: for `source-to-target`, `from` on the source Port and `to` on the target Port; for `target-to-source`, the reverse; for `bidirectional`, either assignment | `invalid_model_binding`/`malformed-declaration` at `Link`, retaining `expected` (the ports in flow order) and `actual` (the roles' ports) |
  | The `carries` type conforms (FSpec FR-151) to the effective type of the flow-source Port's interface type | `ill_typed`/`type-mismatch` at the `carries` type, retaining `expected` and `actual` |

- **Emission.** S4 SHALL write the channel's `connection` argument (FSpec
  FR-738) as a local reference to the model relationship node that the
  clause's `relationships` set already holds (FR-880), and SHALL list it in
  the channel's dependencies. A channel without `via` has no `connection`
  argument and is written as before.
- **Semantics.** `via` adds no behaviour. The channel's ordering, delivery,
  capacity and overflow premises are its own, and the protocol system steps
  the channel as before. The binding is structural evidence: a consumer of
  the package can see which connection each message travels over.

### Decision: a role binds to an endpoint, and a channel binds to a connection

A connection has two ends and no single participant, so binding a role to
it would leave it unclear which end the role acts at. The standard already
binds a role to a Port and refuses a role on a connection for that reason.
What a systems author means by "this protocol runs over that connection" is
that the messages between the two endpoint roles travel over it. That is a
property of a channel, which also has two ends, a direction and a message
type. Binding the channel to the connection lets S3 check the parts that can
go wrong: the wrong ports, the wrong direction, or a message type the ports
cannot carry.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-897-AC-1 | Over FR-383-AC-62's model (`Sys` owning `pump_out` (out) and `tank_in` (in), connection `link` from `pump_out` to `tank_in`, source-to-target), `relationship Link = M::link;` with `channel Feed from Outlet to Inlet via Link carries Flow ...` checks and compiles. The channel node's `connection` argument references `link`'s relationship node, which its dependencies list, and the package is admitted. | Test (TC-2533) |
| FR-897-AC-2 | `via Missing` refuses `missing_declaration`/`missing-name` at `Missing`. `via Pipe`, where `Pipe` is a navigation relationship, refuses `invalid_model_binding`/`wrong-export` at `Pipe`, required `connection`. | Test (TC-2533) |
| FR-897-AC-3 | `channel Back from Inlet to Outlet via Link` (the reverse of a source-to-target connection) refuses `invalid_model_binding`/`malformed-declaration` at `Link`, retaining the expected and actual ports. A `from` role on the component `Sys` refuses at that role's name. | Test (TC-2533) |
| FR-897-AC-4 | `carries Note`, where `Note` does not conform to `pump_out`'s interface type, refuses `ill_typed`/`type-mismatch` at `Note`, retaining expected and actual. | Test (TC-2533) |
| FR-897-AC-5 | `related by Link (a, b)` on a `send` refuses `ill_typed`/`operator-ineligible` at `Link` with the message naming `via`. `role R on M::link` still refuses as FSpec FR-465-AC-12 states. | Test (TC-2533) |

## Dependencies

- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md): roles,
  relationships and channels at S3.
- [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md) and
  [FR-880](FR-880-write-every-checked-protocol-construct-at-s4.md): the
  written nodes.
- FSpec FR-152 (Connection), FR-465 (channel and role nodes) and FR-738
  (the `connection` argument and the grammar).
