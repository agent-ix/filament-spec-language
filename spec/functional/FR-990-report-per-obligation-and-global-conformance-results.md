---
id: FR-990
title: "Report per-obligation and global conformance results with an independent closure record"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-896
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-895
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-636
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-890
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-059
    type: implements
  - target: ix://agent-ix/filament-specification/FR-247
    type: implements
  - target: ix://agent-ix/filament-specification/FR-061
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-433
    type: depends_on
---
# FR-990: Report per-obligation and global conformance results with an independent closure record

## Description

Every settled `global-conformance` item (FR-896), in either mode, SHALL
carry FSpec FR-059's outputs as three members of its analyze item, beside
the FR-360 label: one result per obligation (`obligations`), the
complete-global-conformance closure record (`closure`), and one global
result (`global`). Over every behaviour, it also carries the projection
results that FR-896 reports beside the verdict (`projections`). The global
result is computed from the other two members, and the closure record is
never computed from the global result or from any per-obligation verdict.

### `obligations`: one result per obligation

`obligations` holds one entry for each obligation FR-895 lists for the
protocol. Each causal edge, observation, channel, bound event node, control
node, window, compensation registration, `requires temporal` clause and
protocol instance has exactly one entry, in the protocol's canonical node
order and then by instance key. Each entry has these members:

| Member | Value |
| --- | --- |
| `obligation` | `{kind, node, instance}`. `kind` is one of `causal-order`, `enabledness`, `channel-premise`, `constraint`, `control`, `window`, `compensation`, `requirement` or `closure`. `node` is the protocol node, channel or clause the obligation is about, by its checked identity. `instance` is the protocol instance's FSpec FR-287 runtime key, or absent for an obligation over every instance |
| `verdict` | `satisfied`, `violated` or `pending` |
| `cause` | only with `violated`. Over a recording it is FSpec FR-059's cause: `unexpected`, `missing`, `misordered`, `duplicated` or `wrongly-correlated`. Over every behaviour it is `counterexample` |
| `basis` | `observed` over a recording. Over every behaviour it is `explored` for an obligation the model check decided, and `by-construction` for causal order, enabledness and the channel premises, which hold in the protocol system because it steps only enabled steps (FR-896) |
| `witness` | with `violated`, the evidence: over a recording, the indices of the observations that show the violation; over every behaviour, the index of the protocol counterexample (FSpec FR-433) in the item's `counterexample` member |

Over every behaviour an obligation is `satisfied` when it holds on every
explored behaviour and exploration finished, `violated` when a behaviour
breaks it, and `pending` when a run limit stopped exploration before it was
decided. Over a recording the verdicts are FR-895's. An obligation is never
`satisfied` because no violation was found before a limit or before the
recording ended.

### `closure`: the complete-global-conformance closure record

`closure` is FSpec FR-247's record, kept as a fact of its own (FSpec FR-061,
fifth axis). Its members:

| Member | Value |
| --- | --- |
| `state` | `closed`, `open`, `incomplete` or `contradicted` (FSpec FR-247) |
| `premises` | the required closure-premise identities, each `{kind, identity}`. `kind` is `execution`, `branch` or `workflow` |
| `producer` | `recording` or `engine` |

The required premises are the protocol's own facts, in FSpec FR-052's
authored partial order. There is one `execution` premise for each protocol
instance, keyed by its FR-287 runtime key, or a single one for a protocol
that does not activate `on each`. There is one `branch` premise for each
`choice` case, `parallel` branch and fan-out member of that instance, and
one `workflow` premise for each compensation workflow the instance
registers. They come from the checked protocol and never from the
observations.

**Who produces the record.**

