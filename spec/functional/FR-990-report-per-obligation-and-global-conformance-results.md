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
  - target: ix://agent-ix/filament-specification/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-212
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-428
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1032
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
| `obligation` | `{kind, node, instance[, role_instance]}`. `kind` is one of `causal-order`, `enabledness`, `channel-premise`, `constraint`, `control`, `window`, `compensation`, `requirement` or `closure`. `node` is the protocol node, channel or clause the obligation is about, by its checked identity. `instance` is the protocol instance's FSpec FR-287 runtime key, or absent for an obligation over every instance. `role_instance` is present only on a fan-out member's `control` obligation (below), and is that member's FSpec FR-737 replicated-role `instance` object reference |
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
| `premises` | the required closure-premise identities, each `{kind, instance, node[, role_instance]}` (below). `kind` is `execution`, `branch` or `workflow` |
| `producer` | `recording` or `engine` |

The required premises are the protocol's own facts, in FSpec FR-052's
authored partial order. There is one `execution` premise for each protocol
instance, keyed by its FR-287 runtime key, or a single one for a protocol
that does not activate `on each`. There is one `branch` premise for each
`choice` case, `parallel` branch and fan-out member of that instance, and
one `workflow` premise for each compensation workflow the instance
registers. They come from the checked protocol and never from the
observations.

**Premise identity.** A premise is the closed record `{kind, instance,
node[, role_instance]}`. Premises are authored protocol facts (FSpec FR-247
"Causal order"), so each component is read from the checked protocol and
none is inferred from a display name, a position or an observation:

- `instance` is the protocol instance's FSpec FR-287 runtime key. It is
  absent when the protocol does not activate `on each`, and present on every
  premise when it does.
- `node` is the checked identity of the node that closes the premise,
  spelled as an obligation's `node` is: for `execution`, the protocol's
  `finish` terminal; for `branch`, the `choice` case, `parallel` branch or
  fan-out node the branch is; for `workflow`, the step that registers the
  compensation workflow.
- `role_instance` is present only on the `branch` premise of a fan-out
  member, and is that member's FSpec FR-737 replicated-role `instance`
  object reference.

Two premises are the same premise exactly when every component is equal,
compared componentwise in the order `kind`, `instance`, `node`,
`role_instance`, and an absent component equals only absence. The list is
in FSpec FR-052's authored partial order of the premise nodes; a reader
compares a premise set by these identities and never by list position. A
premise whose `kind` and `node` disagree (an `execution` premise naming a
case node), whose `instance` is present for a protocol without `on each` or
names no activated instance, or whose `role_instance` names no member of the
fan-out is cross-wired or scope-mismatched.

**Fan-out members over a recording.** A fan-out node is a `choice` case or
`parallel` branch that holds a node owned by a replicated role (FSpec
FR-428). Its members in one protocol instance are the role instances that
the deployed system activated for that role in that instance. Which role
instances those are is a runtime binding (FSpec FR-212: a role instance is
bound explicitly, never computed), so neither the checked protocol nor the
observations supply it. The analyze request supplies it as the
recording-mode item's `population` member, and FSL passes it to the
recording assessor as a parameter beside the checked package and the
recording bytes. The FSpec FR-737 recording document is unchanged: its
recorder is the party under assessment, so the member list is not read
from it.

- **The parameter.** The assessor takes the activated population as either
  *known* or *unknown*, FSpec FR-285's two variants. A request item with a
  `population` member passes it known; an item without one passes it
  unknown. A known population is a list of entries `{role, instance,
  members}`: `role` is a replicated role's declared name; `instance` is the
  protocol instance's FSpec FR-287 runtime key, present exactly when the
  protocol activates `on each`; `members` is the duplicate-free array of
  that role's activated role-instance references, each an FSpec FR-737
  replicated-role `instance` object `{authority, identity}`. An empty
  `members` array is a supplied empty set, distinct from an absent entry.
  The item's FR-331 request identity covers the population, as FR-1032's
  covers the recording.
- **Enumeration.** For each recorded protocol instance, each fan-out node of
  that instance, and each replicated role that owns a node inside it, the
  required members are that role's `members` for that instance. Each
  required member contributes one `branch` premise `{kind: branch,
  instance, node, role_instance}` and one `control` obligation with the
  same `instance`, `node` and `role_instance`, in the role's FSpec FR-212
  canonical member order. A member's `control` obligation is `satisfied`
  when that member's observations inside the fan-out node reach the node's
  join or merge, and otherwise follows FR-895's rule for an absent
  deciding observation. The required premises and obligations never come
  from the members the observations name, so a member that is absent from
  the recording is still enumerated.
