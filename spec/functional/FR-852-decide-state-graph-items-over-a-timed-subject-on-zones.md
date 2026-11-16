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
subject SHALL be decided by EN-6 over the subject's timed state graph, as
FSpec FR-712 defines it, through symbolic zone search. EN-6 SHALL advertise
the three state-graph forms in its FSpec FR-331 `state_graph_forms`, so
negotiation routes such an item to it, and the item settles `proved` or
`refuted` with FSpec FR-393 evidence whose delays are exact rationals.

### Decision: EN-6 decides timed state-graph claims; EN-1 never sees them

EN-1 explores an untimed transition system. Running a timed subject's
state-graph claim on it would drop the clocks, the guards that read them and
the time invariants, and so would decide a different system: a node that is
reachable only by waiting past a deadline would count as reachable. A
state-graph claim over a timed subject is therefore decided on EN-6's zone
graph, which keeps the clock constraints. The same items over an untimed
subject keep going to EN-1 (FR-628). The digital-clock route (FR-243) stays
a caller-selected cross-check through `options.method` `digital-clocks` on
a closed subject, as FR-632 states for temporal items.

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
  modes it advertises. Over a timed subject, FR-489's candidate set for a
  state-graph item is EN-6 alone; EN-1 is not a candidate, because it
  matches no item over a timed subject (FSpec FR-712 "Routing"). An item naming `backend` `en-1` over a timed subject therefore
  settles by FSpec FR-290's empty-candidate row, `unsupported`.
- **Deciding.** EN-6 SHALL decide each claim instance by FSpec FR-712:
  `possible` by forward zone reachability of `P`; `always possible` by the
  backward fixpoint of `EF P` over zones, compared with the forward reachable
  zones where `Q` holds; `unique path` by counting realizable discrete-step
  sequences on the zone graph.
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
  (FR-245) over the explored zone graph and, for `always possible`, the
  `EF P` zone sets; certification is `certified` once FR-245's core checker
  accepts the certificate, and `uncertified` otherwise. The proof basis is
  `zone-certified` (FR-632).
- **Budgets.** Reaching a `temporal.limits` limit settles the item
  `incomplete` with `LimitReached{limit}` naming it; `exploration` reports
  the zone states and transitions explored.
- **Derived items.** A request whose items over the timed subject include a
  state-graph item also holds the subject's derived deadlock-freedom and
  time-lock-freedom items, as FR-626 states for a timed temporal item.

## Acceptance Criteria

The timed fixture is FR-632-AC-1's `Rpc` subject at timeout 4: `send` sets
`phase` 1 and resets `x`, `reply` needs `x >= 1 ms`, `timeout` needs
`x >= 4 ms` with `phase` 1, `reset` returns to `phase` 0, and the time
invariants `x <= 3 ms` while in flight and `x <= 4 ms` while `phase` is 1.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-852-AC-1 | `possible Replied using v over (c: M::Call) { c.phase = 2 }` settles `proved` on `en-6` with a witness path `send` then `reply` whose delay before `reply` is an exact rational in `[1, 3]`, the path replays, and certification is `certified`; `possible Late using v over (c: M::Call) { c.phase = 3 }`, unreachable because the server deadline of 3 ms forces `reply` before the 4 ms timeout, settles `refuted`. | Test (TC-2412) |
| FR-852-AC-2 | `always possible BackToIdle using v over (c: M::Call) { c.phase = 0 }` settles `proved`; the same subject with `reset` removed settles `always possible BackToIdle` `refuted` with a counterexample path ending at `phase` 2 and the `EF P` zone sets in its certificate. | Test (TC-2412) |
| FR-852-AC-3 | `unique path OneReply using v over (c: M::Call) from (c.phase = 0) to (c.phase = 2);` settles `refuted` with a cycle through `reset`, and over the subject with `reset` removed settles `proved`. | Test (TC-2412) |
| FR-852-AC-4 | AC-1's `Replied` item with `backend` `en-1` settles `unsupported` with `unsupported_projection`/`unsupported-requested-capability`, exit 21; with `temporal.limits.max_states` 1 it settles `incomplete` `LimitReached{max_states}`, exit 22. | Test (TC-2412) |
| FR-852-AC-5 | `Engines::fsl()`'s `en-6` registration lists `possible`, `always-possible` and `unique-path` in `state_graph_forms`. | Test (TC-2412) |

## Dependencies

- FSpec FR-712 owns the timed state graph and the meaning of each form over
  it; FSpec FR-390 and FR-393 own the forms and their evidence.
- FR-632 and FR-237 to FR-245 own the zone engine, concretization and its
  certificate.
- FR-628 owns state-graph items over an untimed subject.
