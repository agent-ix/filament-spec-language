---
id: FR-868
title: "Refuse a version or digest in an observation document's model header"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-specification/FR-154
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: references
---
# FR-868: Refuse a version or digest in an observation document's model header

## Description

A `filament.state.snapshot/v1` or `filament.state.invocation/v1` document
names the domain package it observes in its `model` member (FR-106). A
domain package is selected by identity alone: the source `model` line, a
compile or run request's `models` entry and the checked package carry no
version and no digest (FSpec FR-154, FSpec FR-322), and the package
document's own `version` member is not read. An observation's `model`
member therefore is the closed object `{"identity": I}`, and FR-106's model
check compares `I` with the package's selection for the clause's alias.

### Decision: refuse the members, as a request selection does

A `version` or `digest` written into the header looks like a pin, but there
is nothing to compare it with: no version or digest of a model exists
anywhere in the pipeline. Accepting and ignoring it tells the author a pin is
checked when none is. A request's source selection already refuses the same
members as unknown, so the observation reader refuses them the same way,
and the two inputs agree: each defines exactly its members and refuses every
other. The identity, which is the one thing that selects a model, stays
checked by FR-106 check 4.

## Inputs

An observation document's `model` member.

## Outputs

The admitted observation, or FR-106's admission refusal.

## Behavior

- The observation reader SHALL read `model` as a closed object whose only
  member is `identity`.
- If `model` carries `version`, `digest` or any other member besides
  `identity`, then admission SHALL refuse `invalid_runtime_input`/
  `unknown-member` with `field` `model.<member>` (for example
  `model.version`), before the model check runs.
- When `model` carries `identity` alone, admission SHALL apply FR-106's
  model check to it unchanged.
- The observation writers (FR-106's document producers and the run's
  environment document) SHALL write `model` as `{"identity": I}`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-868-AC-1 | Through `command::run`, FR-108's healthy-parent `ParentOrder` clause request whose current snapshot's `model` is `{"identity": "example/config-version", "version": "1"}` refuses at stage `admit` `invalid_runtime_input`/`unknown-member` with `field` `model.version`, exit 20; with `{"identity": "example/config-version", "digest": "0000"}` it refuses naming `model.digest`; with `{"identity": "example/config-version", "version": "9.9.9", "digest": "x"}` it refuses naming the first of the two in member order. | Test (TC-2460) |
| FR-868-AC-2 | The same request with `model` `{"identity": "example/config-version"}` runs to `success`, exit 0; with `{"identity": "example/other"}` it refuses `invalid_model_binding`/`wrong-model-selection` (FR-106 check 4). | Test (TC-2460) |
| FR-868-AC-3 | An invocation document whose `model` carries `version` refuses the same way, naming `model.version`; and a compile request's `models` entry and an observation's `model` member each refuse an unknown member, so neither input accepts a member the other refuses. | Test (TC-2460) |

## Dependencies

- [FR-106](FR-106-admit-snapshots-and-invocations.md): the observation
  documents and their model check.
- FSpec FR-154: a model is selected by identity alone; FSpec FR-322: no
  version or digest of a model exists in the checked package.
