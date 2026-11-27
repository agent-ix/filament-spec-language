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
subject SHALL have its selected engine decide over the subject's timed state graph, as FSpec
FR-712 defines it: by EN-6 through symbolic zone search, or, when the item
names `options.method` `digital-clocks` and the subject is closed, by EN-1
over the digitized subject (FR-243), as FR-632 routes a timed temporal item,
where EN-1 decides `possible` only (FR-243).
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
subject (FR-243), with basis `exhaustive`; digitization keeps
the clocks, and for a `possible` claim over a closed subject it gives the
zone verdict, so the route is a cross-check of the zone result. `method` is a provider option EN-1
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
- **Zone deciding.** While EN-6 decides a claim instance using zones, it SHALL
  follow FSpec FR-712: `possible` by forward zone reachability of `P`;
  `always possible` by the backward fixpoint of `EF P` over zones, compared
  with the forward reachable zones where `Q` holds; and `unique path` by
  counting realizable discrete-step sequences on the zone graph.
- **Digital deciding.** When EN-1 decides a claim instance through
  `digital-clocks`, it SHALL follow FSpec FR-712 over the digitized timed
  state graph, with basis `exhaustive` and integer delays that replay.
- **Zone undefined evaluation.** While EN-6 decides a timed state-graph item using zones, it SHALL apply FSpec FR-391-AC-10, FR-391-AC-11 and FR-391-AC-14 to each explored state.
- **Zone refuting-evidence order.** When zone exploration finds a trap, path pair, or undefined evaluation, EN-6 SHALL select evidence from the earliest node in FR-391's canonical breadth-first node order. For `unique path`, a trap is taken before a path pair, at the earliest node that has a trap, and a path pair is taken only where no node has a trap; the comparison with an undefined evaluation uses the node of the evidence so chosen.
- **Zone same-node precedence.** When zone exploration finds both undefined evaluation and trap or path-pair evidence at that same earliest node, EN-6 SHALL select the undefined-evaluation evidence.
- **Digital undefined evaluation.** When EN-1 decides a `possible` claim through `digital-clocks`, it SHALL apply FR-168's undefined-evaluation rules to each explored state of the digitized timed graph. EN-1 declines `always possible` and `unique path` for this route, as FR-243 states.
- **Digital refuting-evidence order.** When digital-clock exploration of a `possible` claim finds a trap or undefined evaluation, EN-1 SHALL select evidence from the earliest node in FR-168's canonical breadth-first node order.
- **Digital same-node precedence.** When digital-clock exploration of a `possible` claim finds both undefined evaluation and trap evidence at that same earliest node, EN-1 SHALL select the undefined-evaluation evidence.
- **Undefined settlement.** When a claim predicate, subject guard, `terminal when` predicate, or selected state constraint evaluates undefined at a reachable state, the selected engine SHALL settle every affected item as `refuted`, with basis `decisive-counterexample` and cause `undefined-evaluation`, unless an earlier trap or path pair decides the item under the stated refuting-evidence order.
- **Undefined evidence.** When the selected engine settles an item for undefined evaluation, it SHALL carry FR-393 undefined-evaluation evidence whose canonical path reaches the deciding state and whose `where` names the state key, expression locus, the FR-181 transition identity for a guard, and the catalogued undefined-value reason.
- **Undefined expression readings.** EN-1 and EN-6 SHALL NOT treat an undefined guard as a disabled transition or an undefined `terminal when` predicate as a non-terminal node. These rules apply to every form decided by the selected engine; EN-1's `digital-clocks` route decides `possible` only.
- **Deciding.** On zones, EN-6 SHALL decide each claim instance by FSpec FR-712:
  `possible` by forward zone reachability of `P` from each initial state;
  `always possible` by the backward fixpoint of `E(P)` over unions of zones,
  which every reached valuation where `Q` holds must be in; `unique path` by
  requiring every reached valuation where `X` holds to be in `E(Y)` and no two
  transition sequences, read backwards from `Y` as unions of zones, to lead
  from one of them. A zone-graph node holds many timed states and an edge says
  only that some of them can take it, so EN-6 SHALL NOT decide a form on whole
  nodes or on node edges. It explores the graph under the aM bounds (`L = U`). On `digital-clocks`, EN-1 SHALL decide a
  `possible` instance by FSpec FR-712 over the digitized timed state graph,
  with basis `exhaustive` and integer delays that replay, and SHALL decline
  `always possible` and `unique path` (FR-243).