- **A member absent from the recording.** A required member whose premise
  the closure record omits makes the recording's premise set incomplete,
  and the recording is refused `invalid_runtime_input`/`invalid-value`
  naming that member's premise, by the strict-reader rule below. When the
  closure record lists the member's premise and the recording holds no
  observation by that member inside the fan-out node, the member's
  `control` obligation is `violated`, cause `missing`, with an empty
  `witness`, once `closure.state` is `closed`, and the item settles
  `refuted`; while the record is not `closed` it is `pending`. An absent
  member is never reported as incomplete closure while the population is
  known.
- **An unknown population.** Without a `population` member the required
  member set is unknown, and no complete set is invented. FSL still admits
  the recording and checks every fan-out premise the closure record states
  against the members its observations name. Every fan-out member's
  `control` obligation and the closure obligation of an instance with a
  fan-out node stay `pending`, `closure.state` is copied as stated, and the
  global result is `inconclusive`, cause `incomplete-closure-scope`
  (the `global` table). This is the only case in which a recording's
  closure is held incomplete because of membership.
- **Refusals.** A `population` member that is not an array, or an entry
  that is not a closed `{role, instance, members}` object, or a member that
  is not a closed `{authority, identity}` object with both strings
  non-empty, refuses the request `invalid-request` at the member's pointer
  (`/items/<i>/population/...`), exit 20, before any item runs, as FR-1032
  refuses a malformed document binding. `population` on an item that has
  no `recording` refuses the request the same way at `/items/<i>/population`.
  The following refuse the item `invalid_runtime_input` naming the entry or
  member, before any verdict:
  - an entry whose `role` the protocol does not declare (`missing-name`), or
    names a static role (`invalid-value`);
  - an entry with `instance` for a protocol without `on each`, or without
    one for a protocol that activates `on each`, or naming an instance the
    recording holds no observation of (`invalid-value`);
  - two entries for one `{role, instance}`, or one reference listed twice in
    one entry's `members` (`invalid-value`);
  - a fan-out role of a recorded instance with no entry (`missing-member`),
    since a known population that leaves a role out is neither known nor
    unknown for that role;
  - an observation or a closure premise naming a `role_instance` that is not
    among its role's `members` for its instance (`invalid-value`, naming the
    observation's index or the premise).

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
| Over a recording, no obligation `violated`, the protocol has a fan-out node and the activated population is unknown | `inconclusive`, cause `incomplete-closure-scope` | `unsettled` | `false` |
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

### Decision: the activated population is a request input, not a recording member

