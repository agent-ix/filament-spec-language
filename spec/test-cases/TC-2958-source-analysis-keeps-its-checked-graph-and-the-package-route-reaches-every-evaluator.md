---
id: TC-2958
title: "Source analysis keeps its checked graph and the package route reaches every evaluator"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-2958: Source analysis keeps its checked graph and the package route reaches every evaluator

## Description

Verify that analysis from source decides every item family over its compiled
package, that a selected package reaches the source-checked evaluators through
their package forms with equal records, and that the package route settles every item
family as the source route does. Scope: FR-940-AC-4 to FR-940-AC-6, FR-940-AC-18, FR-940-AC-21 and FR-940-AC-22: a source run whose compiled package the strict reader refuses on read-back settles as the reader's typed refusal, and a refused replay package read settles only the replaying item `inconclusive` `replay_refused`.

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

Tests: `fsl-analyze/tests/it/analyze_package_selection.rs` (steps 1 to 3),
`fsl-analyze/tests/it/hyper_wire.rs` (step 4) and
`fsl-analyze/tests/it/package_refusal_route.rs` (steps 5 and 6).

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
