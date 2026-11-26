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
  - target: ix://agent-ix/filament-spec-language/FR-990
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1033
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
  An item with a `recording` also takes `assessment`, `batch` (the default)
  or `incremental`; any other word, or `assessment` on an item with no
  `recording`, is refused `invalid-request` naming `/items/<n>/assessment`.
  Under `incremental` the item additionally carries `prefixes`, one entry per
  observation in order, each `{observations, label, obligations}` for the
  assessment of that prefix (FR-895 "Incremental assessment"); the item's own
  verdict is the last prefix's, which equals the `batch` verdict.
- **Over every behaviour.** With no `recording`, the item decides
  conformance for every behaviour of the protocol composed with its role
  implementations. A role's *implementation* is what its binding names:
  - the model of `role r on M::T` or `role r on M`;
  - the component that owns the Port of a role on a Port;
  - the bound model of a replicated role's population.

  Its operations are applied by the protocol's `attempt` steps (ADR-027
  ST-1). The item settles `proved` exactly when the protocol system (FR-205)
  satisfies FR-895's obligations on every behaviour, which is FSpec
  FR-059's judgement over every history the composition can produce.
  Beside that verdict, and never as a premise of it, the record reports each
  role's projection under `bounded` (FR-890): whether each implementation can
  run its role's local graph without global knowledge. A refuted projection
  does not refute conformance, and a proved projection does not establish
  it.

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
  FR-636 states. A refutation carries the protocol counterexample (FSpec
  FR-433), which replays. A projection counterexample (FR-574) is reported
  with its projection result beside the verdict.
- **Missing implementation.** A role whose binding names no model,
  component or population in the linked package or the protocol subject is
  refused `missing_declaration`/`missing-name` naming the role, before any
  analysis. The claim is never settled `unsupported` for want of an engine.

### The objects checked over every behaviour

Over every behaviour, a protocol clause whose `over` parameter binds an
object type is checked over protocol subjects, each binding `over` to one
object (FR-205). The protocol deadlock item (`temporal.item_kind`
`deadlock-freedom`, FR-636) is given no subject per object: the first rule
below applies to it, and when its subject carries no `protocol` member FSL
SHALL settle it `declined` `invalid-input` with `invalid_runtime_input`/
`invalid-value` naming `temporal.subject.protocol`.

- When the item's subject (FR-1033) carries a `protocol` member naming a
  clause other than the claimed one, FSL SHALL settle the item `declined`
  `invalid-input` with `invalid_runtime_input`/`invalid-value` naming
  `temporal.subject.protocol`, before it explores a state; the request's
  other items run. A request-level `subject.protocol` an item inherits is
  read the same way.
- When the item's subject carries a `protocol` member naming the
  claimed clause, FSL SHALL decide the item over the protocol subject built
  from that binding, its `over` object the only object checked.
- When the item's subject carries no `protocol` member, FSL SHALL check one
  protocol subject for each admitted key of the `over` type (FR-125
  "Instancing"), in FR-125's canonical instance order. It SHALL NOT check
  the first admitted key alone. The item reports one record, combined as
  FR-621 "Items over several claim instances" states, and the run's limits
  and stops apply over the keys as FR-126 "Bindings and ranking" states them
  over the bindings of a TP-4 item.
- The item's `obligations`, `closure`, `global`, `projections`,
  counterexample and replay SHALL be those of the instance whose record the
  item reports: for a proved item, the instance FR-621 "The proved record
  over several instances" chooses. A counterexample's protocol subject, and
  the subject its replay reads (FR-991), bind that instance's object.
- When the subject gives no instance, the item SHALL settle as FR-125
  "Instancing" states, `NoInstance` or `NoInitialState` among its outcomes,
  and never as a fault.

An `over` input that is not an object reference ranges over its type's
values within one protocol system (FR-882).

The capability `Capability::GlobalConformance` (FR-057) is served by the
protocol-analysis provider in both modes. A request routed to it settles by
this requirement, never with a stub's pending obligation. In both modes the
item reports its per-obligation results, closure record and global result
in the shape FR-990 states, and its counterexample replays in process as
FR-991 states.

### Decision: one meaning, two evidence sources

FSpec FR-059 defines global conformance once, as conformance of executions
to the protocol's obligations. A recorded history and the set of every
behaviour that the protocol and its bound implementations can produce are
two sources of such executions, so one claim kind serves both. In FSL a
role's implementation is its bound model, and the protocol system already
composes those models through the attempts, so the model check of the
composition decides the judgement. Whether each role can also run locally is
a different question, answered by projection; the record reports it beside
the verdict, because FR-059 judges histories alone and a projection result
is not one. An out-of-protocol send cannot arise from the composition,
because the protocol system takes only enabled steps. It arises only in a
recording, where replay mode reports it as `unexpected`.

- A `proved` item decided over the protocol system is `certified` only when the
  core's closure check (FR-338) covers every obligation. That check reads
  closure and deadlock freedom; it reads neither the premise that an
  `exactly-once-premise` channel holds no undelivered message at `finish` nor
  a `requires temporal` requirement, so over a protocol with either the item
  SHALL settle `uncertified`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-896-AC-1 | The FR-636 order fixture with `Buyer` and `Seller` bound to `M::Cell` settles `global-conformance` `proved` with no `recording`, every obligation holding over every behaviour. The protocol-analysis record lists the obligation verdicts and, beside them, the two `bounded` projection results, both `proved`. | Test (TC-2532) |
