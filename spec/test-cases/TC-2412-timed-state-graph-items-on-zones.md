---
id: TC-2412
title: "State-graph items over a timed subject are decided on the zone engine"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-852
    type: verifies
  - target: ix://agent-ix/filament-specification/FR-712
    type: verifies
---
# TC-2412: State-graph items over a timed subject are decided on the zone engine

## Description

Verify that `possible`, `always possible` and `unique path` items over the
timed `Rpc` subject route to EN-6, or to EN-1 under `digital-clocks`, are decided over the timed state graph
with exact-rational, replayed evidence and a zone certificate, and that EN-1
is no candidate without `digital-clocks`. Verify that EN-1 decides `possible`
and declines `always possible` and `unique path`. Scope: FR-852-AC-1 to
FR-852-AC-6, FR-243-AC-6; FSpec FR-712-AC-1 to FR-712-AC-4 through FSL's
engine.

## Test Procedure

1. Analyze `possible Replied` and `possible Late` over `Rpc` at timeout 4.
2. Analyze `always possible BackToIdle` over `Rpc`, then over `Rpc` without `reset`.
3. Analyze `unique path OneReply` over both subjects.
4. Analyze `Replied` with `backend` `en-1`, and with `temporal.limits.max_states` 1.
5. Read `Engines::fsl()`'s `en-6` and `en-1` registrations.
6. Repeat step 1 with `options.method` `digital-clocks`.
7. Analyze `always possible BackToIdle` and `unique path OneReply` with `options.method` `digital-clocks`.
8. Over a dense subject with clocks `x` and `y`: `first` runs by `y = 1 ms` and resets `x`; from `phase` 1, bounded by `y <= 2 ms`, `zero` needs `y >= 2 ms` and `x >= 2 ms`, and `one` needs `y >= 2 ms` and `x <= 1 ms`. Analyze `always possible` of `c.phase >= 2`, and `unique path` from `c.phase = 1` to `c.phase >= 2`, with `digital-clocks`.

## Expected Results

- Step 1: `Replied` `proved` on `en-6`, witness `send` then `reply` with an exact rational delay in `[1, 3]`, replayed, certified; `Late` `refuted`.
- Step 2: `proved`, the accepted certificate's `document.state_graph.ef_nodes` non-empty; then `refuted` with a path ending at `phase` 2.
- Step 3: `refuted` with a cycle through `reset`; `proved` without `reset`.
- Step 4: `unsupported` `unsupported-requested-capability`, exit 21; `incomplete` `LimitReached{max_states}`, exit 22.
- Step 5: each `state_graph_forms` lists `possible`, `always-possible` and `unique-path`.
- Step 6: the same verdicts on `en-1`, basis `exhaustive`, integer delays that replay.
- Step 7: each `declined` `invalid-input`, `invalid_runtime_input`/`invalid-value`, `backend` `en-1`, and no search runs; `not_digitisable` has `rule` `always-possible` for the first item and `unique-path` for the second, and `span` that of the claim's declaration. This step asserts the decline only and compares nothing with EN-6. The route that gives a verdict for these two forms is EN-6 (steps 2 and 3), whose node-level reading of a partly covered zone is wrong until QSL-3478 is fixed.
- Step 8: each declined the same way, with the cause and `not_digitisable` of step 7, never `proved`. By hand both claims are false: with `first` strictly between 0 and 1, `x` at `y = 2 ms` is strictly between 1 and 2, so neither `zero` nor `one` is ever taken. The step relies on no EN-6 verdict: EN-6's node-level reading of this subject is wrong (QSL-3478), and the verdict is to be asserted on EN-6 by the test that fixes it.

Test: `fsl-analyze/tests/it/timed_state_graph.rs`.