- **Witness and counterexample.** A `possible` proof carries, for each
  initial state, a witness path; an `always possible` refutation carries a
  path to a reachable state where `Q` holds and from which no `P` state is
  reachable; a `unique path` refutation carries the two distinct step
  sequences or the cycle, or, when a reachable state where `X` holds has no
  sequence to `Y`, a path to that state as FSpec FR-393 trap evidence. The selected engine SHALL concretize each path with exact rational
  delays (FR-241), written in FSpec FR-393's timed evidence form, and replayed
  through FSpec FR-393's model-graph arm before it reaches the document; a
  path replay does not reproduce settles `inconclusive` `replay-parity`
  (FR-281). FSpec FR-393's trap path has no delay after its last step, so it
  SHALL end at a state that is itself a counterexample, one its last step, or
  the initial state, lands in. EN-6 SHALL try the violating nodes in the order
  the zone search discovered them, and each convex piece of a node's violating
  states, with the search's path to the node; the first that ends in a violating
  state is the trap. When none does, `unique path` SHALL still search for two
  sequences. With neither, the instance has no counterexample form: the item
  SHALL settle `inconclusive` `lasso-not-concretized` (FSpec FR-712-AC-12),
  never `refuted` with another path, unless another instance is `refuted`.
- **Certificate.** A proof or refutation SHALL carry EN-6's zone certificate
  (FR-245) over the explored zone graph and, for `always possible`, FSpec
  FR-712's `state_graph` part whose `ef_nodes` list the nodes whose whole zone
  lies in `E(P)`; the
  item's `certificate` member carries that certificate as its `document`, and
  certification is `certified` once FR-245's core checker
  accepts the certificate, and `uncertified` otherwise. The proof basis is
  `zone-certified` (FR-632). The certificate certifies the reachable timed
  graph and carries no time-divergent witness (FR-244 "State-graph form"), so
  a subject whose every infinite path is Zeno still yields a certified
  state-graph verdict.
- **Budgets.** Reaching a `temporal.limits` limit settles the item
  `incomplete` with `LimitReached{limit}` naming it; `exploration` reports
  the zone states and transitions explored. The exploration and the decision
  draw on one budget: each zone the decision holds counts against
  `max_states` with the graph's states, and each predecessor or successor it
  computes, each piece a zone subtraction makes and each pair of zones it
  reads or intersects, a subtraction's zone against a piece included, counts
  against `max_transitions` with the graph's edges. No
  subtraction recurses once per zone of a union. The certificate checker
  charges the same work to its `max_certificate_edges`.
- **Derived items.** A request whose items over the timed subject include a
  state-graph item also holds the subject's derived deadlock-freedom and
  time-lock-freedom items, as FR-626 states for a timed temporal item.

## Acceptance Criteria

The timed fixture is FR-632-AC-3's `Rpc` subject, provisioned with one call, at timeout 4: `send` sets
`phase` 1 and resets `x`, `reply` needs `x >= 1 ms`, `timeout` needs
`x >= 4 ms` with `phase` 1, `reset` returns to `phase` 0, and the time
invariants `x <= 3 ms` while in flight and `x <= 4 ms` while `phase` is 1.

