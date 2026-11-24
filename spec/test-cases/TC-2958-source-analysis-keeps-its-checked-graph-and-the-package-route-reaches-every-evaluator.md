---
id: TC-2958
title: "Source analysis keeps its checked graph and the package route reaches every evaluator"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-200
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-945
    type: references
---
# TC-2958: Source analysis keeps its checked graph and the package route reaches every evaluator

## Description

Verify that analysis from source decides every item family over its compiled
package, that a selected package reaches the source-checked evaluators through
their package forms with equal records, and that the package route settles every item
family as the source route does. Scope: FR-940-AC-4 to FR-940-AC-6, FR-940-AC-18, FR-940-AC-21, FR-940-AC-22, FR-940-AC-28 to FR-940-AC-32, FR-940-AC-40 to FR-940-AC-44 and FR-200-AC-8 to FR-200-AC-10: fair eventual claims with strong and per-instance constraints retain their source/package outcomes; a source run whose compiled package the strict reader refuses on read-back settles as the reader's typed refusal, and a refused replay package read settles only the replaying item `inconclusive` `replay_refused`.

## Test Procedure

1. Analyze the Counter program over source with `Bounded`, `AtMostTwo` and the
   invariant `NotAboveThree`, and the `SB` litmus claim under `memory` `tso`.
2. Compile the Counter unit with the compile request's `handoff` member,
   select the handoff directory as `package` `{"directory": ...}`, and analyze
   `NotAboveThree` and `AtMostTwo`; compare with step 1.
3. Analyze `Bounded`, `AtMostTwo`, `NotAboveThree`, the deadlock-freedom item
   and the `SB` litmus claim under `tso` over both routes (the package route
   over the handoff directory), a handoff directory whose `checked-package.json`
   the strict reader refuses, and one with no `identities.json`.
4. Overwrite the hyper Counter unit's source with bytes that are not a native
   unit, delete the handoff directory's `sources/`, and analyze `Equal`
   (FR-940-AC-16) through the `package` selection; check its certificate.
5. Analyze the Counter program over source with a binding whose `packages`
   leave out `test/counter`, and again with it supplied.
6. Analyze `Bounded` with `AtMostTwo`, and `ReachThree`, over source and
   over the handoff directory with `replay.input_bytes` below the
   emission's length, and over source again with a test-only emitter that
   fails at the replay's emission.
7. Analyze a unit containing an operation-contract item from source. Compile
   it with a handoff, analyze that exact claim through the `package` route,
   and compare the requirement row, request index, typed operation-contract
   record, selected `ContractClaim`, subject, case limit, item record and
   process exit. Then submit a malformed operation-contract record through
   the strict reader and verify no engine runs.
8. Use FR-230-AC-63's selected model with `Phase { Armed(Clock), Idle }` and
   its dense `ms` time declaration. Analyze the timed guard and reset over the
   source route and over the admitted package route. Compare the `Armed`
   clock-place identity, guard amount `1 ms`, presence predicate, reset place
   and reset amount `0 ms`; verify the package result does not create
   `Value::Clock`.
   Bind the test `#[trace("TC-2958", "FR-940-AC-29")]`.
9. Select a model declaring `tuple Slot(Integer, Integer)` and
   `union Outcome { FromTuple(Slot), Empty }`. Analyze the invariant and
   state-graph claims that read separate tuple positions through `FromTuple`
   from source and from the handoff package. Compare item records, witnesses or
   counterexamples, replay data and charge logs. Remove only `model_tuples` from
   the handoff and verify package admission refuses before any item runs and
   never reads source.
   Bind the test `#[trace("TC-2958", "FR-940-AC-30")]`.
10. Analyze the checked unit with `union Reading { Raw(Integer), Wide(Int[0, 9]) }`,
    invariant `ReadingInvariant` and state-graph claim `ReadingReach` from the
    source route, then from its handoff package. Compare both item records and
    process exits and check both claims prove. Bind the test
    `#[trace("TC-2958", "FR-940-AC-31")]`.
11. In `fsl-analyze/tests/it/analyze_package_value.rs`, declare `union Shape { Empty, Circle(Boolean) }` and a pure `isCircle`
    function with a `Shape` parameter whose case returns the `Circle` Boolean
    payload or `false` for `Empty`. Invoke `isCircle` through the source and
    package run-call routes with `Circle(true)` and with `Empty`. Compare the
    argument values, Boolean results and semantic charge logs. Bind the test
    `#[trace("TC-2958", "FR-940-AC-32")]`.