- **Over a recording**, the recording produces it: the recorder of the
  deployed system writes it into the FSpec FR-737 document, and FSL copies
  it. `producer` is `recording`. FSL checks the premise set against the
  premises the protocol requires for the recorded instances. An omitted,
  duplicated, cross-wired or scope-mismatched premise refuses the recording
  with `invalid_runtime_input` before any verdict, naming the premise (FSpec
  FR-059's strict reader). FSL never upgrades the stated `state`, and in
  particular never makes it `closed` because a `finish` was observed.
- **Over every behaviour**, the engine produces it from the protocol system,
  and `producer` is `engine`. A premise is closed when every explored
  behaviour that reaches it also reaches its declared closure: an execution
  reaches its `finish` terminal state, a branch reaches its join or merge,
  and a workflow reaches its commit or a FR-893 compensation outcome. The
  state is:
  - `closed` when every required premise is closed on every behaviour;
  - `open` when some explored behaviour never closes a premise: a deadlock
    short of `finish`, or a fair lasso that never reaches it;
  - `incomplete` when a run limit stopped exploration before every
    premise's closure was shown;
  - never `contradicted`: the protocol system is a function of its state
    key, so its closure facts cannot disagree. A `contradicted` record
    comes only from a recorder that reports its own closure facts as
    disagreeing.

  The run's completion is never itself the closure fact. A completed
  exploration with an unclosed premise gives `open`, not `closed`.

**Independence.** No other fact stands in for `closure.state`: not a closed
decision scope, a closed surrounding execution, a completed assessment, a
settled per-obligation verdict, nor a decisive counterexample. Changing the
closure record never changes a per-obligation verdict that was settled from
the observations or from exploration (FSpec FR-247-AC-5). FR-895's rule
that an absent observation is `pending` while the record is not `closed`,
and `violated`, cause `missing`, once it is `closed`, is the only way the
record affects a verdict. It reads the record and never writes it.

### `global`: the global conformance result

`global` has the members `label`, `basis`, `complete` and, when the label is
`inconclusive` or `incomplete`, `cause`. `label` is the item's FR-360 label:

| Condition | `label` | `basis` | `complete` |
| --- | --- | --- | --- |
| Any obligation `violated` | `refuted` | `decisive-counterexample` | `false` |
| Every obligation `satisfied`, and `closure.state` is `closed` with every required premise present | `proved` | `closed-scope` | `true` |
| No obligation `violated`, no run limit reached, and some obligation is `pending` or `closure.state` is `open`, `incomplete` or `contradicted` | `inconclusive`, cause `open-history` | `unsettled` | `false` |
| No obligation `violated`, and a run limit stopped the run | `incomplete`, cause `limit-reached`, naming the limit and the value used | `unavailable` | `false` |

The `contradicted` closure state takes the `open-history` row, as FR-895
settles any unviolated, incomplete assessment. The closure record's `state`
carries the detail, and no cause is added for it. A run that a limit
stopped settles by FR-636's budgets rule. This is FSpec FR-360's `failed`
with execution `resource-incomplete`, and FR-573 settles a stopped run the
same way. It is never `inconclusive` `open-history`, because the
undecided obligations come from the limit and not from the history. `complete` is
`true` in one case only: complete global success, which is FSpec FR-059's
judgement. The complete result is `proved` together with a `closed` record
that holds every required premise. A `refuted` result settles without
waiting for closure, because a violation that has been shown stays a
violation. The refutation's counterexample is FR-896's: the protocol
counterexample over every behaviour, or the recording prefix up to the first
violating observation over a recording. It replays as FR-991 states.

### `projections`: beside the verdict

Over every behaviour, `projections` holds one entry per role,
`{role, label, counterexample}`, with FR-890's `bounded` result and its
FR-574 counterexample when it is refuted. The entries are evidence that a
role can run its local graph without global knowledge. They are never inputs
to `obligations`, `closure` or `global` (FR-896). Over a recording the
member is absent.

### Decision: three members, and a closure record that nothing else can stand in for

FSpec FR-059 asks for per-obligation results, a global result and a
closure state that is kept independently of both. If these were one label,
a reader could not tell which promise broke, and could not tell a run that
showed no violation from a run that showed conformance. The members are
therefore separate, and only `global.complete` says that the protocol was
followed completely. The closure record must come from whoever can see
the executions end. For a deployed system that is the recorder, so FSL
copies the record and checks it but never creates it. Over every behaviour,
the protocol system is the execution, so the engine is the producer, and it
derives closure from terminal facts about the behaviours rather than from
the engine finishing its run. That keeps the five FR-061 axes apart in both
modes.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-990-AC-1 | `global-conformance` over the FR-636 order fixture with no recording settles an item whose `obligations` has one entry per FR-895 obligation. The causal-order, enabledness and channel-premise entries are `satisfied`, `by-construction`, and every other entry is `satisfied`, `explored`. `closure` is `{state: closed, producer: engine}` with one `execution` premise and one `branch` premise for each of `Ship` and `Cancel`. `global` is `{label: proved, basis: closed-scope, complete: true}`, and `projections` holds `Buyer` and `Seller`. | Test (TC-2800) |
| FR-990-AC-2 | The fixture with the `receive` removed settles `global` `{label: refuted, complete: false}`. Its closure obligation is `violated`, cause `counterexample`, with a `witness` that indexes the item's deadlock counterexample, and `closure.state` is `open`. | Test (TC-2800) |
| FR-990-AC-3 | FR-895-AC-1's closed recording settles `closure` `{state: closed, producer: recording}` and `global.complete` `true`. The same recording with closure `open` settles `inconclusive`, cause `open-history`, with `complete` `false`, and the verdicts of the obligations its observations decide are unchanged. With closure `contradicted` it settles `inconclusive`, cause `open-history`, with `closure.state` `contradicted`. | Test (TC-2801) |
| FR-990-AC-4 | A recording whose closure is `closed` but whose premise set omits the `Ship` branch premise, lists the execution premise twice, or names the premise of another instance is refused `invalid_runtime_input` naming the premise, and no verdict is reported. A recording that observes `finish` but states closure `open` keeps `closure.state` `open`. | Test (TC-2801) |
| FR-990-AC-5 | A run of the order fixture with `max_states` 1 reports its undecided obligations `pending` and `closure.state` `incomplete`. `global` is `{label: incomplete, cause: limit-reached, basis: unavailable, complete: false}` and names `max_states`, which is FSpec FR-360 `failed`, execution `resource-incomplete`. It is never `proved` or `inconclusive`. | Test (TC-2800) |

## Dependencies

- [FR-896](FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md):
  the claim, its two modes and the projections beside the verdict.
- [FR-895](FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md):
  the obligation list and the recording verdicts.
- [FR-636](FR-636-request-protocol-analysis-items-and-replay-supplied-protocol-traces.md):
  budgets and the model-checking items.
- [FR-991](FR-991-publish-in-process-replay-of-a-global-conformance-counterexample.md):
  replay of the refutation's counterexample.
- FSpec FR-059, FR-061 and FR-247.
