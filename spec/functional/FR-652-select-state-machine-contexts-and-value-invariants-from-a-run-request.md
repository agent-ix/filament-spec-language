---
id: FR-652
title: "Select state-machine contexts and value invariants from a run request"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-014
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-366
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-651
    type: depends_on
---
# FR-652: Select state-machine contexts and value invariants from a run request

## Description

FR-106 admits two observations that select no ordinary clause: `Context`,
the invariant of a state machine over one side of an invocation, and
`ValueInvariants`, the type-level invariants of every type over every value
an observation reaches (FR-366). The run request reader SHALL accept a
`context` selection and a `value_invariants` selection, so that every
observation FR-106 admits is reachable from a request file and not only from
a library call.

## Inputs

Two selection variants, admitted wherever FR-312's selection variants are
(a `clause` request's `selection`, except as stated for `value_invariants`,
and an FR-651 `items` selection):

- `{"context": {"name": N, "invocation": K, "side": S}}`: `N` the state
  machine invariant's qualified name, `K` a key of `observations` naming a
  `filament.state.invocation/v1` document, and `S` either `"pre"` or
  `"post"`. The member set is exactly `{name, invocation, side}`.
- `{"value_invariants": {"snapshot": K}}` or
  `{"value_invariants": {"snapshot": K, "invocation": K2}}`: `K` a key
  naming a `current` `filament.state.snapshot/v1` document and `K2` a key
  naming a `filament.state.invocation/v1` document. The member set is
  `{snapshot}` or `{snapshot, invocation}`.

## Outputs

- A `context` selection: the FR-109 report of its run, written as one
  FR-267 `native-run-result/2` clause-run document, as for a `clause`
  selection.
- A `value_invariants` selection: one FR-651 item per result of FR-366's
  `evaluate_value_invariants`, in the order FR-366 reports them (the FR-207
  key order of their value paths, then clause key). Each item carries, besides
  FR-651's members, `invariant`, the invariant clause's declaration key, and
  `path`, the evaluated value's runtime value path in FR-207's spelling; its
  `result` is the `native-run-result/2` clause-run document of that one
  evaluation.

## Behavior

- The reader SHALL construct FR-106 `Context { invocation, side }` for a
  `context` selection, with the document reference of `K`, and
  `ValueInvariants { snapshot, invocation }` for a `value_invariants`
  selection, with the references of `K` and, when given, `K2`. FR-106's
  checks for those selections then apply unchanged at stage `admit`,
  including its `wrong_snapshot`/`wrong-observation` refusal of a `Context`
  selection of anything but a state machine's invariant.
- A `context` selection whose `invocation` names a document that is not an
  invocation, a `side` other than `pre` and `post`, a `value_invariants`
  selection whose `snapshot` names an invocation document or whose
  `invocation` names a snapshot, and a member outside the sets above SHALL
  refuse `invalid-request` at stage `request`, naming the member, exit 20.
  A `snapshot` naming a `pre` or `post` snapshot is a document of the right
  format and reaches FR-106, which refuses it `wrong_snapshot`/
  `wrong-observation` at stage `admit`.
- The reader SHALL admit a `value_invariants` selection only as an `items`
  selection, because it yields one verdict per invariant and value
  (FR-651). Its admission is one FR-106 admission; an admission refusal is
  one item whose `result` is the refusal's clause-run document, with no
  `invariant` or `path`.
- A `context` selection's report is evaluated once, by FR-107's evaluation of
  the selected side's observation.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-652-AC-1 | Through `run`, over FR-106-AC-41's TC-1045 `Orders` fixture, a `clause` request selecting `{"context": {"name": "Consistent", "invocation": "i1", "side": "pre"}}` returns one `native-run-result/2` document reporting `success`, `truth: true`, exit 0, and with `side` `post` the same; each equals the report of the `ClauseRunRequest` built in process with FR-106 `Context` for that side. With FR-106-AC-46's `place` modifying only `placed` and a post context also changing `cancelled`, side `post` reports stage `admit`, `frame_violation`/`unauthorized-change`, exit 20. | Test (TC-1714) |
| FR-652-AC-2 | Each refuses `invalid-request` at stage `request`, exit 20, no result document, naming the member: a `context` whose `side` is `"mid"`; a `context` whose `invocation` names a snapshot document; a `context` carrying `self`; a `value_invariants` whose `snapshot` names an invocation document; a `value_invariants` as a `clause` request's `selection`. | Test (TC-1715) |
| FR-652-AC-3 | Through `run`, over FR-106-AC-48's TC-1051 K2 fixture, an `items` request with the selection `{"value_invariants": {"snapshot": "s1", "invocation": "i1"}}` returns one item per `evaluate_value_invariants` result in FR-366's order, each with `invariant` and `path`; the item for `Money`'s non-negative invariant at `i1`'s parameter `amount` (`amount_minor` `-7`) reports `violation`, and the document's `category` is `violation`, exit 10. The same selection with a `pre` snapshot as `snapshot` gives one item refused `wrong_snapshot`/`wrong-observation` at stage `admit`, with no `invariant` or `path`. | Test (TC-1716) |

## Dependencies

- FR-106: `Context`, `ValueInvariants` and their admission.
- FR-107 and FR-366: their evaluation.
- FR-109 and FR-312: the clause-run report and request reader.
- FR-651: the `items` request and its result document.
