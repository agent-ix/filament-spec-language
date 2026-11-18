---
id: FR-962
title: "Resolve an unnamed multi-candidate item by the production tie rule before negotiation"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-489
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-625
    type: references
  - target: ix://agent-ix/filament-specification/FR-776
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-290
    type: depends_on
---
# FR-962: Resolve an unnamed multi-candidate item by the production tie rule before negotiation

## Description

When an item names no backend and more than one registered backend matches
it, FSL SHALL apply FR-489's claim-form tie rule while it computes the item's
candidates, before the item reaches the negotiator. When a row of that rule
decides the item, the item's candidate set SHALL be the one provider the row
selects, so `negotiate_item` receives a single candidate and settles the item
through that candidate's arm. Such an item never settles
`invalid-request` `ambiguous-backend`.

`negotiate_item` never selects among several candidates (FSpec FR-290), and
the caller resolves a multi-candidate item. FR-489's rule is FSL's published,
deterministic resolution of the one tie its linked set has by design (EN-1
and `contract-checker` both advertise `operation-contract`). The rule reads
only the claim's form and whether a model subject is named, so the narrowed
set is a function of the item and the registry, as FSpec FR-776 requires of a
candidate set. It is applied in candidate computation rather than by writing
a named backend, because the request did not name one: the item's FSpec
FR-331 request item keeps `backend` absent and carries the narrowed set as
`candidates` `{type: "set", backends: [<selected>]}`.

The rule applies to every provider-routing entry (FR-635 "One rule for every
provider-routing entry"): `analyze`, `prove`, `monitor`, the run's requirement
items (FR-961), `replay` and `generate`.

## Inputs

- The item, its claim form, whether its request names a model subject, and
  whether it names a backend.
- FR-075's matching set over the run's registry.

## Outputs

The item's candidate set: FR-075's matching set, or the one provider a row of
FR-489's rule selects.

## Behavior

- For an unnamed `operation-contract` item whose claim is not an `invariant`
  state clause in a request naming a model subject, `en-1` SHALL be excluded
  from the matching set before any row is applied.
- The rule SHALL apply only to an item that names no backend and whose
  matching set has more than one member.
- A row SHALL decide the item exactly when the item's kind is the row's kind
  and the matching set is exactly the providers the rule ties: for
  `operation-contract`, `en-1` and `contract-checker`. The candidate set is then
  the provider the row selects for the claim's form: `en-1` for an `invariant`
  state clause in a request naming a model subject; `contract-checker` for an
  `invariant` state clause with no model subject, and for an operation's `pre`
  or `post` contract or a frame obligation.
- A matching set that no row decides SHALL stay whole: a set holding a third
  backend that also matches (such as a process provider advertising
  `operation-contract`), and a set of an item kind the rule has no row for
  (such as two backends advertising `value-validity`, one linked and one a
  process provider). Its item settles `invalid-request`
  `invalid_capability`/`ambiguous-backend` naming every candidate in candidate
  order, as FSpec FR-290 states, and the caller resolves it by naming one
  backend (FR-635 request item, FR-960 for a run call).
- An item that names a backend SHALL never be narrowed by the rule; FR-075's
  named-backend rule alone gives its set.
- The rule SHALL read no registration order, display name, caller preference
  or ambient state.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-962-AC-1 | Over `Engines::fsl()`, an unnamed `operation-contract` item for the `invariant` state clause `self.value <= 3` in a request naming a model subject reaches the negotiator with candidates exactly `[en-1]` and no named backend, settles `supported` on `en-1`, and its record names `backend` `en-1`; the same clause with no model subject, and an operation's `post` contract, reach it with exactly `[contract-checker]`; no such item settles `ambiguous-backend`. | Test (TC-2702) |
| FR-962-AC-2 | The FR-331 request item written for AC-1's first item has no `backend` member and `candidates` `{"type": "set", "backends": ["en-1"]}`; two runs registering the linked descriptors in two orders write equal request items. | Test (TC-2702) |
| FR-962-AC-3 | Negative: with a process provider advertising `operation-contract` registered beside `Engines::fsl()`, AC-1's first unnamed item has candidates `[contract-checker, en-1, <provider>]` in candidate order and settles `invalid-request` `invalid_capability`/`ambiguous-backend` naming all three; naming `en-1` settles it on `en-1`. | Test (TC-2702) |
| FR-962-AC-4 | Negative: with a process provider advertising `value-validity` `unbounded` registered, an unnamed run requirement item of `sq` whose matching set is `value-checker` and the provider settles `invalid-request` `ambiguous-backend` naming both; the same item with `call.requirements` `{"backend": "value-checker"}` settles on `value-checker`. | Test (TC-2702) |
| FR-962-AC-5 | Over a registry holding `en-1` and an SMT backend advertising `operation-contract` in `bounded` mode, each FR-146 per-step simulation record item of the `cas_refines_counter` refinement matches exactly `[smt]`; an unnamed `operation-contract` item for the `invariant` state clause `self.value <= 3` in a request naming a model subject still matches `en-1`. | Test (TC-2702) |

## Dependencies

- FR-489: the tie rule. FR-075: the matching set it narrows.
- FR-635, FR-961: the entries that apply it. FR-625: the invariant item it routes to EN-1.
- FSpec FR-290, FR-776: negotiation never selects; a producer's published claim-form rule may narrow the set it computes.