12. Over the `Link` unit, model and `LINK_START` snapshot, submit the
    every-scheduler eventual claim `TooLikely` with
    `probability >= rational(99, 100) [eventually holds(m.delivered)]` for
    each of `fair weak M::Msg::send_a`, `fair strong M::Msg::send_a`,
    `fair weak each M::Msg::send_a` and `fair strong each M::Msg::send_a`.
    For every form, analyze the same item from the source request and from its
    checked-package handoff. Compare the complete outcome documents and process
    exits. Repeat the weak-whole form with
    `probability <= rational(98, 100) [eventually holds(m.delivered)]`.
    In `every_fairness_preserves_source_package_refutation_parity`, bind the
    weak-whole form to `FR-200-AC-4` and `FR-940-AC-40`, the strong-whole form
    to `FR-200-AC-8` and `FR-940-AC-41`, weak each to `FR-200-AC-9` and
    `FR-940-AC-42`, and strong each to `FR-200-AC-10` and `FR-940-AC-43`.
    Add traces for `TC-2958` and each listed owning AC to that test. In
    `a_fair_every_scheduler_refutation_replays_with_source_package_parity`,
    bind weak whole `<= 98/100` to `FR-200-AC-4` and `FR-940-AC-44`, and add
    traces for `TC-2958` and both owning ACs.

Tests: `fsl-analyze/tests/it/analyze_package_selection.rs` (steps 1 to 3 and 8 to 10),
`fsl-analyze/tests/it/hyper_wire.rs` (step 4) and
`fsl-analyze/tests/it/package_refusal_route.rs` (steps 5 and 6), and the
named `operation_contract_package_route_matches_source` case for step 7.
`fsl-analyze/tests/it/analyze_package_value.rs` covers step 11;
`fsl-analyze/tests/it/package_probabilistic_refutation.rs` covers step 12.

## Expected Results

- Step 1: `proved`, `refuted` (reproduced), `proved` (`exhaustive`); `SB`
  `refuted`; no `unsupported` record and no package-fact diagnostic.
- Step 2: item records equal to step 1's.
- Step 3: equal records, counterexamples and replays, none `unsupported`; the
  refused directories settle no item and compile no source.
- Step 4: `Equal` settles `proved` with the source route's record, and the
  certificate is accepted with no source present.
- Step 5: `missing_import`/`missing-selection` naming `test/counter`, `details.path`
  under `/semantic_graph/nodes/`, `items` `[]`, `last_stage` `S4`, exit 20,
  never `runtime_invariant`; with the package supplied, exit 0.
- Step 6: on both routes `AtMostTwo` and `ReachThree` settle `inconclusive`,
  `replay_refused` carrying `stage_limit_exceeded`/`input-bytes-exceeded`
  naming `replay.input_bytes`, never `package-emission` or `package-bytes`
  and never `proved` for `ReachThree`; `Bounded` `proved`; no whole-run
  refusal; with the failing emitter, `AtMostTwo` settles `failed` with
  `analyze`/`package-emission`.
- Step 7: both valid routes return equal operation-contract item records and
  the same process exit; malformed package data is refused by admission before
  engine execution, never as "requirement record has no package form".
- Step 8: both routes identify the same `Armed` clock place and return the
  same exact guard and reset amounts and presence condition; the package route
  does not create `Value::Clock`.
- Step 9: both routes return equal item records, witnesses or counterexamples,
  replay data and charge logs. Missing `model_tuples` refuses the whole package
  before item execution, with no source fallback.
- Step 10: both routes settle the source-union invariant and state-graph items
  `proved` with equal records and process exits; neither package item is
  `unsupported` for the payload-bearing source union.
- Step 11: both routes return `true` for `Circle(true)` and `false` for
  `Empty`, with equal argument values, results and charge logs. The package
  run-call does not refuse or settle `unsupported` for the union parameter.
- Step 12: all source/package pairs have equal complete outcome documents and
  exit codes. Each `>= 99/100` claim and the weak-whole `<= 98/100` claim is
  `refuted`, exit `10`; the reverse-bound weak-whole package replay reproduces.