The undefined-evaluation cases below use this subject with a selected integer-division law. `U-P` sets the target predicate to `c.phase = 2 and 1 div (2 - c.phase) > 0`; `U-G` sets the `Reset` guard to `self.phase = 2 and 1 div (2 - self.phase) > 0`; `U-T` disables `Reset` and sets its `terminal when` predicate to `1 div (2 - self.phase) > 0`; `U-C` sets the selected state constraint to `1 div (2 - c.phase) >= 0`. Each expression first evaluates undefined with cause `division-by-zero` at the reachable `phase = 2` state. For each variant and form, the literal item is `possible U using v over (c: M::Call) { P }`, `always possible U using v over (c: M::Call) { Q } { P }`, or `unique path U using v over (c: M::Call) from { Q } to { P }`; `Q` is `c.phase = 0`, and `P` is the variant's target predicate (`c.phase = 2` for U-G, U-T and U-C). An earlier trap or path pair in canonical breadth-first order remains the selected refutation. The undefined-evaluation result in an AC below applies when the undefined state is the earliest refuting node; when a trap or path pair is earlier, that evidence remains selected, and when evidence coincides at one node the undefined-evaluation evidence wins under FR-168 and FSpec FR-391.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-852-AC-1 | `possible Replied using v over (c: M::Call) { c.phase = 2 }` settles `proved` on `en-6` with a witness path `send` then `reply` whose delay before `reply` is an exact rational in `[1, 3]`, the path replays, and certification is `certified`; `possible Late using v over (c: M::Call) { c.phase = 3 }`, unreachable because the server deadline of 3 ms forces `reply` before the 4 ms timeout, settles `refuted`. | Test (TC-2412) |
| FR-852-AC-2 | `always possible BackToIdle using v over (c: M::Call) { c.phase = 0 }` settles `proved`, its `certificate` accepted with `document.state_graph.ef_nodes` listing the nodes whose whole zone lies in `E(P)`; the same subject with `reset` removed settles `always possible BackToIdle` `refuted` with a counterexample path ending at `phase` 2. | Test (TC-2412) |
| FR-852-AC-3 | `unique path OneReply using v over (c: M::Call) from (c.phase = 0) to (c.phase = 2);` settles `refuted` with a cycle through `reset`, and over the subject with `reset` removed settles `proved`. | Test (TC-2412) |
| FR-852-AC-4 | AC-1's `Replied` item with `backend` `en-1` and no `options.method` settles `unsupported` with `unsupported_projection`/`unsupported-requested-capability`, exit 21; with `temporal.limits.max_states` 1 it settles `incomplete` `LimitReached{max_states}`, exit 22. | Test (TC-2412) |
| FR-852-AC-5 | `Engines::fsl()`'s `en-6` registration lists `possible`, `always-possible` and `unique-path` in `state_graph_forms`. | Test (TC-2412) |
| FR-852-AC-6 | AC-1's `Replied` and `Late` items with `options.method` `digital-clocks` settle the same verdicts on backend `en-1`, the proof with basis `exhaustive` and witness delays that are integers and replay; `Engines::fsl()`'s `en-1` registration lists `possible`, `always-possible` and `unique-path` in `state_graph_forms`, while EN-1 declines the latter two forms as FR-243-AC-6 specifies. | Test (TC-2412) |
| FR-852-AC-7 | Over the Zeno-only variant of AC-1's subject (time invariant `x <= 0 ms` at every state, every infinite path zero-delay), `possible Replied` on `en-6` settles `proved` with a witness and a zone `certificate` that carries no `admitted` member and is accepted (FR-244-AC-7, FR-245-AC-11), and `possible Late` (the item whose predicate is `c.phase = 3`, the `TimedOut` state) settles `refuted`; neither is refused by backend candidate selection as `invalid_capability`/`inconsistent-candidates` or for a missing divergence witness, and a certificate given an `admitted` member settles `inconclusive`, `certificate-rejected`, `malformed`. | Test (TC-3030) |
| FR-852-AC-8 | In variant `U-P`, EN-6's `possible` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 evidence naming the `phase = 2` state and `division-by-zero`. | Test |
| FR-852-AC-9 | In variant `U-G`, EN-6's `possible` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state, guard locus, and `Reset` transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-10 | In variant `U-T`, EN-6's `possible` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state and terminal-predicate locus, no transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-11 | In variant `U-C`, EN-6's `possible` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state and constraint locus, no transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-12 | In variant `U-P`, EN-1's `possible` route with `options.method` `digital-clocks` settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 evidence naming the `phase = 2` state and `division-by-zero`. | Test |
| FR-852-AC-13 | In variant `U-G`, EN-1's `possible` route with `options.method` `digital-clocks` settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state, guard locus, and `Reset` transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-14 | In variant `U-T`, EN-1's `possible` route with `options.method` `digital-clocks` settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state and terminal-predicate locus, no transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-15 | In variant `U-C`, EN-1's `possible` route with `options.method` `digital-clocks` settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state and constraint locus, no transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-16 | In variant `U-P`, EN-6's `always possible` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 evidence naming the `phase = 2` state and `division-by-zero`. | Test |
| FR-852-AC-17 | In variant `U-G`, EN-6's `always possible` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state, guard locus, and `Reset` transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-18 | In variant `U-T`, EN-6's `always possible` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state and terminal-predicate locus, no transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-19 | In variant `U-C`, EN-6's `always possible` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state and constraint locus, no transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-20 | In variant `U-P`, EN-1 declines an `always possible` item with `options.method` `digital-clocks`, settling `invalid-input` with diagnostic `not_digitisable` naming `always possible`, `backend` `en-1`, and no search run, as FR-243-AC-6 specifies. | Test |
| FR-852-AC-21 | In variant `U-G`, EN-1 declines an `always possible` item with `options.method` `digital-clocks`, settling `invalid-input` with diagnostic `not_digitisable` naming `always possible`, `backend` `en-1`, and no search run, as FR-243-AC-6 specifies. | Test |
| FR-852-AC-22 | In variant `U-T`, EN-1 declines an `always possible` item with `options.method` `digital-clocks`, settling `invalid-input` with diagnostic `not_digitisable` naming `always possible`, `backend` `en-1`, and no search run, as FR-243-AC-6 specifies. | Test |
| FR-852-AC-23 | In variant `U-C`, EN-1 declines an `always possible` item with `options.method` `digital-clocks`, settling `invalid-input` with diagnostic `not_digitisable` naming `always possible`, `backend` `en-1`, and no search run, as FR-243-AC-6 specifies. | Test |
| FR-852-AC-24 | In variant `U-P`, EN-6's `unique path` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 evidence naming the `phase = 2` state and `division-by-zero`. | Test |
| FR-852-AC-25 | In variant `U-G`, EN-6's `unique path` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state, guard locus, and `Reset` transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-26 | In variant `U-T`, EN-6's `unique path` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state and terminal-predicate locus, no transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-27 | In variant `U-C`, EN-6's `unique path` route settles `refuted`, `decisive-counterexample`, cause `undefined-evaluation`, with FR-393 `where` naming the `phase = 2` state and constraint locus, no transition identity, and cause `division-by-zero`. | Test |
| FR-852-AC-28 | In variant `U-P`, EN-1 declines a `unique path` item with `options.method` `digital-clocks`, settling `invalid-input` with diagnostic `not_digitisable` naming `unique path`, `backend` `en-1`, and no search run, as FR-243-AC-6 specifies. | Test |
| FR-852-AC-29 | In variant `U-G`, EN-1 declines a `unique path` item with `options.method` `digital-clocks`, settling `invalid-input` with diagnostic `not_digitisable` naming `unique path`, `backend` `en-1`, and no search run, as FR-243-AC-6 specifies. | Test |
| FR-852-AC-30 | In variant `U-T`, EN-1 declines a `unique path` item with `options.method` `digital-clocks`, settling `invalid-input` with diagnostic `not_digitisable` naming `unique path`, `backend` `en-1`, and no search run, as FR-243-AC-6 specifies. | Test |
| FR-852-AC-31 | In variant `U-C`, EN-1 declines a `unique path` item with `options.method` `digital-clocks`, settling `invalid-input` with diagnostic `not_digitisable` naming `unique path`, `backend` `en-1`, and no search run, as FR-243-AC-6 specifies. | Test |
| FR-852-AC-32 | EN-6 and its certificate checker decide each form on timed states, not on whole zone nodes. Over TC-2412 step 8's subject, `always possible` of `c.phase >= 2` and `unique path` from `c.phase = 1` to `c.phase >= 2` each settle `refuted`, replayed, with an unaccepted certificate; both do so with `one`'s guard replaced by `false`, and by `y >= 2 ms and x < 2 ms and x > 1 ms`. With `one`'s guard `y >= 2 ms and x < 2 ms` both settle `proved` with an accepted certificate, and with `x <= 2 ms` `unique path` settles `refuted`. With `one` never enabled the checker rejects an `always possible` certificate that lists all of phases 0, 1 and 2 in `ef_nodes` with `rank-broken`. | Test (TC-2412) |
| FR-852-AC-33 | A `max_states` or `max_transitions` that the explored graph fills exactly lets the exploration finish and stops the decision, and the item settles `incomplete` `LimitReached` naming it. A zone subtraction over a union of more than 10,000 zones completes, or stops at its budget with a typed result, on both the producer and the checker. | Test (TC-2412) |
| FR-852-AC-34 | With `escape` enabled in phase 1 only at the instant phase 1 is entered, `always possible` and `unique path` to phase 2 from phase 1 settle `inconclusive` `lasso-not-concretized` with no counterexample. When a further step takes some of the later times to a state that is trapped on entry, both settle `refuted` with a two-step path to that state, replayed. With a second way from phase 0 to phase 2, `unique path` from phase 0 or 1 settles `refuted` by the two sequences. Over a subject with a path pair at node 0, an undefined guard at node 2 and a trap at node 3, the analyzer's `unique path` decision is the undefined outcome at node 2; without the undefined guard it is the trap at node 3, never the pair. A path pair is taken at the earliest node that has one, with that node's shortest sequences, though a later node has a pair of shorter sequences. | Test (TC-2412) |

## Dependencies

- FSpec FR-712 owns the timed state graph and the meaning of each form over
  it; FSpec FR-390 and FR-393 own the forms and their evidence.
- FR-632 and FR-237 to FR-245 own the zone engine, concretization and its
  certificate.
- FR-628 owns state-graph items over an untimed subject.
