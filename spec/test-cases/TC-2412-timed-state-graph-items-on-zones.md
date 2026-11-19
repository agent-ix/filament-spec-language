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
is no candidate without `digital-clocks`. Scope: FR-852-AC-1 to FR-852-AC-6; FSpec
FR-712-AC-1 to FR-712-AC-4 through FSL's engine.

## Test Procedure

1. Analyze `possible Replied` and `possible Late` over `Rpc` at timeout 4.
2. Analyze `always possible BackToIdle` over `Rpc`, then over `Rpc` without `reset`.
3. Analyze `unique path OneReply` over both subjects.
4. Analyze `Replied` with `backend` `en-1`, and with `temporal.limits.max_states` 1.
5. Read `Engines::fsl()`'s `en-6` and `en-1` registrations.
6. Repeat step 1 with `options.method` `digital-clocks`.

## Expected Results

- Step 1: `Replied` `proved` on `en-6`, witness `send` then `reply` with an exact rational delay in `[1, 3]`, replayed, certified; `Late` `refuted`.
- Step 2: `proved`, the accepted certificate's `document.state_graph.ef_nodes` non-empty; then `refuted` with a path ending at `phase` 2.
- Step 3: `refuted` with a cycle through `reset`; `proved` without `reset`.
- Step 4: `unsupported` `unsupported-requested-capability`, exit 21; `incomplete` `LimitReached{max_states}`, exit 22.
- Step 5: each `state_graph_forms` lists `possible`, `always-possible` and `unique-path`.
- Step 6: the same verdicts on `en-1`, basis `exhaustive`, integer delays that replay.

Test: `fsl-analyze/tests/it/timed_state_graph.rs`.
