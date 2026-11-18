---
id: FR-1033
title: "Select a protocol subject and its memory models in a native-analyze/1 request"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-625
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-629
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-219
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-224
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-225
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-520
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-425
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-434
    type: depends_on
---
# FR-1033: Select a protocol subject and its memory models in a native-analyze/1 request

## Description

An analyze item decides a claim over a subject. FR-620 gives the request a
default model subject, and FR-205 defines the protocol subject the protocol
engines check: a model subject together with one protocol clause, its `over`
binding, its static role bindings, its capacity binding and the resolved
memory model of each outermost `parallel`. FSL SHALL read a protocol subject
from a `native-analyze/1` request, through the `protocol` member of the
request's `subject` or of an item's `temporal.subject`, and SHALL read the
request's `memory` member as FR-219's selection of the memory model of each
outermost `parallel`, so that a claim can be decided over a protocol under
`sc`, `tso` or `ra`, and a race-freedom item can be requested, from a file.

### Decision: the protocol binding is part of the subject, the memory selection stays request-level

A protocol is not a claim: a temporal clause such as
`always holds(not (core.r1 = 0 and core.r2 = 0))` reads the model state, and
the same clause can be checked over the model's own steps or over the steps
of a protocol that runs the model's operations. The protocol therefore
belongs to the subject, as FR-205 and FSpec FR-425 define it, and is written
where the subject is written: in the request's default `subject`, or per item
in `temporal.subject`. Its spelling is FSpec FR-433's `protocol-subject`
spelling without `type`, the spelling a protocol counterexample already uses,
so a subject read back from a counterexample can be requested again
unchanged. The `memory` member stays at request level, as FSpec FR-520 and
FR-434 place it, because a scoped `parallel` path names its protocol, so one
selection list serves every item over every protocol of the package, and
FR-219 already states its resolution and refusals.

## Inputs

- `subject.protocol` and `items[i].temporal.subject.protocol`, each optional:
  `{"protocol": "<qualified name>", "over": <over>, "roles": {"<role>": <binding>}, "capacities": {"<name>": "<decimal>"}}`.
  `protocol` is required and names a protocol clause of the checked package.
  `over` is the clause-run request's `over` spelling,
  `{"reference": {"population": "<population identity>", "key": "<object key>"}}`;
  it is required: the subject binds the clause's `over` parameter to exactly
  one object of its population universe (FR-205, FSpec FR-425, FR-433).
  `roles` (default empty) binds each static role on a model type to
  `{"type": "model-binding", "selection": "<domain package identity>"}`;
  a role on a whole model is bound by construction and takes no entry.
  `capacities` (default empty) is FSpec FR-785's capacity binding.
- `memory`: `[{"parallel": "<scoped path>", "model": "sc" | "tso" | "ra"}]`
  (FSpec FR-434, FR-520), the scoped path of an outermost `parallel`
  (FR-112), its first segment the protocol's name.
- A race-freedom item: `{"claim": "<scoped path of a parallel>", "temporal": {"item_kind": "race-freedom"}}` (FR-629).
- `temporal.limits.max_store_buffer` and `temporal.limits.max_messages` of an item (FR-629 places them in `temporal.limits`): FR-225's
  method bounds, each a positive count, 4 when absent.

## Outputs

