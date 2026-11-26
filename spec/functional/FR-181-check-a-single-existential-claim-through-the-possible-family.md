---
id: FR-181
title: "Check a single-existential hyper claim through the possible family"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-021
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-173
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
---
# FR-181: Check a single-existential hyper claim through the possible family

## Description

FSL's layer-A `fsl-analyze` engine EN-1 (ADR-029 CB-3) SHALL check an HP-5 claim, a `hyper` clause over
`behaviours` whose prefix is exactly one `exists trace b of M`, as a
possibility claim with a temporal body (ADR-023 SE-1 to SE-5): from every
initial state of the subject, some behaviour fair under the clause's
fairness set satisfies the body. It is settled by the possible family's
verdict map and evidence shapes (ADR-022 GV-1, GV-2, GV-5, GV-7, GX-1, RU-5):
a witness lasso per initial state proves it once exploration of the product
has completed with no open node and no undefined evaluation of the body, a
closed part of the product with no fair accepting cycle refutes it, an
undefined evaluation of the body at a reachable position refutes it, and
anything else stays undecided. The
possible family's requirements (FR-166, FR-169 and FR-170) apply as stated
here.

## Use case

A verification operator asks whether a vault can settle into a public
output of 1 for good, from every start. The engine finds, per initial
state, a lasso of `step(1)` steps on which the claim holds, and the result
is `proved` with those lassos. Asked whether the output can reach 2, which
its type never allows, the engine examines each start's whole product and
returns `refuted` with basis `closed-scope`, since no single trace can show
that no trace exists.

## Inputs

- A `ModelCheckRequest` (FR-126) whose item is an HP-5 clause (FR-173), with
  its subject, the clause's fairness set and `ModelCheckLimits`, including
  `witness_samples` and the seed (FR-166).

## Outputs

- FR-168's `StateGraphOutcome` shapes: `Witnessed` with one lasso per
  initial state and its source, `WitnessedUnchecked` with the same lassos,
  `Trapped` naming an initial state with an empty stem, `Undefined`,
  `NoDecision`, or `NoInitialState`; settled by FR-169's map.
- `GraphEvidence::Witness` whose paths are FR-170's `WitnessPath::Lasso`
  (`ModelLasso`), each a lasso in ADR-018 CX-2's step and loop shape; a
  state-graph witness is a `WitnessPath::Path`.
- `GraphEvidence::Trap` with FR-170's `TrapClosure::Product`, naming the
  initial state whose part of the body-automaton product is closed and
  holds no fair accepting cycle; a state-graph trap is
  `TrapClosure::StateGraph`. FR-170 routes a `Product` trap's replay here.
  Each is replayed as stated below.

## Behavior

### Order

- The engine SHALL classify every root of the subject as FR-176's
  pre-check does, and SHALL run no separate exploration of the subject for
  an HP-5 item. Phase 0 SHALL run next, then the product exploration below,
  whose `max_states` counts product states (ADR-023 HC-8).

### Check

- The engine SHALL build the product of the subject with the generalized
  Büchi automaton for the body itself, not its negation, and explore it by
  FR-126's first phase, retaining every edge.
- For each initial state, the engine SHALL search that initial state's part
  of the product for a fair accepting cycle by FR-126's SCC phase, with the
  clause's fairness set in FR-126's fairness filter.
- A product node the engine did not fully expand SHALL be open, with the
  causes FR-167 names.
- A cycle through an open node SHALL NOT count as a fair accepting cycle, and
  a phase 0 loop through a state whose contract evaluation had no value SHALL
  NOT count as a witness: what the state enables is unknown, so fairness
  cannot be read there.

### Phase 0

- Before exploring, the engine SHALL draw FR-166's walks per initial state,
  with FR-166's trace indices and seed. A walk that revisits a state SHALL
  give a candidate lasso. A candidate SHALL count as a witness only when its
  loop is fair under the clause's fairness set and the body evaluates `true`
  on it by FR-125.
- Phase 0 SHALL NOT settle the item: the product exploration SHALL run
  whatever phase 0 found.

### Outcomes

- An initial state with a fair accepting cycle in its part, or a sampled
  witness, SHALL have a witness lasso: the canonical lasso of FR-126 for an
  explored one, source `Explored`; the candidate, source `Sampled`, for a
  sampled one.
