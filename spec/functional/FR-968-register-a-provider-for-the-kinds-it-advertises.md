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
  - target: ix://agent-ix/filament-specification/FR-485
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-075
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
  `value-validity` in some mode but not in `unbounded`. The refusal also names
  which of the two rules the descriptor breaks (`no-advertised-pair` or
  `value-validity-not-unbounded`). A refused provider is never called and the
  registry never lists it. These are the only registration refusals: a
  descriptor's kinds, origin and mode set are otherwise never a reason to
  refuse it.
- Every other descriptor SHALL register, whatever kinds it advertises, and
  negotiation (FSpec FR-290) routes to it only items whose (kind, mode) it
  advertises; an item handed `Supported` on it for another pair settles
  `inconsistent-candidates` (FR-281).
- **Candidates.** A registered provider SHALL be a member of an item's
  FR-075 matching set exactly when it advertises the item's (kind, mode) and
  FR-075's other filters (evidence kind, domain, dynamics) admit it, the same
  test every linked engine passes; registration order never changes the set or
  its candidate order. FR-962 then narrows the set before negotiation: its
  `en-1` exclusion for an `operation-contract` item that is not an `invariant`
  state clause over a named model subject applies with the provider present,
  and a row decides the item only when the set is exactly the providers the
  row ties, so a set that also holds the provider stays whole and an unnamed
  item settles `invalid-request` `invalid_capability`/`ambiguous-backend`
  naming every candidate. An item naming the provider has the provider alone
  as its candidate when the provider matches it, and FR-075's named-backend
  rule otherwise.
- **Request.** An item settled `Supported` on the provider SHALL be sent to
  it in one FSpec FR-331 request per package with the routed request item,
  the admitted package bytes and the registry manifest, whatever the item's
  kind. The in-process interface (`ProviderRequest`) gives each item, beside
  its request item, the claim target that the kind's linked arm selects for
  the item: the value target for `value-validity`, the FSpec FR-485 contract
  target (form, claim, subject) for `operation-contract`, and for every other
  kind the target its linked engine reads. An item whose target its kind's
  arm refuses to select settles as that arm settles it and is not sent.
- **Verdicts.** A provider's verdict settles by FR-281's provider-arm rules
  for every kind: `proved` naming the provider (`uncertified` for a `Linked`
  origin, `trusted` for a `Process` one), `unknown` as `inconclusive`
  `solver-inconclusive`, and `refuted` only after the replay that owns the
  counterexample's type reproduces it for the item's claim. A refuted verdict
  carries one FR-281 `Counterexample`: `Assignments` replayed by the
  value-validity replay of FR-281 "External provider arm"; `Contract`, an
  FSpec FR-485 `contract-counterexample`, replayed by FSpec FR-485 "Replay"
  with no reachability prefix (a `bound` one, which this reader cannot
  re-execute, settles `inconclusive` `replay-refused` and is never `refuted`);
  and each other variant by the replay FR-281's `Counterexample` list names
  for it. A counterexample whose type does not belong to the item's kind
  (FSpec FR-331: a process provider's entry carries `assignments`, a
  `contract-counterexample` only on an `operation-contract` item, a
  `hybrid-counterexample` only on a hybrid item), or that its replay does not
  reproduce, settles the item `inconclusive` `replay-parity` and is recorded
  nowhere.
- The rule is the same for `Engine::external` and for a process provider
  started from `--provider-config` (FR-635).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-968-AC-1 | Building an engine from a provider whose descriptor advertises only (`operation-contract`, `unbounded`) succeeds and the registry beside `Engines::fsl()` lists it; one advertising (`operation-contract`, `bounded`) and (`value-validity`, `unbounded`) succeeds; one advertising (`operation-contract`, `unbounded`) and (`value-validity`, `bounded`) only is refused naming its `BackendId`; one advertising no pair is refused naming its `BackendId`; neither refused provider is called. | Test (TC-2725) |
| FR-968-AC-2 | With AC-1's first provider registered beside `Engines::fsl()`, an unnamed `operation-contract` item for the `post` clause `Keeps` (`self.value = n` on `M::Counter::setTo`) reaches the negotiator with candidates exactly `[contract-checker, <provider>]` (FR-962 excludes `en-1` for a `post` contract) and settles `invalid-request` `invalid_capability`/`ambiguous-backend` naming both; the unnamed `invariant` state clause `self.value <= 3` in a request naming a model subject reaches it with `[contract-checker, en-1, <provider>]` and settles `ambiguous-backend` naming all three (FR-962-AC-3); the `Keeps` item naming the provider is sent to it as one request item of kind `operation-contract` whose target is `Keeps`'s FSpec FR-485 `post` target; and a `value-validity` item handed `Supported` on it settles `invalid-request` `invalid_capability`/`inconsistent-candidates` naming it, and the provider is not called for it. | Test (TC-2725) |
| FR-968-AC-3 | With AC-1's first provider registered and the Counter unit holding the `post` clause `Wrong` (`self.value = n + 1` on `M::Counter::setTo`), the `Wrong` item naming the provider settles by the verdict the provider returns: `Proved` gives `proved` naming the provider, certification `trusted`; `Unknown` gives `inconclusive` `solver-inconclusive`; a `Contract` counterexample with `bound` `false`, obligation `postcondition`, `violated` `Wrong`'s occurrence and one execution from pre-state `value = 0` with argument `n = 0` and post-state `value = 0` gives `refuted` naming the provider with exactly that counterexample in `counterexamples`; the same counterexample with post-state `value = 1`, which satisfies the clause, gives `inconclusive` `replay-parity` with no counterexample; the first with `bound` `true` gives `inconclusive` `replay-refused` with no counterexample; and an `Assignments` counterexample gives `inconclusive` `replay-parity` with no counterexample. No result is `refuted` without its replay. | Test (TC-2725) |
| FR-968-AC-4 | Negative: a manifest advertising no pair never registers: the manifest reader refuses it (FR-288-AC-3), or, where it builds a descriptor, `Engine::external` refuses that descriptor with reason `no-advertised-pair`; `Engine::external` over one advertising (`value-validity`, `bounded`) only with reason `value-validity-not-unbounded`, each naming its `BackendId`; a descriptor advertising only (`temporal-satisfaction`, `bounded`), and one advertising only (`schedulability`, `unbounded`), each register. | Test (TC-2725) |

## Dependencies

- FR-281: the provider arm; FR-635: process providers; FR-288: descriptors.
- FR-962: the tie a third `operation-contract` provider leaves whole.
- FSpec FR-290: negotiation routes only advertised pairs.
- FR-075: the matching set a provider joins. FSpec FR-485: the contract target and the replay of a `contract-counterexample`. FSpec FR-331: the counterexample types a plugin entry may carry.