A fan-out member that the recording omits entirely leaves no trace in it,
so the member set cannot be derived from the observations. The recording
cannot be its source either: the recorder is the system under assessment,
and a member list it writes would let it omit a member and its premise
together, which no check could catch. The checked protocol's population
declaration is not the source because a replicated role spawns from its
population nondeterministically and at most `max n` at a time (FSpec
FR-428), so the population's universe is the set a role may draw from, not
the set a run activated; requiring every universe object would refute a
conforming run that activated fewer. The activated role instances are a
runtime binding that FSpec FR-212 says is supplied explicitly, so the
analyze request supplies them, as it supplies the subject's bindings to the
other analyses. Supplied, the required set is FSpec FR-285's `known`
variant and an absent member is a refutation or a refused premise set;
not supplied, it is `unknown`, and the result says so with its own cause
instead of reporting a closure that nothing established.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-990-AC-1 | `global-conformance` over the FR-636 order fixture with no recording settles an item whose `obligations` has one entry per FR-895 obligation. The causal-order, enabledness and channel-premise entries are `satisfied`, `by-construction`, and every other entry is `satisfied`, `explored`. `closure` is `{state: closed, producer: engine}` with one `execution` premise and one `branch` premise for each of `Ship` and `Cancel`. `global` is `{label: proved, basis: closed-scope, complete: true}`, and `projections` holds `Buyer` and `Seller`. | Test (TC-2800) |
| FR-990-AC-2 | The fixture with the `receive` removed settles `global` `{label: refuted, complete: false}`. Its closure obligation is `violated`, cause `counterexample`, with a `witness` that indexes the item's deadlock counterexample, and `closure.state` is `open`. | Test (TC-2800) |
| FR-990-AC-3 | FR-895-AC-1's closed recording settles `closure` `{state: closed, producer: recording}` and `global.complete` `true`. The same recording with closure `open` settles `inconclusive`, cause `open-history`, with `complete` `false`, and the verdicts of the obligations its observations decide are unchanged. With closure `contradicted` it settles `inconclusive`, cause `open-history`, with `closure.state` `contradicted`. | Test (TC-2801) |
| FR-990-AC-4 | A recording whose closure is `closed` but whose premise set omits the `Ship` branch premise, lists the execution premise twice, or names an instance the protocol does not activate is refused `invalid_runtime_input` naming the premise, and no verdict is reported. A recording that observes `finish` but states closure `open` keeps `closure.state` `open`. | Test (TC-2801) |
| FR-990-AC-5 | A run of the order fixture with `max_states` 1 reports its undecided obligations `pending` and `closure.state` `incomplete`. `global` is `{label: incomplete, cause: limit-reached, basis: unavailable, complete: false}` and names `max_states`, which is FSpec FR-360 `failed`, execution `resource-incomplete`. It is never `proved` or `inconclusive`. | Test (TC-2800) |
| FR-990-AC-6 | `global-conformance` over the FR-636 order fixture with no recording settles `closure.premises` exactly `[{kind: execution, node: Flow::Done}, {kind: branch, node: Flow::Main::Decide::no}, {kind: branch, node: Flow::Main::Decide::yes}]`, with no `instance` member (the fixture does not activate `on each`) and no `role_instance`. FR-895-AC-1's closed recording whose premises are those three records is admitted. The same recording with an `instance` member on the execution premise, with the execution premise naming `Flow::Main::Decide::yes`, or with the two branch premises' nodes exchanged for a `workflow` kind is refused `invalid_runtime_input` naming that premise, before any verdict. | Test (TC-2831) |
| FR-990-AC-7 | The fan-out fixture is protocol `R` over the litmus domain with `role r each M::Cell from cells max 2 lifetime workflow` and `parallel Both` of branches `left` (event `LeftWork by r`) and `right` (event `RightWork by r`) joined `all`. With `population` `[{role: r, members: [x, y]}]`, a closed recording in which `x` and `y` each take both events and whose premises are the execution premise and the four `{branch, node, role_instance}` premises settles `proved`, `complete` `true`, with one `control` obligation per member per branch, each `satisfied` and carrying its `role_instance`. | Test (TC-2951) |
| FR-990-AC-8 | With the same `population`, a closed recording in which only `x` is observed and whose premises omit `y`'s two branch premises is refused `invalid_runtime_input` naming `{branch, node: R::Main::Both::left, role_instance: y}`, before any verdict. The same recording whose premises list `y`'s branch premises settles `refuted`, with `y`'s `control` obligations at `R::Main::Both::left` and `R::Main::Both::right` `violated`, cause `missing`; with closure `open` those obligations are `pending` and the item is `inconclusive`, cause `open-history`. | Test (TC-2951) |
| FR-990-AC-9 | AC-7's complete closed recording with no `population` member settles `inconclusive`, cause `incomplete-closure-scope`, `complete` `false`, with `closure.state` `closed` as stated and every member `control` obligation `pending`; it is never `proved`. | Test (TC-2951) |
| FR-990-AC-10 | With the AC-7 recording, a `population` entry for an undeclared role `q` refuses the item `invalid_runtime_input`/`missing-name`; an entry with an `instance` member, a `members` array listing `x` twice, and an observation by `z` not in `members` each refuse it `invalid_runtime_input`/`invalid-value`; a `population` with no entry for `r` refuses it `invalid_runtime_input`/`missing-member`. A `population` that is not an array, a member `{authority: "test"}`, and `population` on an item with no `recording` each refuse the request `invalid-request` at the member's pointer, exit 20. | Test (TC-2951) |

## Dependencies

- [FR-896](FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md):
  the claim, its two modes and the projections beside the verdict.
- [FR-895](FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md):
  the obligation list and the recording verdicts.
- [FR-636](FR-636-request-protocol-analysis-items-and-replay-supplied-protocol-traces.md):
  budgets and the model-checking items.
- [FR-991](FR-991-publish-in-process-replay-of-a-global-conformance-counterexample.md):
  replay of the refutation's counterexample.
- [FR-1032](FR-1032-bind-a-protocol-recording-to-the-analyze-request-by-document-identity.md):
  the recording-mode item and its request identity, which the `population`
  member joins.
- FSpec FR-059, FR-061 and FR-247; FSpec FR-285 (known and unknown
  participation), FR-212 (role-instance binding and order) and FR-428
  (replicated roles).
