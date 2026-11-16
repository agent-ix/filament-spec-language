---
id: FR-896
title: "Decide a global-conformance claim over the protocol system or a recording"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-636
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-890
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-895
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-059
    type: implements
  - target: ix://agent-ix/filament-specification/FR-060
    type: depends_on
---
# FR-896: Decide a global-conformance claim over the protocol system or a recording

## Description

FSL SHALL decide the claim kind `global-conformance` (FR-057, FR-507) over a
protocol clause as FSpec FR-059's judgement: the executions conform to the
protocol's causal, channel, activation, compensation and closure
obligations, the obligations FR-895 lists. An engine serves the kind in two
modes, selected by the item.

- **Over a recording.** With a `recording` member, the item is assessed in
  replay mode (FR-895): one recorded history, one verdict per obligation.
- **Over every behaviour.** With no `recording`, the item decides
  conformance for every behaviour of the protocol composed with its role
  implementations. A role's *implementation* is what its binding names:
  - the model of `role r on M::T` or `role r on M`;
  - the component that owns the Port of a role on a Port;
  - the bound model of a replicated role's population.

  Its operations are applied by the protocol's `attempt` steps (ADR-027
  ST-1). The item settles `proved` exactly when both of the following hold:
  1. Each role's projection under `bounded` (FR-890) is `proved`, so each
     implementation can run its role's local graph without global knowledge.
  2. The protocol system (FR-205) satisfies FR-895's obligations on every
     behaviour.

  The protocol system steps only enabled steps, so in this mode causal
  order, enabledness and the channel premises hold by construction. The
  obligations decided are:
  - the constraint and control obligations, with FR-120's invariant findings
    and FR-882's undefined-evaluation rule;
  - the compensation obligations;
  - the `requires temporal` requirements;
  - closure, read as FR-211's deadlock freedom against the declared terminal
    states.

  These are the protocol model-checking items of FR-636, so they settle as
  FR-636 states. A refutation carries the failing part's counterexample: a
  projection counterexample (FR-574) naming the role, or a protocol
  counterexample (FSpec FR-433). Either replays.
- **Missing implementation.** A role whose binding names no model,
  component or population in the linked package or the protocol subject is
  refused `missing_declaration`/`missing-name` naming the role, before any
  analysis. The claim is never settled `unsupported` for want of an engine.

The capability `Capability::GlobalConformance` (FR-057) is served by the
protocol-analysis provider in both modes. A request routed to it settles by
this requirement, never with a stub's pending obligation.

### Decision: one meaning, two evidence sources

FSpec FR-059 defines global conformance once, as conformance to the
protocol's obligations. FR-636 used the same kind name for checking the
protocol system, and the ticket asked for checking a set of implementations.
These are one judgement over two sources of executions: a recorded history,
or every behaviour that the protocol and its bound implementations can
produce. In FSL, a role's implementation is its bound model, and the protocol
system already composes those models through the attempts. So "do the
implementations together follow the protocol" is answered by the
projections, which show that each role can act locally, together with the
model check of the composition. An out-of-protocol send cannot arise from
the composition, because the protocol system takes only enabled steps. It
arises only in a recording, where replay mode reports it as `unexpected`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-896-AC-1 | The FR-636 order fixture with `Buyer` and `Seller` bound to `M::Cell` settles `global-conformance` `proved` with no `recording`: both projections are `proved` and every obligation holds over every behaviour. The protocol-analysis record lists the two projection results and the obligation verdicts. | Test (TC-2532) |
| FR-896-AC-2 | The fixture with the `receive` removed settles `refuted` with the deadlock counterexample of FR-636-AC-1, which replays. FR-890-AC-2's unmerged `repeat` settles `refuted` with its projection counterexample naming `Seller`. | Test (TC-2532) |
| FR-896-AC-3 | The same item with FR-895-AC-5's recording, in which `Seller` sends on `Tell` where the protocol enables no send, settles `refuted` with the `unexpected` verdict, and its counterexample replays as a recording prefix. | Test (TC-2532) |
| FR-896-AC-4 | A protocol whose role `Courier on N::Truck` names a model `N` that the linked package does not select is refused `missing_declaration`/`missing-name` naming `Courier`, and no verdict is settled. No `global-conformance` item settles `unsupported` with a pending obligation in the corpus. | Test (TC-2532) |

## Dependencies

- [FR-636](FR-636-request-protocol-analysis-items-and-replay-supplied-protocol-traces.md):
  the protocol model-checking items.
- [FR-890](FR-890-project-every-protocol-construct-under-the-bounded-profile.md):
  projection.
- [FR-895](FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md):
  replay mode and the obligation list.
- [FR-205](FR-205-define-the-protocol-subject-and-its-state-key.md): the
  protocol system and role bindings.
- FSpec FR-059 and FR-060.
