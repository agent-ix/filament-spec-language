---
id: FR-968
title: "Register a provider for the kinds it advertises, and value-validity only in the unbounded mode"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-962
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-288
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-290
    type: depends_on
---
# FR-968: Register a provider for the kinds it advertises, and value-validity only in the unbounded mode

## Description

A provider, linked or a process plugin, SHALL register for every (kind, mode)
pair its descriptor advertises, subject to two refusals at registration: a
descriptor that advertises no pair, and a descriptor that advertises
`value-validity` without advertising it in the `unbounded` mode.

The registration refusal has two purposes, and this rule keeps both. First, an
item reaches a provider only for a pair the provider advertises, so a
descriptor that advertises nothing could never be negotiated and is a
configuration error. Second, the value-validity provider arm exists to decide
items over unbounded mathematical `Integer` operands that no in-process
engine decides (FR-281 "External provider arm", FR-495); a provider that
advertises `value-validity` only in the `bounded` mode would compete with the
linked bounded arm for items it was never meant to take. Neither purpose
applies to a provider of another kind: a process provider advertising
`operation-contract` is a legitimate third candidate beside `en-1` and
`contract-checker`, and FR-962 states how such a tie settles.

## Inputs

The provider's descriptor (FR-288): its `BackendId` and advertised pairs.

## Outputs

A registered engine, or a registration refusal naming the `BackendId`.

## Behavior

- Registration SHALL refuse, naming the `BackendId`, a descriptor that
  advertises no (kind, mode) pair, and a descriptor that advertises
  `value-validity` in some mode but not in `unbounded`. A refused provider is
  never called and the registry never lists it.
- Every other descriptor SHALL register, whatever kinds it advertises, and
  negotiation (FSpec FR-290) routes to it only items whose (kind, mode) it
  advertises; an item handed `Supported` on it for another pair settles
  `inconsistent-candidates` (FR-281).
- A provider's verdict settles by FR-281's provider-arm rules for every
  kind: `proved` naming the provider, `refuted` only after S6a replay
  reproduces the counterexample for the item's kind, `unknown` as
  `inconclusive` `solver-inconclusive`.
- The rule is the same for `Engine::external` and for a process provider
  started from `--provider-config` (FR-635).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-968-AC-1 | Building an engine from a provider whose descriptor advertises only (`operation-contract`, `unbounded`) succeeds and the registry beside `Engines::fsl()` lists it; one advertising (`operation-contract`, `bounded`) and (`value-validity`, `unbounded`) succeeds; one advertising (`operation-contract`, `unbounded`) and (`value-validity`, `bounded`) only is refused naming its `BackendId`; one advertising no pair is refused naming its `BackendId`; neither refused provider is called. | Test (TC-2725) |
| FR-968-AC-2 | With AC-1's first provider registered, an unnamed `operation-contract` item for an operation's `post` contract has candidates `[contract-checker, en-1, <provider>]` and settles `ambiguous-backend` (FR-962-AC-3); the same item naming the provider is sent to it, and a `value-validity` item handed `Supported` on it settles `invalid-request` `invalid_capability`/`inconsistent-candidates` naming it, and the provider is not called. | Test (TC-2725) |

## Dependencies

- FR-281: the provider arm; FR-635: process providers; FR-288: descriptors.
- FR-962: the tie a third `operation-contract` provider leaves whole.
- FSpec FR-290: negotiation routes only advertised pairs.