FR-621's outcome document. An item over a protocol subject echoes its
`temporal.subject` with the `protocol` member and every default made explicit
(the role bindings, and each outermost `parallel`'s effective model in the
item's FSpec FR-331 `memory`), so it can be re-requested exactly.

## Behavior

### Selecting the subject of an item

- An item's subject SHALL be its `temporal.subject` when present, else the
  request's `subject`, else FR-620's default subject.
- When that subject carries `protocol`, FSL SHALL decide the item over the
  protocol subject FR-205's subject builder builds from the model subject and
  the binding, with each outermost `parallel` of the protocol resolved by
  FR-219 from the request's `memory` member; otherwise FSL SHALL decide it over
  the model subject, as FR-625 states.
- A race-freedom item whose `claim` is the scoped path of an outermost
  `parallel` SHALL be decided over the protocol its path names. When the
  item's subject carries no `protocol`, the item SHALL settle `declined`
  `invalid-input` with `invalid_runtime_input`/`missing-member` naming
  `temporal.subject.protocol`; when it carries a `protocol` naming another
  clause, the item SHALL settle `invalid-request`
  with `invalid_capability`/`unknown-claim` naming the path.
- The derived race-freedom item of FR-224 SHALL be added for each distinct
  protocol subject among the request's items whose resolved model is weak and
  that has a non-atomic shared access, after the caller's items, as FR-620
  places derived items.
- Two items whose subjects differ only in the protocol binding or in an
  effective memory model SHALL carry different request identities (FR-219,
  FSpec FR-331-AC-48).

### Reading `memory`

- The analyze reader SHALL read the `memory` member and hand it to the
  subject builder; it SHALL NOT refuse a request for carrying `memory`.
- Each entry SHALL be a closed object with exactly `parallel` and `model`.
  A `model` other than `sc`, `tso` and `ra`, a missing or unknown member, or a
  member of the wrong JSON type SHALL refuse the whole request at the request
  stage, category `invalid-request`, exit 20, naming the member by its
  pointer (`/memory/<i>/model`).
- An entry whose `parallel` names no outermost `parallel` of a protocol
  clause of the package (a nested `parallel`, a control that is not a
  `parallel`, a protocol the package does not declare), and two entries naming
  one `parallel`, SHALL refuse the whole request at the request stage,
  category `invalid-request`, code `invalid_runtime_input`/`invalid-value`,
  exit 20, naming the entry, or both entries (FR-219, FR-629-AC-5), before any
  item runs.
- An entry naming an outermost `parallel` of a protocol no item is decided
  over is admitted and changes no item's identity.

### Refusing a protocol binding

- A protocol binding the subject builder refuses (FR-205: a protocol the
  package does not declare, `invalid_runtime_input`/`invalid-value`; a role
  binding missing, extra or naming a model the role does not admit,
  `wrong-role-mapping`; an `over` object outside its population universe,
  `invalid-value`; a capacity entry naming no capacity, `unknown-member`, or
  repeated, `duplicate-member`) SHALL settle each item over that subject
  `declined` `invalid-input` with that code, naming the member, as FR-625
  settles a refused subject admission; the request's other items run.
- A `protocol` member with no `over` SHALL settle the item the same way with `invalid_runtime_input`/`missing-member`
  naming `over`.
- An unknown member of the binding SHALL refuse the whole request at the
  request stage, `invalid-request`, exit 20, naming it (FR-620 closure).

## Acceptance Criteria

The `SB` fixture is ADR-025 §10's store-buffering protocol (roles `w on M::Cell` and `c on M::Core`, `over (k: M::Core)`) over the litmus
domain (`crate::litmus`), with the claim `NoBothZero` that the two reads never
both return 0.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-1033-AC-1 | Through `fsl_command::command::analyze`, a request whose `subject.protocol` is `{"protocol": "SB", "over": O, "roles": {"w": B, "c": B}}`, `B` the binding `{"type": "model-binding", "selection": "test/litmus"}` and `O` the `over` binding `{"reference": {"population": "ix://test/litmus/cores", "key": "core"}}`, with the item `{"claim": "NoBothZero"}` and no `memory` settles `proved`; the same request with `memory` `[{"parallel": "SB::Both", "model": "tso"}]` settles `refuted` with a counterexample carrying both pending stores and `replay.reproduced` `true`, exit 10; each item's echoed `temporal.subject` carries `protocol` `SB` and its effective model, and the two request identities differ. | Test (TC-2959) |
| FR-1033-AC-2 | Over a variant of `SB` whose branches each make an unsynchronised non-atomic write to `x`, under `tso`, with AC-1's `subject.protocol` over the variant, the item `{"claim": "SB::Both", "temporal": {"item_kind": "race-freedom"}}` settles `refuted` with a `Race` counterexample naming both writes; the same request with no `items` and that `subject.protocol` holds the derived race-freedom item after the claim items; the item with no `subject.protocol` settles `declined` `invalid-input` with `invalid_runtime_input`/`missing-member` naming `temporal.subject.protocol`; the same item with `subject.protocol` naming another clause settles `invalid-request` with `invalid_capability`/`unknown-claim` naming `SB::Both`. | Test (TC-2959) |
| FR-1033-AC-3 | The item `NoBothZero` with no `subject.protocol` is decided over the model subject, as FR-625 states, and its record names no memory bound. | Test (TC-2959) |
| FR-1033-AC-4 | A `memory` entry with `model` `pso` refuses the whole request `invalid-request`, exit 20, naming `/memory/0/model`; entries naming `SB::Both::left::L::Inner` (nested), `SB::Both::Sx` (an attempt) and `Nope::Both` (no such protocol), and two entries naming `SB::Both`, each refuse the whole request `invalid-request` with `invalid_runtime_input`/`invalid-value` naming the entry, or both entries, exit 20, with no item run. | Test (TC-2959) |
| FR-1033-AC-5 | `subject.protocol` `{"protocol": "Missing"}` settles the item `declined` `invalid-input` with `invalid_runtime_input`/`invalid-value` naming `Missing`; a `roles` entry for a role on a whole model settles it `declined` with `invalid_runtime_input`/`wrong-role-mapping` naming the role; an unknown member `extra` of the binding refuses the whole request `invalid-request`, exit 20, naming it. | Test (TC-2959) |
| FR-1033-AC-6 | FR-225-AC-4's three-store protocol requested through `analyze` under `memory` `tso` with `temporal.limits.max_store_buffer` 2 and `reductions` naming the constraint `Zero` settles `inconclusive` `ConstraintReached{boundary_states: 8}` with no counterexample, `exploration.states` 20, and its record states `max_store_buffer` 2 as reached; without `Zero` it settles `MemoryBoundReached` naming `max_store_buffer` 2. | Test (TC-2959) |

## Dependencies

- FR-205 and FSpec FR-425 own the protocol subject and its binding refusals;
  FR-219 and FSpec FR-434 the memory selection; FR-224 and FR-629 the
  race-freedom item; FR-225 the memory bounds.
- FSpec FR-520 owns the request document's members, FSpec FR-433 the
  protocol-subject spelling.