| FR-896-AC-2 | The fixture with the `receive` removed settles `refuted` with the deadlock counterexample of FR-636-AC-1, which replays. A protocol whose obligations hold on every behaviour but whose projection onto `Seller` is refuted (FR-890-AC-2's unmerged `repeat`) settles `proved`, and the record reports the refuted projection with its counterexample beside the verdict. | Test (TC-2532) |
| FR-896-AC-3 | The same item with FR-895-AC-5's recording, in which `Seller` sends on `Tell` where the protocol enables no send, settles `refuted` with the `unexpected` verdict, and its counterexample replays as a recording prefix. | Test (TC-2532) |
| FR-896-AC-4 | A protocol whose role `Courier on N::Truck` names a model `N` that the linked package does not select is refused `missing_declaration`/`missing-name` naming `Courier`, and no verdict is settled. No `global-conformance` item settles `unsupported` with a pending obligation in the corpus. | Test (TC-2532) |
| FR-896-AC-5 | The AC-10 recording of FR-895 named by an item with `assessment` `incremental` settles `refuted` and carries six `prefixes` whose `label`s are `inconclusive`, then `refuted` five times, each with `observations` counts 1 to 6; its `Note` enabledness is `violated`, cause `unexpected`, from `prefixes[1]` on, and the last prefix's `obligations` equal the item's. The same item with `assessment` `batch`, or none, carries no `prefixes`. The word `streaming`, or `assessment` on an item with no `recording`, is refused `invalid-request` naming `/items/0/assessment`. | Test (TC-2531) |
| FR-896-AC-6 | An `on each` protocol recording of two instances `a` and `b`, listed one after the other, with an activated population naming both, assessed with `assessment` `incremental` settles `proved` with six `prefixes`; no prefix is refused for naming an unrecorded instance, each prefix holding the population of the instances it has begun, and the last prefix's `obligations` equal the `batch` item's. A refused prefix refuses the whole item `invalid_runtime_input`, with the refusal's cause, its detail and the `prefix` index. | Test (TC-2531) |
| FR-896-AC-7 | The keyed order fixture is the FR-636 order fixture with the constraint of `send Placed`, the first control of `Main`, written `{ k.r1 = 0 }`, over the universe `cores` `[p, q]` with an initial state holding `p.r1 = 0` and `q.r1 = 1`. With no `subject.protocol`, `global-conformance` settles `refuted`, `p` being proved and `q` blocked at `Placed` with wait cause `Constraint`: the counterexample is the deadlock with the empty prefix, its protocol subject's `over` names `q`, it replays, and `closure.state` is `open`. With `q.r1 = 0` too, the item settles `proved` with `closure.state` `closed`. | Test (TC-2532) |
| FR-896-AC-8 | Over AC-7's first state, with `subject.protocol` naming `Flow` and binding both roles, `over` `p` settles `proved`, and `over` `q` settles `refuted` with AC-7's counterexample. | Test (TC-2532) |
| FR-896-AC-9 | Over AC-7's fixture with a second protocol clause `Other` declared, a request whose `subject.protocol` binds `Other` holds two `global-conformance` items claiming `Flow`: the first inherits that subject and settles `declined` `invalid-input` with `invalid_runtime_input`/`invalid-value` naming `temporal.subject.protocol`, carrying no `obligations`, `closure`, `global`, `projections` or `counterexample`; the second, whose `temporal.subject` binds `Flow` to `p`, settles `proved`. | Test (TC-2532) |
| FR-896-AC-10 | The protocol deadlock item of AC-7's fixture (`claim` `Flow`, `temporal.item_kind` `deadlock-freedom`) with no `subject.protocol` settles `declined` `invalid-input` with `invalid_runtime_input`/`invalid-value` naming `temporal.subject.protocol`. | Test (TC-2532) |
| FR-896-AC-11 | The order fixture, whose `Order` channel is `exactly-once-premise`, settles `proved`, `uncertified`; with that channel `at-most-once` and no `requires temporal`, `proved`, `certified`. | Test (TC-2532) |

## Dependencies

- [FR-636](FR-636-request-protocol-analysis-items-and-replay-supplied-protocol-traces.md):
  the protocol model-checking items.
- [FR-890](FR-890-project-every-protocol-construct-under-the-bounded-profile.md):
  projection.
- [FR-895](FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md):
  replay mode and the obligation list.
- [FR-205](FR-205-define-the-protocol-subject-and-its-state-key.md): the
  protocol system and role bindings.
- [FR-1033](FR-1033-select-a-protocol-subject-and-memory-models-in-an-analyze-request.md):
  the request's protocol binding.
- [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-621](FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md)
  and [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md):
  the admitted keys, the one record over them and the stops.
- [FR-990](FR-990-report-per-obligation-and-global-conformance-results.md)
  and [FR-991](FR-991-publish-in-process-replay-of-a-global-conformance-counterexample.md):
  the result shape and its in-process replay.
- FSpec FR-059 and FR-060.
