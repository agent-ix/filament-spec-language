---
id: FR-960
title: "Name the backend of a run call's requirement items"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-395
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-651
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-962
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-967
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-775
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
---
# FR-960: Name the backend of a run call's requirement items

## Description

A native-run/1 `call` (FR-100) SHALL carry an optional member
`requirements`. Its presence asks the run to settle the requirement items of
the call: the items whose owner is the called function (FR-395 "Item of the
call"), routed, negotiated and settled as FR-961 states. The member also
names, for any of those items, the backend that decides it. A call without
`requirements` settles no item, and its run is exactly FR-100's.

The run's requirement items are routed by the same rule as analyze's request
items (FR-635 "Request item", FR-075), so one item names a backend the same
way whichever verb carries it: the `BackendId` it names selects that backend
among the item's candidates, and the request names no binary or path, because
which providers exist is the invocation's configuration and not the request's.
A call-level default exists because a function usually has several items and
the caller usually wants them all on one provider; a per-item entry exists
because a caller may need one item on another backend. The per-item entry
names its item by the item's FSpec FR-331 claim `{node, occurrence}`, the
identity the run outcome reports for it (FR-963), so a caller can copy an
item's identity from one outcome into the next request.

## Inputs

`call.requirements`, an optional closed object with these optional members:

- `backend`: a `BackendId` string (FR-288's spelling). It names the backend of
  every item of the call that has no entry in `items`, or whose entry names no
  `backend`.
- `options`: a map of the named backend's option names to string values
  (FSpec FR-196). It applies with `backend` and is accepted only beside it.
- `items`: an array of closed objects, each `{claim, backend?, options?}`:
  - `claim`: `{node, occurrence}` in FSpec FR-331's claim spelling, naming one
    item of the call;
  - `backend` and `options`: as above, for that item alone, in place of the
    call-level members.

`requirements: {}` settles every item of the call with no named backend.

An FR-651 `call` selection is FR-100's call object, so it carries the same
member, and each item that selection yields settles its own call's
requirement items.

## Outputs

For each item of the call, its named backend (absent when neither its entry
nor the call names one) and its options, which FR-961 writes into the item's
FSpec FR-331 request item (`backend`, `options`) before candidates are
computed.

## Behavior

- **Named backend.** An item whose named `BackendId` the run's registry holds
  SHALL have the candidate set FR-075 gives a named backend: that backend when
  it matches the item, otherwise empty, which settles `unsupported`
  `unsupported_projection`/`unsupported-requested-capability` naming the
  backend. An item naming a `BackendId` the registry does not hold SHALL carry
  FR-075's unknown-backend marker and settle `invalid-request`
  `invalid_capability`/`unknown-backend` naming it, and no provider SHALL be
  called for it.
- **Options.** An option name the named backend's manifest does not publish
  SHALL settle that item `invalid-request` `invalid_capability`/`unknown-option`
  with `request_index`, `backend` and `option` (FR-967, FSpec FR-779), as for
  analyze, and settle no other item that way; the call's value is unchanged.
- **Unnamed items.** An item with no named backend SHALL have every registered
  matching backend as its candidates, narrowed by FR-962's tie rule.
- **Precedence.** An item's own `items` entry SHALL take precedence over the
  call-level `backend` and `options`; an entry that names no `backend` takes
  the call-level ones.
- **Refusals before the call.** These SHALL refuse the request at stage
  `request` with `invalid-request` naming the member's JSON pointer, exit 20,
  as FR-100's row for a `call` that is not its closed object: `requirements`
  that is not a closed object of the members above; a `backend` that is not a
  non-empty string spelling a `BackendId`; `options` without a `backend` in
  the same object, or with a value that is not a string; an `items` entry that
  is not a closed object with a `claim`; and two entries naming the same claim.
- **Unknown claim.** An `items` entry whose `claim` names no item of the call
  SHALL refuse the request at stage `call` with `missing_declaration` and
  `details` `{"claim": {"node": N, "occurrence": O}}`, exit 20, before any
  item is negotiated. A claim of another function's item is not an item of
  the call (FR-395), so it refuses the same way.
- **Identity.** The named backend and options are members of each item's
  FSpec FR-331 request identity, so the same call naming two backends is two
  requests.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-960-AC-1 | A run of `sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }` with `x = 3` and `call.requirements` `{}`, through `command::run` and FR-961's linked registry, returns the completed value `{"integer": "16"}` and one settled entry per item of `sq` (three, in FR-395's package order), each with no named backend and `value-checker` as its one candidate; the same call without `requirements` returns the same value and no `requirements` member. | Test (TC-2700) |
| FR-960-AC-2 | The same call with `call.requirements` `{"backend": "value-checker"}` gives each item request `backend` `value-checker`; with `{"backend": "fsl.absent"}` every item settles `invalid-request` `invalid_capability`/`unknown-backend` naming `fsl.absent`; with `{"backend": "fsl.absent", "items": [{"claim": C, "backend": "value-checker"}]}`, `C` the first item's claim, the first item settles on `value-checker`, its request naming `backend` `value-checker` and the other two settle `unknown-backend`. | Test (TC-2700) |
| FR-960-AC-3 | Negative: `requirements` `[]`, `{"backend": ""}`, `{"options": {"k": "v"}}`, `{"backend": "value-checker", "options": {"k": 1}}`, `{"extra": 1}`, an `items` entry without `claim`, and two entries naming one claim each refuse at stage `request` with `invalid-request` naming the member's JSON pointer, exit 20, before any source is compiled; an entry whose `claim` names no item of `sq` refuses at stage `call` with `missing_declaration` and `details` naming that claim, exit 20, and no item is negotiated. | Test (TC-2700) |
| FR-960-AC-4 | An option name `value-checker`'s manifest does not publish, given as `{"backend": "value-checker", "options": {"unknown": "1"}}`, settles each item `invalid-request` `invalid_capability`/`unknown-option` with its `request_index`, `backend` `value-checker` and `option` `unknown`, never `invalid_runtime_input`, calls no engine for it, and leaves the call's completed value unchanged. | Test (TC-2700) |

## Dependencies

- FR-100: the `call` object this member extends.
- FR-395: the items of the call and their identities.
- FR-075, FR-635: the named-backend and option rules shared with analyze.
- FR-961: the entry that settles the items; FR-962: unnamed items.
- FSpec FR-775: the standard's rule that an `execute` request names its items'
  backends; FSpec FR-331: the claim spelling and request identity.
