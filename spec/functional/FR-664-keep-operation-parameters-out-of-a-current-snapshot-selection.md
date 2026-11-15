---
id: FR-664
title: "Keep operation parameters out of a current-snapshot selection"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-003
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-652
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: references
---
# FR-664: Keep operation parameters out of a current-snapshot selection

## Description

A clause-run selection names one observation (FR-106 `ClauseSelection`).
A `Current` selection observes the state between operation calls: no call
is in progress, so no operation parameter is bound, and a clause selected
over it reads only the snapshot, its anchor and `self`. Operation
parameters belong to the observation of a call: `PreCall` supplies the
pre state, `self` and `parameters`, and `Invocation` supplies them through
the invocation document. A `Current` selection therefore takes no
`parameters`. The request reader refuses a selection that supplies them
rather than reading and ignoring them, so a caller who meant a call
observation learns which selection form carries parameters.

This is the specified behavior of the selection wire, not a gap: every
clause that reads parameters (a precondition or postcondition) is reachable
through `PreCall` or `Invocation`, and a `Current` selection of such a
clause already refuses at FR-106 check 2.

## Behavior

- FR-312's `clause` selection names its observation by a key, and the
  selected document's `observation` role decides whether it is a `Current`
  or a `PreCall` selection. If a `clause` selection carries a `parameters`
  member, of any value, and its selected document declares the `current`
  role, then the request reader SHALL refuse `invalid-request` at stage
  `request`, exit 20, naming `parameters`, before any model or program file
  is read and before any compile.
- If a `context` or `value_invariants` selection (FR-652) carries a
  `parameters` member, then the request reader SHALL refuse it the same way,
  before any file is read, since both are closed records that bind no
  operation parameter.
- A `PreCall` selection SHALL keep its required `parameters` member
  (FR-106).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-664-AC-1 | FR-312's current-snapshot clause-run request over the ConfigVersion spine files, with its `Current` selection given `"parameters": {}`, and again with `"parameters": {"target": {"reference": "child"}}`, refuses `invalid-request` at stage `request`, exit 20, with a message naming `parameters`; with the model and program files of the request absent the refusal is the same. The request without the member runs. | Test (TC-1747) |
| FR-664-AC-2 | FR-652's `{"context": {...}}` and `{"value_invariants": {...}}` selections each given `"parameters": {}` refuse the same way, with every dependent file of the request absent. | Test (TC-1747) |

## Dependencies

- [FR-106](FR-106-admit-snapshots-and-invocations.md): the selection forms.
- [FR-312](FR-312-read-a-clause-run-request-and-run-it.md): the clause-run request reader.