- When every initial state has a witness, the engine SHALL return
  `Witnessed` if the product exploration completed with no open node, and
  `WitnessedUnchecked` with the run's end and open causes otherwise, since a
  reachable undefined evaluation of the body is then not ruled out
  (ADR-022 RU-5).
- `Witnessed` SHALL carry the product-closure certificate of the explored
  product (FR-163); the item SHALL settle `proved` only after its witnesses
  replay and the certificate check accepts (FR-169's `settle_existential`,
  ADR-023 HX-7).
- If a letter of the body evaluates `Undefined` at a reachable product
  state, the engine SHALL return `Undefined` with FR-126's canonical
  breadth-first path to the first such product state, `where` naming that
  position and `cause` the evaluator's undefined cause (ADR-023 HV-8,
  ADR-018 UE-4). `Undefined` SHALL take the place of `Witnessed` and
  `WitnessedUnchecked`. Between `Undefined` and a trap, the one whose first
  product state is first in canonical breadth-first order SHALL be returned, a
  trap ranking at the first product state of its initial state's part; at the
  same product state `Undefined` wins, since a position's letter is read before
  the automaton steps (ADR-018 UE-4).
- `Trapped` SHALL name the first initial state, in subject order, whose part
  of the product is closed (no open node) and holds no fair accepting
  cycle.
- A product state from which the body automaton has no run lies on no
  accepting cycle, and no state after it does, so one a run limit or
  `max_depth` left unexpanded SHALL NOT count against a part being closed for
  `Trapped` when every contract conjunction of its model state has a value.
  It stays open for `Witnessed`. An undefined contract conjunction is never
  read as a disabled step (ADR-018 UE-6). Those conjunctions SHALL be read
  when the product state is created, so that deciding a part closed expands
  no state the run limits did not admit.
- A clause with object parameters (`over`) SHALL be checked once per instance
  in FR-125's canonical instance order, the instances sharing the run's
  `max_states` and `max_transitions` as FR-126 shares them across bindings, and
  the result SHALL be one outcome per instance; a clause with no instance has
  no outcome of this check, and its item settles `NoInstance`, or
  `NoInitialState` for a subject with no initial state, as FR-125 Instancing
  states: `inconclusive`, never a fault.
- The item's one record over its instances' outcomes SHALL follow FR-621
  "Items over several claim instances": a refuted instance decides the item,
  an earlier instance that is not `proved` notwithstanding.
- `settle_existential` SHALL refuse
  `ProductClosureRefusal::Input("HP-5-instance")`, and write no record, an
  outcome whose instance does not bind each object parameter, in parameter
  order, to an instance of it (FR-125 Instancing).
- A package selection of an HP-5 clause with an object parameter SHALL settle
  the item `unsupported_construct`/`not-yet-implemented`, exit 21, with no
  verdict and no instance.
- `check_single_existential` and `settle_existential` SHALL decide and settle
  each instance of a clause with two or more object parameters. FSpec
  FR-393's evidence instance names one parameter, so the source
  `native-analyze/1` item of such a clause SHALL settle
  `unsupported_construct`/`not-yet-implemented`, exit 21, with no verdict and
  no instance.
- Every other case SHALL return `NoDecision` with the run's end and open
  causes.

### Replay

- A witness lasso SHALL replay by FR-128's step, loop and stutter rules and
  refusals, with fairness checked on its loop and the body evaluated by
  FR-125: `true` SHALL settle `reproduced-with-evaluated-witness`; `false`
  SHALL settle `inconclusive`, `Verdicts`.
- A trap SHALL replay by re-exploring that initial state's part of the
  product, unreduced, under the request's limits, and running the SCC phase
  with the fairness filter: no fair accepting cycle SHALL settle
  `reproduced-with-evaluated-witness`; one found SHALL settle
  `inconclusive`, `Verdicts`; a stopped exploration SHALL return the replay
  result stopped with its limit. The re-exploration SHALL NOT expand a state
  from which the body automaton has no run, nor one whose letter has no
  value, so neither can stop it. A no-run state whose contract conjunction,
  read when the state is created, has no value or is refused SHALL NOT
  reproduce the trap.
- An `Undefined` path SHALL replay as FR-170 replays undefined-evaluation
  evidence, re-executing the path and evaluating the body's letter at its
  last position.
