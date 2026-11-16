---
id: FR-852
title: "Decide state-graph items over a timed subject on the zone engine"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-628
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-632
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-238
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-241
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-243
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-245
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-712
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-390
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-393
    type: depends_on
---
# FR-852: Decide state-graph items over a timed subject on the zone engine

## Description

A `possible`, `always possible` or `unique path` item over a timed model
subject SHALL be decided over the subject's timed state graph, as FSpec
FR-712 defines it: by EN-6 through symbolic zone search, or, when the item
names `options.method` `digital-clocks` and the subject is closed, by EN-1
over the digitized subject (FR-243), as FR-632 routes a timed temporal item.
EN-6 SHALL advertise the three state-graph forms in its FSpec FR-331
`state_graph_forms`, and EN-1 keeps advertising them (FR-628), so
negotiation routes each item by its `method`, and the item settles `proved` or
`refuted` with FSpec FR-393 evidence whose delays are exact rationals.

### Decision: zones by default, digitized explicit state on request

EN-1's untimed search over a timed subject would drop the clocks, the guards
that read them and the time invariants, and so would decide a different
system: a node reachable only by waiting past a deadline would count as
reachable. A timed state-graph item is therefore decided with its clocks,
by one of the two routes FR-632 gives every timed item. Without
`options.method`, or with `zones`, EN-6 decides it on its zone graph. With
`digital-clocks`, EN-1 decides it over the integer-clock reading of the closed
subject (FR-243), with basis `digitized-explicit-state`; digitization keeps
the clocks, and over a closed subject it gives the zone verdict, so the route
is a cross-check of the zone result. `method` is a provider option EN-1
publishes as `digital-clocks` and EN-6 as `zones`, so one item reaches one
engine. The same items over an untimed subject go to EN-1 (FR-628).

## Inputs

A `native-analyze/1` item naming a state-graph claim whose subject is a
timed model (FR-632 "timed subject"), with FR-628's members and FR-632's
`temporal.limits` (FR-625's limits plus `max_clocks`).

## Outputs

FR-621's item: the record, the FSpec FR-393 evidence, `replay`, `certificate`
and `exploration`.

## Behavior

- **Routing.** EN-6's manifest SHALL list `possible`, `always-possible` and
  `unique-path` in `state_graph_forms` for `temporal-satisfaction` in the
  modes it advertises, and EN-1's lists them as FR-628 states. Over a timed
  subject, an item without `options.method`, or with `zones`, has EN-6 as its
  one candidate; an item with `digital-clocks` has EN-1 as its one candidate
  and is decided by FR-243's route, which refuses a subject that is not
  closed as FR-632 states. An item naming `backend` `en-1` without
  `digital-clocks` matches no candidate and settles by FSpec FR-290's
  empty-candidate row, `unsupported` (FSpec FR-712 "Routing").
- **Deciding.** On zones, EN-6 SHALL decide each claim instance by FSpec FR-712:
  `possible` by forward zone reachability of `P`; `always possible` by the
  backward fixpoint of `EF P` over zones, compared with the forward reachable
  zones where `Q` holds; `unique path` by counting realizable discrete-step
  sequences on the zone graph. On `digital-clocks`, EN-1 SHALL decide each
  instance by FSpec FR-712 over the digitized timed state graph, with basis
  `digitized-explicit-state` and integer delays that replay.
- **Witness and counterexample.** A `possible` proof carries, for each
  initial state, a witness path; an `always possible` refutation carries a
  path to a reachable state where `Q` holds and from which no `P` state is
  reachable; a `unique path` refutation carries the two distinct step
  sequences or the cycle. Each path SHALL be concretized with exact rational
  delays (FR-241), written in FSpec FR-393's timed evidence form, and replayed
  through FSpec FR-393's model-graph arm before it reaches the document; a
  path replay does not reproduce settles `inconclusive` `replay-parity`
  (FR-281).
- **Certificate.** A proof or refutation SHALL carry EN-6's zone certificate
  (FR-245) over the explored zone graph and, for `always possible`, FSpec
  FR-712's `state_graph` part whose `ef_nodes` list the `EF P` zones; the
  item's `certificate` member carries that certificate as its `document`, and
  certification is `certified` once FR-245's core checker
  accepts the certificate, and `uncertified` otherwise. The proof basis is
  `zone-certified` (FR-632).
- **Budgets.** Reaching a `temporal.limits` limit settles the item
  `incomplete` with `LimitReached{limit}` naming it; `exploration` reports
  the zone states and transitions explored.
- **Derived items.** A request whose items over the timed subject include a
  state-graph item also holds the subject's derived deadlock-freedom and
  time-lock-freedom items, as FR-626 states for a timed temporal item.

## Acceptance Criteria

The timed fixture is FR-632-AC-3's `Rpc` subject, provisioned with one call, at timeout 4: `send` sets
`phase` 1 and resets `x`, `reply` needs `x >= 1 ms`, `timeout` needs
`x >= 4 ms` with `phase` 1, `reset` returns to `phase` 0, and the time
invariants `x <= 3 ms` while in flight and `x <= 4 ms` while `phase` is 1.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-852-AC-1 | `possible Replied using v over (c: M::Call) { c.phase = 2 }` settles `proved` on `en-6` with a witness path `send` then `reply` whose delay before `reply` is an exact rational in `[1, 3]`, the path replays, and certification is `certified`; `possible Late using v over (c: M::Call) { c.phase = 3 }`, unreachable because the server deadline of 3 ms forces `reply` before the 4 ms timeout, settles `refuted`. | Test (TC-2412) |
| FR-852-AC-2 | `always possible BackToIdle using v over (c: M::Call) { c.phase = 0 }` settles `proved`, its `certificate` accepted with `document.state_graph.ef_nodes` listing the `EF P` zones; the same subject with `reset` removed settles `always possible BackToIdle` `refuted` with a counterexample path ending at `phase` 2. | Test (TC-2412) |
| FR-852-AC-3 | `unique path OneReply using v over (c: M::Call) from (c.phase = 0) to (c.phase = 2);` settles `refuted` with a cycle through `reset`, and over the subject with `reset` removed settles `proved`. | Test (TC-2412) |
| FR-852-AC-4 | AC-1's `Replied` item with `backend` `en-1` and no `options.method` settles `unsupported` with `unsupported_projection`/`unsupported-requested-capability`, exit 21; with `temporal.limits.max_states` 1 it settles `incomplete` `LimitReached{max_states}`, exit 22. | Test (TC-2412) |
| FR-852-AC-5 | `Engines::fsl()`'s `en-6` registration lists `possible`, `always-possible` and `unique-path` in `state_graph_forms`. | Test (TC-2412) |
| FR-852-AC-6 | AC-1's `Replied` and `Late` items with `options.method` `digital-clocks` settle the same verdicts on backend `en-1`, the proof with basis `digitized-explicit-state` and witness delays that are integers and replay; `Engines::fsl()`'s `en-1` registration lists the three state-graph forms. | Test (TC-2412) |

## Dependencies

- FSpec FR-712 owns the timed state graph and the meaning of each form over
  it; FSpec FR-390 and FR-393 own the forms and their evidence.
- FR-632 and FR-237 to FR-245 own the zone engine, concretization and its
  certificate.
- FR-628 owns state-graph items over an untimed subject.