- Each result SHALL state how it was settled, as FR-169 requires: the
  refutation's basis is `closed-scope` and its method exhaustive
  exploration of the trap's closure.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-181-AC-1 | Over ADR-023 §8's secure vault, `exists trace b of V { eventually always holds(v.l @ b = 1) }` with `witness_samples` 0 returns `Witnessed` with one lasso per initial state, each ending in a loop of `step(1)` steps, source `Explored`; each replays to `reproduced-with-evaluated-witness`, and the item settles `proved`, `decisive-witness`. With the default `witness_samples` it settles `proved` too, and its record names each witness's source. | Test (TC-606) |
| FR-181-AC-2 | `exists trace b of V { eventually holds(v.l @ b = 2) }` returns `Trapped` for initial state 0 with an empty stem; replay re-explores its part of the product, finds no accepting cycle, and the item settles `refuted`, `closed-scope`. | Test (TC-606) |
| FR-181-AC-3 | Over the secure vault with FR-176-AC-3's `reset` operation, `exists trace b of V fair weak V::Vault::reset; { eventually always holds(v.l @ b = 1) }` settles `refuted`, `closed-scope`, and the same clause with no fairness set settles `proved`. | Test (TC-606) |
| FR-181-AC-4 | AC-1's claim with `max_depth` 1 and `witness_samples` 0 returns `NoDecision` and settles `inconclusive`, `BoundReached{depth: 1}`. AC-1's witness with its loop's last step removed refuses as FR-128 refuses a loop that does not close. | Test (TC-606) |
| FR-181-AC-5 | Over the secure vault, with `function remainder using v(x: Integer): Integer pure { 1 mod x }`, its body checked under `CheckMode::Kernel` and called by the Linked claim (FR-495-AC-3, FR-952), `exists trace b of V { eventually always holds(remainder(1 - v.l @ b) = 0 and v.l @ b = 0) }`, whose division is evaluated first and has no value at `l = 1`, returns `Undefined` at the first product state with `l = 1`, with the default `witness_samples` as with `witness_samples` 0, while phase 0 finds a sampled witness lasso looping `step(0)` at `(h, 0)` for each initial state; cause `division-by-zero`. It settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`. AC-1's claim with the default `witness_samples` settles `proved` only from a product exploration that completed with no open node. | Test (TC-615) |
| FR-181-AC-6 | AC-1's claim with the default `witness_samples` and `max_states` 2 gets a sampled witness lasso per initial state, the exploration stops at `max_states`, and the item returns `WitnessedUnchecked` and settles `inconclusive`, `WellDefinednessUnchecked`, never `proved`; with `witness_samples` 0 the same run settles `incomplete`, `LimitReached`, naming `max_states`. | Test (TC-616) |
| FR-181-AC-7 | Two vaults in one start, so two instances of `over (v: V::Vault)`, give one outcome per instance in canonical order, and the second instance's product is explored with the `max_states` the first left; a clause with no instance has no outcome. | Test (TC-606) |
| FR-181-AC-8 | `pets` declares `Thing`, which has the subtype `Lamp`; its universe is `[l, m]`; the model has no operation; the one initial state holds a `Lamp` under `l`, and none admits `m`. With `v: M::Thing`, `exists trace a of M { always holds(BODY @ a) }` under the infinite-trace profile, default limits, for `BODY` `present(lookup<M::Thing>(M::pets, v) absent empty)` and for `BODY` `(v = v)`: `check_single_existential` returns exactly one instance, `l`, `Witnessed`; `settle_existential` settles it `proved` and refuses an outcome at the instance `m` as `ProductClosureRefusal::Input("HP-5-instance")`; `excluded_keys` is `[m]`; and the source `native-analyze/1` item is `proved`, exit 0, its evidence binding `v` to the reference with identity `l` and object type `Lamp`, with no instance binding naming `m`. With universe `[k, l]`, the initial state holding only `l`, and `BODY` `(v != v)`, the item is `refuted`, exit 10, at the instance `l`, never `k`. | Test (TC-3219) |
| FR-181-AC-9 | AC-8's two bodies over AC-8's first fixture, under `filament.temporal.event-position.false-extension/v1` with `always [0, 0] holds(BODY @ a)`, give the one instance `l` and settle `proved`, with no instance binding naming `m`. | Test (TC-3219) |
| FR-181-AC-10 | AC-8's request selected by package settles `unsupported_construct`/`not-yet-implemented`, exit 21, with no record verdict and no instance. | Test (TC-3219) |
| FR-181-AC-11 | A refutation after an inconclusive instance. `Counter`'s `value` is `Int[0, 5]`, `inc` raises its receiver's value by 1 and is enabled below 3, the universe is `[p, q]`, and the one initial state holds `p` at 0 and `q` at 4. For `v: M::Counter`, `exists trace a of M { always holds((v.value <= 3) @ a) }` with `max_depth` 1 and `witness_samples` 0 gives `p` `NoDecision`, bound reached at depth 1, and `q` `Trapped`, and the item settles `refuted`, exit 10, at the instance `q`. Over the universe `[p]` with the same initial `p`, the item settles `inconclusive`. Over the universe `[r]` with one initial state that holds no `Counter`, the item settles `inconclusive`, `no-instance`, and is not a fault. | Test (TC-3219) |
| FR-181-AC-12 | Two parameters. Over two secure vaults, `v` at `l = 0` and `w` at `l = 1`, `over behaviours (x: V::Vault, y: V::Vault) exists trace b of V { always holds(x.l @ b = y.l @ b) }`, and its body under `always [0, 0]` and the event-position profile: the instances `(v, v)`, `(v, w)`, `(w, v)` and `(w, w)` settle `proved`, `refuted`, `refuted` and `proved`, and `settle_existential` refuses `HP-5-instance` an outcome that binds one parameter or both out of order. Over `pets` with two admitted keys and `(v: M::Thing, w: M::Thing)`, the source `native-analyze/1` item is `unsupported_construct`/`not-yet-implemented`, exit 21, with no verdict and no instance. | Test (TC-3219) |
| FR-181-AC-13 | Unexpanded states of a trap. Over the secure vault with `witness_samples` 0, each of these returns `Trapped` and settles `refuted`, `closed-scope`: `holds(v.l @ b = 1)` from `l = 0` with `max_states` 1, and with `max_transitions` 1; and, from `l = 0` with AC-5's `remainder`, which has no value after a step to `l = 1`, `holds(v.l @ b = 1) and always holds(remainder(1 - v.l @ b) = 0 and v.l @ b = 0)`. From `(h, l) = (0, 1)`, where `step`'s precondition has no value, `holds(v.l @ b = 0)` does not return `Trapped`, with the default limits or `max_states` 1, and `settle_existential` does not confirm a `Trapped` outcome for it (ADR-018 UE-6). | Test (TC-3219) |
| FR-181-AC-14 | An undefined read through the source item. `Counter`'s `inc` is disabled and an operation `drop` is framed `deletes Counter`; the universe and the initial state hold one counter `c` at `value = 0`. `exists trace a of M { always holds((v.value = 0) @ a) }` for `v: M::Counter` as a `native-analyze/1` item settles `refuted`, `decisive-counterexample`, exit 10, with an `undefined` counterexample of cause `absent-key` at position 1. | Test (TC-3219) |

## Dependencies

- ADR-023 §15 SE-1 to SE-5, §12 RU-3, HV-8; ADR-022 §3 GE-2, §4 GV-1, GV-2,
  GV-5, GV-7 and "Settlement method", §10 RU-5, §5 GX-1 and GX-3; ADR-018
  §3 EN-1, CX-2, CX-3.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (product phases, fairness filter, canonical lasso),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-128](FR-128-replay-a-model-counterexample.md) (lasso replay rules),
  [FR-173](FR-173-classify-hyper-clauses-into-forms.md).
- The possible family's requirements: FR-166 (phase 0), FR-167 (open
  nodes), FR-168 (outcome shapes), FR-169 (verdict map and settlement
  method) and FR-170 (`GraphEvidence` replay).
- FSpec owns HP-5's routing to the possible family with its witness, trap
  and verdict rules (ADR-023 QS-3).

## References

- ADR-023, ADR-022. FSpec halves: FSpec FR-396's single-existential rows
  (Linear STD-136; ADR-023 QS-3) and FSpec FR-390 to FR-394, the possible
  family (Linear STD-135).
